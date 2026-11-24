//! Runs a planned install against the server through Wings.
use crate::{
    db::{Install, InstallStatus, StepProgress, StepStatus},
    installer::MAX_FILE_BYTES,
    model::{PowerAction, StepKind, absolute, join_path, search_regex, split_path},
};
use axum::http::StatusCode;
use shared::{State, models::server::Server};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use uuid::Uuid;
use wings_api::client::{ApiHttpError, AsyncRequestReader, WingsClient};

const POWER_TIMEOUT: Duration = Duration::from_secs(120);
const POWER_POLL_FAST: Duration = Duration::from_secs(1);
const POWER_POLL_FAST_WINDOW: Duration = Duration::from_secs(10);
const POWER_POLL: Duration = Duration::from_secs(2);
/// How long Wings may wait for another power action to finish first.
const POWER_LOCK_SECONDS: u64 = 30;
/// Keeps `updated` fresh while a long step runs.
const HEARTBEAT: Duration = Duration::from_secs(60);
const LIST_PER_PAGE: u64 = 100;
const LIST_MAX_PAGES: u64 = 100;

type StepResult = Result<Option<String>, String>;

fn wings_error(err: ApiHttpError) -> String {
    match err {
        ApiHttpError::Http(status, err) => format!("Wings answered {status}: {}", err.error),
        err => format!("Wings request failed: {}", anyhow::Error::from(err)),
    }
}

fn mib(bytes: usize) -> usize {
    bytes / 1024 / 1024
}

fn check_output_size(path: &str, len: usize) -> Result<(), String> {
    if len > MAX_FILE_BYTES {
        return Err(format!(
            "`{}` would exceed {} MiB",
            absolute(path),
            mib(MAX_FILE_BYTES)
        ));
    }
    Ok(())
}

/// Runs the install in a background task.
pub fn spawn(
    state: State,
    server: Server,
    user: Uuid,
    install: Uuid,
    planned: Vec<Option<StepKind>>,
    mut progress: Vec<StepProgress>,
) {
    tokio::spawn(async move {
        if let Err(err) = run(&state, server, user, install, planned, &mut progress).await {
            tracing::error!(%install, "template install aborted unexpectedly: {err:#}");
            let error = format!("unexpected error: {err:#}");
            for step in &mut progress {
                if step.status == StepStatus::Running {
                    step.status = StepStatus::Failed;
                    step.message.get_or_insert_with(|| error.clone());
                }
            }
            if let Err(finish_err) = Install::finish(
                &state.database,
                install,
                InstallStatus::Failed,
                &progress,
                Some(&error),
            )
            .await
            {
                tracing::error!(
                    %install,
                    "failed to mark aborted template install as failed: {finish_err:#}"
                );
            }
        }
    });
}

