//! Template document types shared by the API, the database, the export
//! format and the executor.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A value the user fills in before installing.
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: String,
    /// Laravel-style rules, same syntax as egg variables.
    #[serde(default)]
    pub rules: Vec<compact_str::CompactString>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Start,
    Stop,
    Restart,
    Kill,
}

impl PowerAction {
    /// The core permission a user needs to send this action.
    pub fn permission(self) -> &'static str {
        match self {
            Self::Start => "control.start",
            Self::Stop | Self::Kill => "control.stop",
            Self::Restart => "control.restart",
        }
    }

    pub fn wings(self) -> wings_api::ServerPowerAction {
        match self {
            Self::Start => wings_api::ServerPowerAction::Start,
            Self::Stop => wings_api::ServerPowerAction::Stop,
            Self::Restart => wings_api::ServerPowerAction::Restart,
            Self::Kill => wings_api::ServerPowerAction::Kill,
        }
    }
}

fn default_true() -> bool {
    true
}

/// What a step does; every string field is a minijinja template.
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepKind {
    WriteFile {
        path: String,
        content: String,
    },
    ReplaceInFile {
        path: String,
        search: String,
        replace: String,
        #[serde(default)]
        regex: bool,
    },
    AppendFile {
        path: String,
        content: String,
    },
    PrependFile {
        path: String,
        content: String,
    },
    CreateFolder {
        path: String,
    },
    PullFile {
        url: String,
        #[serde(default)]
        directory: String,
        #[serde(default)]
        file_name: Option<String>,
    },
    UnzipFile {
        path: String,
        #[serde(default)]
        destination: Option<String>,
        #[serde(default)]
        delete_archive: bool,
    },
    Move {
        from: String,
        to: String,
    },
    MoveFolderContents {
        from: String,
        to: String,
    },
    Delete {
        path: String,
    },
    Power {
        action: PowerAction,
        #[serde(default = "default_true")]
        wait: bool,
    },
}

/// How a string field of a step is checked and post-processed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// A server path; `allow_root` says whether the server root itself is valid.
    Path { allow_root: bool },
    /// Empty means "not set" (e.g. the archive's own directory).
    OptionalPath,
    /// A single file name, empty means "not set".
    FileName,
    Url,
    /// File contents, search/replace strings.
    Content,
}

/// A mutable string field of a step.
pub struct Field<'a> {
    pub name: &'static str,
    pub kind: FieldKind,
    pub value: &'a mut String,
}

const fn field<'a>(name: &'static str, kind: FieldKind, value: &'a mut String) -> Field<'a> {
    Field { name, kind, value }
}

const PATH: FieldKind = FieldKind::Path { allow_root: false };
const PATH_OR_ROOT: FieldKind = FieldKind::Path { allow_root: true };

impl StepKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::WriteFile { .. } => "write_file",
            Self::ReplaceInFile { .. } => "replace_in_file",
            Self::AppendFile { .. } => "append_file",
            Self::PrependFile { .. } => "prepend_file",
            Self::CreateFolder { .. } => "create_folder",
            Self::PullFile { .. } => "pull_file",
            Self::UnzipFile { .. } => "unzip_file",
            Self::Move { .. } => "move",
            Self::MoveFolderContents { .. } => "move_folder_contents",
            Self::Delete { .. } => "delete",
            Self::Power { .. } => "power",
        }
    }

    /// Every templated string field of the step.
    pub fn fields_mut(&mut self) -> Vec<Field<'_>> {
        match self {
            Self::WriteFile { path, content }
            | Self::AppendFile { path, content }
            | Self::PrependFile { path, content } => vec![
                field("path", PATH, path),
                field("content", FieldKind::Content, content),
            ],
            Self::ReplaceInFile {
                path,
                search,
                replace,
                ..
            } => vec![
                field("path", PATH, path),
                field("search", FieldKind::Content, search),
                field("replace", FieldKind::Content, replace),
            ],
            Self::CreateFolder { path } | Self::Delete { path } => vec![field("path", PATH, path)],
            Self::PullFile {
                url,
                directory,
                file_name,
            } => {
                let mut fields = vec![
                    field("url", FieldKind::Url, url),
                    field("directory", PATH_OR_ROOT, directory),
                ];
                if let Some(file_name) = file_name {
                    fields.push(field("file_name", FieldKind::FileName, file_name));
                }
                fields
            }
            Self::UnzipFile {
                path, destination, ..
            } => {
                let mut fields = vec![field("path", PATH, path)];
                if let Some(destination) = destination {
                    fields.push(field("destination", FieldKind::OptionalPath, destination));
                }
                fields
            }
            Self::Move { from, to } => vec![field("from", PATH, from), field("to", PATH, to)],
            Self::MoveFolderContents { from, to } => {
                vec![field("from", PATH, from), field("to", PATH_OR_ROOT, to)]
            }
            Self::Power { .. } => Vec::new(),
        }
    }

    /// Turns optional fields that ended up empty into `None`.
    pub fn clear_empty_options(&mut self) {
        let blank = |value: &Option<String>| value.as_deref().is_some_and(|v| v.trim().is_empty());
        match self {
            Self::PullFile { file_name, .. } if blank(file_name) => *file_name = None,
            Self::UnzipFile { destination, .. } if blank(destination) => *destination = None,
            _ => {}
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Step {
    #[serde(default)]
    pub name: Option<String>,
    /// Template; the step runs only when the rendered result is truthy.
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(flatten)]
    pub kind: StepKind,
}

/// Whether a rendered condition lets its step run.
pub fn is_truthy(rendered: &str) -> bool {
    let value = rendered.trim();
    !["", "0", "false", "no", "off"]
        .iter()
        .any(|falsy| value.eq_ignore_ascii_case(falsy))
}

/// Resolves `.` and `..` in a server path, returning it without leading
/// slash (`""` is the server root). Paths escaping the root are rejected.
pub fn normalize_path(path: &str) -> Result<String, String> {
    if path.contains('\0') {
        return Err("the path contains a NUL character".to_string());
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.trim().split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(format!("`{path}` escapes the server root"));
                }
            }
            segment => segments.push(segment),
        }
    }
    Ok(segments.join("/"))
}