async fn run(
    state: &State,
    server: Server,
    user: Uuid,
    install: Uuid,
    planned: Vec<Option<StepKind>>,
    progress: &mut [StepProgress],
) -> Result<(), anyhow::Error> {
    let client = match connect(state, &server).await {
        Ok(client) => client,
        Err(err) => {
            let error = format!("could not reach the node: {err:#}");
            Install::finish(
                &state.database,
                install,
                InstallStatus::Failed,
                progress,
                Some(&error),
            )
            .await?;
            return Ok(());
        }
    };
    let mut executor = Executor {
        client,
        ignored: server.subuser_ignored_files.clone(),
        server,
        user,
    };

    for (index, kind) in planned.iter().enumerate() {
        let Some(kind) = kind else {
            progress[index].status = StepStatus::Skipped;
            progress[index].message = Some("condition not met".to_string());
            Install::set_progress(&state.database, install, progress).await?;
            continue;
        };

        progress[index].status = StepStatus::Running;
        Install::set_progress(&state.database, install, progress).await?;

        let result = {
            let step = executor.execute(kind);
            tokio::pin!(step);
            let mut heartbeat = tokio::time::interval_at(
                tokio::time::Instant::now() + HEARTBEAT,
                HEARTBEAT,
            );
            loop {
                tokio::select! {
                    result = &mut step => break result,
                    _ = heartbeat.tick() => {
                        if let Err(err) = Install::touch(&state.database, install).await {
                            tracing::warn!(%install, "failed to refresh template install: {err}");
                        }
                    }
                }
            }
        };

        match result {
            Ok(message) => {
                progress[index].status = StepStatus::Success;
                progress[index].message = message;
            }
            Err(message) => {
                let label = progress[index]
                    .name
                    .clone()
                    .unwrap_or_else(|| progress[index].r#type.clone());
                let error = format!("step {} ({label}) failed: {message}", index + 1);
                progress[index].status = StepStatus::Failed;
                progress[index].message = Some(message);
                Install::finish(
                    &state.database,
                    install,
                    InstallStatus::Failed,
                    progress,
                    Some(&error),
                )
                .await?;
                return Ok(());
            }
        }
    }

    Install::finish(
        &state.database,
        install,
        InstallStatus::Success,
        progress,
        None,
    )
    .await?;
    Ok(())
}

async fn connect(state: &State, server: &Server) -> Result<WingsClient, anyhow::Error> {
    Ok(server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .ignoring(server.subuser_ignored_files.clone().unwrap_or_default()))
}

struct Executor {
    client: WingsClient,
    server: Server,
    ignored: Option<Vec<compact_str::CompactString>>,
    user: Uuid,
}

impl Executor {
    async fn execute(&mut self, kind: &StepKind) -> StepResult {
        match kind {
            StepKind::WriteFile { path, content } => {
                self.write(path, content.as_bytes().to_vec()).await?;
                Ok(None)
            }
            StepKind::ReplaceInFile {
                path,
                search,
                replace,
                regex,
            } => self.replace(path, search, replace, *regex).await,
            StepKind::AppendFile { path, content } => {
                let mut data = self.read(path).await?.unwrap_or_default();
                check_output_size(path, data.len().saturating_add(content.len()))?;
                data.extend_from_slice(content.as_bytes());
                self.write(path, data).await?;
                Ok(None)
            }
            StepKind::PrependFile { path, content } => {
                let existing = self.read(path).await?.unwrap_or_default();
                let total = existing.len().saturating_add(content.len());
                check_output_size(path, total)?;
                let mut data = Vec::with_capacity(total);
                data.extend_from_slice(content.as_bytes());
                data.extend_from_slice(&existing);
                self.write(path, data).await?;
                Ok(None)
            }
            StepKind::CreateFolder { path } => {
                self.create_folder(path).await?;
                Ok(None)
            }
            StepKind::PullFile {
                url,
                directory,
                file_name,
            } => self.pull(url, directory, file_name.as_deref()).await,
            StepKind::UnzipFile {
                path,
                destination,
                delete_archive,
            } => {
                self.unzip(path, destination.as_deref(), *delete_archive)
                    .await
            }
            StepKind::Move { from, to } => self.rename(from, to).await,
            StepKind::MoveFolderContents { from, to } => self.move_contents(from, to).await,
            StepKind::Delete { path } => self.delete(path).await,
            StepKind::Power { action, wait } => self.power(*action, *wait).await,
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<(), String> {
        check_output_size(path, data.len())?;
        self.client
            .post_servers_server_files_write(
                self.server.uuid,
                AsyncRequestReader::new(std::io::Cursor::new(data)),
                &wings_api::servers_server_files_write::post::Query {
                    file: Some(absolute(path).into()),
                    user: Some(self.user),
                    ignored: self.ignored.clone(),
                    ..Default::default()
                },
            )
            .await
            .map_err(wings_error)?;
        Ok(())
    }

    /// The file's bytes, `None` if it does not exist.
    async fn read(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        let reader = match self
            .client
            .get_servers_server_files_contents(
                self.server.uuid,
                &wings_api::servers_server_files_contents::get::Query {
                    file: Some(absolute(path).into()),
                    max_size: Some(MAX_FILE_BYTES as u64),
                    ignored: self.ignored.clone(),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(reader) => reader,
            Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => return Ok(None),
            Err(ApiHttpError::Http(StatusCode::PAYLOAD_TOO_LARGE, _)) => {
                return Err(format!(
                    "`{}` is larger than {} MiB",
                    absolute(path),
                    mib(MAX_FILE_BYTES)
                ));
            }
            Err(err) => return Err(wings_error(err)),
        };

        let mut data = Vec::new();
        reader
            .take(MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .await
            .map_err(|err| format!("reading `{}` failed: {err}", absolute(path)))?;
        if data.len() > MAX_FILE_BYTES {
            return Err(format!(
                "`{}` is larger than {} MiB",
                absolute(path),
                mib(MAX_FILE_BYTES)
            ));
        }
        Ok(Some(data))
    }

    async fn replace(&self, path: &str, search: &str, replace: &str, regex: bool) -> StepResult {
        let data = self
            .read(path)
            .await?
            .ok_or_else(|| format!("`{}` does not exist", absolute(path)))?;
        let content = String::from_utf8(data)
            .map_err(|_| format!("`{}` is not a UTF-8 text file", absolute(path)))?;

        let Some((count, replaced)) = apply_replace(path, &content, search, replace, regex)? else {
            return Ok(Some("no matches".to_string()));
        };
        self.write(path, replaced.into_bytes()).await?;
        Ok(Some(format!(
            "replaced {count} {}",
            if count == 1 { "match" } else { "matches" }
        )))
    }

    /// Creates the folder and its parents; existing folders are fine.
    async fn create_folder(&self, path: &str) -> Result<(), String> {
        self.client
            .post_servers_server_files_create_directory(
                self.server.uuid,
                &wings_api::servers_server_files_create_directory::post::RequestBody {
                    root: "/".into(),
                    name: path.into(),
                },
            )
            .await
            .map_err(wings_error)?;
        Ok(())
    }

    async fn pull(&self, url: &str, directory: &str, file_name: Option<&str>) -> StepResult {
        // Wings follows redirects itself and names the file after the
        // Content-Disposition header or the final url when no name is given.
        self.client
            .post_servers_server_files_pull(
                self.server.uuid,
                &wings_api::servers_server_files_pull::post::RequestBody {
                    root: absolute(directory).into(),
                    url: url.into(),
                    file_name: file_name.map(Into::into),
                    use_header: file_name.is_none(),
                    foreground: true,
                },
            )
            .await
            .map_err(wings_error)?;
        Ok(Some(match file_name {
            Some(name) => format!("downloaded to `{}`", absolute(&join_path(directory, name))),
            None => format!("downloaded into `{}`", absolute(directory)),
        }))
    }

    async fn unzip(
        &self,
        path: &str,
        destination: Option<&str>,
        delete_archive: bool,
    ) -> StepResult {
        let destination = destination.unwrap_or(split_path(path).0);
        if !destination.is_empty() {
            // Wings extracts into `root`, which has to exist
            self.create_folder(destination).await?;
        }
        self.client
            .post_servers_server_files_decompress(
                self.server.uuid,
                &wings_api::servers_server_files_decompress::post::RequestBody {
                    root: absolute(destination).into(),
                    // absolute, so it does not resolve against `root`
                    file: absolute(path).into(),
                    foreground: true,
                },
            )
            .await
            .map_err(wings_error)?;

        if delete_archive {
            self.delete(path).await?;
        }
        Ok(Some(format!("extracted into `{}`", absolute(destination))))
    }

    async fn rename(&self, from: &str, to: &str) -> StepResult {
        let renamed = self
            .rename_all(vec![(from.to_string(), to.to_string())])
            .await?;
        if renamed == 0 {
            return Err(format!(
                "could not move `{}` to `{}`: the source is missing or the destination already exists",
                absolute(from),
                absolute(to)
            ));
        }
        Ok(None)
    }

    /// Renames relative to the server root, creating parent folders. Wings
    /// skips entries whose source is missing or whose target exists.
    async fn rename_all(&self, files: Vec<(String, String)>) -> Result<u64, String> {
        let body = wings_api::servers_server_files_rename::put::RequestBody {
            root: "/".into(),
            files: files
                .into_iter()
                .map(
                    |(from, to)| wings_api::servers_server_files_rename::put::RequestBodyFiles {
                        from: from.into(),
                        to: to.into(),
                    },
                )
                .collect(),
        };
        let extra = wings_api::servers_server_files_rename::put::Extra {
            create_directories: true,
            ..Default::default()
        };
        Ok(self
            .client
            .put_servers_server_files_rename_with(self.server.uuid, &body, &extra)
            .await
            .map_err(wings_error)?
            .renamed)
    }

    /// Names of the entries of a folder, `None` if it does not exist.
    async fn list(&self, directory: &str) -> Result<Option<Vec<String>>, String> {
        let mut query = wings_api::servers_server_files_list::get::Query {
            directory: Some(absolute(directory).into()),
            ignored: self.ignored.clone(),
            per_page: Some(LIST_PER_PAGE),
            ..Default::default()
        };
        let mut names = Vec::new();
        for page in 1..=LIST_MAX_PAGES {
            query.page = Some(page);
            let response = match self
                .client
                .get_servers_server_files_list(self.server.uuid, &query)
                .await
            {
                Ok(response) => response,
                Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) if page == 1 => {
                    return Ok(None);
                }
                Err(err) => return Err(wings_error(err)),
            };
            let received = response.entries.len() as u64;
            names.extend(response.entries.into_iter().map(|entry| entry.name.to_string()));
            if received < LIST_PER_PAGE || names.len() as u64 >= response.total {
                return Ok(Some(names));
            }
        }
        Err(format!(
            "`{}` has more than {} entries",
            absolute(directory),
            LIST_PER_PAGE * LIST_MAX_PAGES
        ))
    }

    async fn move_contents(&mut self, from: &str, to: &str) -> StepResult {
        if from == to {
            return Err("source and destination are the same folder".to_string());
        }
        if !to.is_empty() {
            self.create_folder(to).await?;
        }
        // moving a folder into one of its subfolders leaves that subfolder in place
        let keep = to
            .strip_prefix(from)
            .and_then(|rest| rest.strip_prefix('/'))
            .and_then(|rest| rest.split('/').next())
            .map(ToString::to_string);

        let names: Vec<String> = self
            .list(from)
            .await?
            .ok_or_else(|| format!("`{}` does not exist", absolute(from)))?
            .into_iter()
            .filter(|name| Some(name) != keep.as_ref())
            .collect();
        if names.is_empty() {
            return Ok(Some("the folder is empty".to_string()));
        }

        let mut files = Vec::with_capacity(names.len());
        for name in &names {
            let source = join_path(from, name);
            let target = join_path(to, name);
            for path in [&source, &target] {
                if self.server.is_ignored_either(absolute(path)) {
                    return Err(format!("`{}` is not accessible", absolute(path)));
                }
            }
            files.push((source, target));
        }

        let total = files.len() as u64;
        let renamed = self.rename_all(files).await?;
        if renamed < total {
            let left: Vec<String> = self
                .list(from)
                .await?
                .unwrap_or_default()
                .into_iter()
                .filter(|name| Some(name) != keep.as_ref())
                .collect();
            return Err(format!(
                "moved {renamed} of {total} entries; already present in `{}`: {}",
                absolute(to),
                left.join(", ")
            ));
        }
        Ok(Some(format!(
            "moved {renamed} {}",
            if renamed == 1 { "entry" } else { "entries" }
        )))
    }

    async fn delete(&self, path: &str) -> StepResult {
        let deleted = match self
            .client
            .post_servers_server_files_delete(
                self.server.uuid,
                &wings_api::servers_server_files_delete::post::RequestBody {
                    root: "/".into(),
                    files: vec![path.into()],
                },
            )
            .await
        {
            Ok(response) => response.deleted,
            Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => 0,
            Err(err) => return Err(wings_error(err)),
        };
        Ok((deleted == 0).then(|| "nothing to delete".to_string()))
    }

    async fn power(&self, action: PowerAction, wait: bool) -> StepResult {
        // Capture pre-restart state and uptime so a fast restart completing within a single
        // poll window is still detected via a decreased uptime (and an offline server does
        // not stall waiting to leave `running`).
        let before = if action == PowerAction::Restart && wait {
            self.client
                .get_servers_server_utilization(self.server.uuid)
                .await
                .ok()
                .map(|response| (response.utilization.state, response.utilization.uptime))
        } else {
            None
        };

        self.client
            .post_servers_server_power(
                self.server.uuid,
                &wings_api::servers_server_power::post::RequestBody {
                    action: action.wings(),
                    wait_seconds: Some(POWER_LOCK_SECONDS),
                },
            )
            .await
            .map_err(wings_error)?;
        if !wait {
            return Ok(None);
        }

        let mut tracker = PowerWait::new(action, before);
        let started = tokio::time::Instant::now();
        let deadline = started + POWER_TIMEOUT;
        loop {
            let poll = if started.elapsed() < POWER_POLL_FAST_WINDOW {
                POWER_POLL_FAST
            } else {
                POWER_POLL
            };
            tokio::time::sleep(poll).await;
            // the utilization payload is small and stable across Wings versions, unlike the
            // full server details (older nodes fail to deserialize those)
            let utilization = self
                .client
                .get_servers_server_utilization(self.server.uuid)
                .await
                .map_err(wings_error)?
                .utilization;
            if let Some(message) = tracker.observe(utilization.state, utilization.uptime) {
                return Ok(Some(message.to_string()));
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "the server did not become {} within {} seconds (state: {})",
                    if tracker.want_running {
                        "running"
                    } else {
                        "offline"
                    },
                    POWER_TIMEOUT.as_secs(),
                    state_name(utilization.state)
                ));
            }
        }
    }
}

fn apply_replace(
    path: &str,
    content: &str,
    search: &str,
    replace: &str,
    regex: bool,
) -> Result<Option<(usize, String)>, String> {
    if regex {
        let pattern = search_regex(search).map_err(|err| format!("invalid regex: {err}"))?;
        let count = pattern.find_iter(content).count();
        if count == 0 {
            return Ok(None);
        }
        let replaced = pattern.replace_all(content, replace).into_owned();
        check_output_size(path, replaced.len())?;
        Ok(Some((count, replaced)))
    } else {
        let count = content.matches(search).count();
        if count == 0 {
            return Ok(None);
        }
        let removed = count.saturating_mul(search.len());
        let added = count.saturating_mul(replace.len());
        let projected = content.len().saturating_sub(removed).saturating_add(added);
        check_output_size(path, projected)?;
        let replaced = content.replace(search, replace);
        check_output_size(path, replaced.len())?;
        Ok(Some((count, replaced)))
    }
}

struct PowerWait {
    want_running: bool,
    left_running: bool,
    previous_uptime: Option<u64>,
}

impl PowerWait {
    fn new(action: PowerAction, before: Option<(wings_api::ServerState, u64)>) -> Self {
        let want_running = matches!(action, PowerAction::Start | PowerAction::Restart);
        let (left_running, previous_uptime) = if action == PowerAction::Restart {
            match before {
                Some((wings_api::ServerState::Running, uptime)) => (false, Some(uptime)),
                Some(_) => (true, None),
                None => (false, None),
            }
        } else {
            (true, None)
        };
        Self {
            want_running,
            left_running,
            previous_uptime,
        }
    }

    fn observe(&mut self, state: wings_api::ServerState, uptime: u64) -> Option<&'static str> {
        match state {
            wings_api::ServerState::Offline if !self.want_running => {
                Some("the server is offline")
            }
            wings_api::ServerState::Running if self.want_running => {
                if !self.left_running && self.previous_uptime.is_some_and(|prev| uptime < prev) {
                    self.left_running = true;
                }
                self.left_running.then_some("the server is running")
            }
            wings_api::ServerState::Running => None,
            _ => {
                self.left_running = true;
                None
            }
        }
    }
}

fn state_name(state: wings_api::ServerState) -> &'static str {
    match state {
        wings_api::ServerState::Offline => "offline",
        wings_api::ServerState::Starting => "starting",
        wings_api::ServerState::Stopping => "stopping",
        wings_api::ServerState::Running => "running",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wings_api::ServerState;

    #[test]
    fn power_wait_detects_fast_restart_via_decreased_uptime() {
        let mut wait = PowerWait::new(PowerAction::Restart, Some((ServerState::Running, 60_000)));
        assert_eq!(wait.observe(ServerState::Running, 61_000), None);
        assert_eq!(
            wait.observe(ServerState::Running, 400),
            Some("the server is running")
        );
    }

    #[test]
    fn power_wait_handles_normal_restart_and_offline_start() {
        let mut wait = PowerWait::new(PowerAction::Restart, Some((ServerState::Running, 10_000)));
        assert_eq!(wait.observe(ServerState::Stopping, 0), None);
        assert_eq!(wait.observe(ServerState::Starting, 0), None);
        assert_eq!(
            wait.observe(ServerState::Running, 12_000),
            Some("the server is running")
        );

        let mut from_offline =
            PowerWait::new(PowerAction::Restart, Some((ServerState::Offline, 0)));
        assert_eq!(
            from_offline.observe(ServerState::Running, 100),
            Some("the server is running")
        );

        let mut fallback = PowerWait::new(PowerAction::Restart, None);
        assert_eq!(fallback.observe(ServerState::Running, 500), None);
        assert_eq!(fallback.observe(ServerState::Stopping, 0), None);
        assert_eq!(
            fallback.observe(ServerState::Running, 100),
            Some("the server is running")
        );
    }

    #[test]
    fn power_wait_stop_and_start() {
        let mut stop = PowerWait::new(PowerAction::Stop, None);
        assert_eq!(stop.observe(ServerState::Running, 5_000), None);
        assert_eq!(stop.observe(ServerState::Stopping, 0), None);
        assert_eq!(
            stop.observe(ServerState::Offline, 0),
            Some("the server is offline")
        );

        let mut start = PowerWait::new(PowerAction::Start, None);
        assert_eq!(start.observe(ServerState::Starting, 0), None);
        assert_eq!(
            start.observe(ServerState::Running, 10),
            Some("the server is running")
        );
    }

    #[test]
    fn output_size_guard_rejects_oversized_writes_and_replaces() {
        assert!(check_output_size("a.txt", MAX_FILE_BYTES).is_ok());
        assert!(check_output_size("a.txt", MAX_FILE_BYTES + 1).is_err());

        // Literal expansion exceeding MAX_FILE_BYTES is rejected before allocating replacement
        let content = "x".repeat(10_000);
        let huge = "y".repeat(1000);
        let err = apply_replace("a.txt", &content, "x", &huge, false).unwrap_err();
        assert!(err.contains("would exceed"), "{err}");

        // Regex expansion exceeding MAX_FILE_BYTES is also rejected
        let err = apply_replace("a.txt", &content, "x", &huge, true).unwrap_err();
        assert!(err.contains("would exceed"), "{err}");

        // No matches returns Ok(None)
        assert_eq!(
            apply_replace("a.txt", "hello", "missing", "world", false).unwrap(),
            None
        );
        assert_eq!(
            apply_replace("a.txt", "hello", "hello", "world", false).unwrap(),
            Some((1, "world".to_string()))
        );
    }
}