/// Parent directory and file name of a normalized, non-root path.
pub fn split_path(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", path),
    }
}

/// `path` as the absolute path Wings expects.
pub fn absolute(path: &str) -> String {
    format!("/{path}")
}

/// The pattern of a regex `replace_in_file` step. `^`/`$` match at line
/// boundaries, since config files are edited line by line (`\A`/`\z` anchor
/// the whole file).
pub fn search_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    regex::RegexBuilder::new(pattern).multi_line(true).build()
}

pub const EXPORT_FORMAT: &str = "calagopus-template-installer";
pub const EXPORT_VERSION: u32 = 1;

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct ExportTemplate {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// The portable JSON document of a template.
#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct ExportDocument {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub exported_at: Option<chrono::DateTime<chrono::Utc>>,
    pub template: ExportTemplate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falsy_conditions() {
        for value in ["", "  ", "0", "false", "FALSE", " No ", "off", "Off\n"] {
            assert!(!is_truthy(value), "{value:?}");
        }
        for value in ["1", "true", "yes", "on", "anything", "00", "none"] {
            assert!(is_truthy(value), "{value:?}");
        }
    }

    #[test]
    fn paths_are_normalized() {
        assert_eq!(normalize_path("/config//server.properties").unwrap(), "config/server.properties");
        assert_eq!(normalize_path("./a/./b/../c/").unwrap(), "a/c");
        assert_eq!(normalize_path("/").unwrap(), "");
        assert_eq!(normalize_path("a/..").unwrap(), "");
        assert!(normalize_path("../etc/passwd").is_err());
        assert!(normalize_path("a/../../b").is_err());
        assert!(normalize_path("a\0b").is_err());
    }

    #[test]
    fn split_paths() {
        assert_eq!(split_path("a/b/c.zip"), ("a/b", "c.zip"));
        assert_eq!(split_path("c.zip"), ("", "c.zip"));
    }

    #[test]
    fn steps_use_the_documented_json_shape() {
        let step: Step = serde_json::from_value(serde_json::json!({
            "type": "unzip_file",
            "name": null,
            "path": "a.zip",
        }))
        .unwrap();
        assert_eq!(
            step.kind,
            StepKind::UnzipFile {
                path: "a.zip".to_string(),
                destination: None,
                delete_archive: false,
            }
        );
        assert_eq!(step.condition, None);

        let step: Step =
            serde_json::from_value(serde_json::json!({ "type": "power", "action": "restart" }))
                .unwrap();
        assert_eq!(
            step.kind,
            StepKind::Power {
                action: PowerAction::Restart,
                wait: true,
            }
        );
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "name": null, "condition": null, "type": "power", "action": "restart", "wait": true,
            })
        );

        assert!(serde_json::from_value::<Step>(serde_json::json!({ "type": "nope" })).is_err());
    }

    #[test]
    fn search_regex_anchors_match_lines() {
        let pattern = search_regex("^motd=(.*)$").unwrap();
        assert_eq!(
            pattern.replace_all("pvp=true\nmotd=Old\nport=1", "motd=[$1]"),
            "pvp=true\nmotd=[Old]\nport=1"
        );
    }
}
