//! Starting an install: user values, rendering every step up front, access
//! checks, the install row and the background executor.
use crate::{
    db::{Install, StepProgress, StepStatus, Template},
    model::{FieldKind, Step, StepKind, Variable, absolute, is_truthy, split_path},
    render::{PanelContext, Renderer, ServerContext, UserContext},
    validate::check_value,
};
use serde::Deserialize;
use shared::{
    State,
    models::{server::Server, user::User},
};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Largest value a user may submit for a variable.
const MAX_VALUE_CHARS: usize = 65536;
/// Largest rendered content/search/replace field and file read for editing.
pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

#[derive(ToSchema, Deserialize, Clone, Debug)]
pub struct SubmittedVariable {
    pub key: String,
    pub value: String,
}

pub enum Outcome {
    Started(Uuid),
    Invalid(Vec<String>),
    /// Another install is running on the server.
    Busy,
}

/// The core permissions the template's power steps need.
pub fn required_permissions(steps: &[Step]) -> Vec<&'static str> {
    let mut permissions = Vec::new();
    for step in steps {
        if let StepKind::Power { action, .. } = step.kind
            && !permissions.contains(&action.permission())
        {
            permissions.push(action.permission());
        }
    }
    permissions
}

/// Submitted values (defaults for missing ones) checked against the rules,
/// in template order.
pub fn check_values(
    variables: &[Variable],
    submitted: Vec<SubmittedVariable>,
) -> Result<Vec<(String, String)>, Vec<String>> {
    let mut errors = Vec::new();
    let mut values: HashMap<String, String> = HashMap::new();
    for SubmittedVariable { key, value } in submitted {
        if !variables.iter().any(|variable| variable.key == key) {
            errors.push(format!("{key}: unknown variable"));
        } else if value.chars().count() > MAX_VALUE_CHARS {
            errors.push(format!("{key}: must be at most {MAX_VALUE_CHARS} characters"));
        } else if values.insert(key.clone(), value).is_some() {
            errors.push(format!("{key}: submitted twice"));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let values: Vec<(String, String)> = variables
        .iter()
        .map(|variable| {
            let value = values
                .remove(&variable.key)
                .unwrap_or_else(|| variable.default_value.clone());
            (variable.key.clone(), value)
        })
        .collect();

    let data = variables
        .iter()
        .zip(&values)
        .map(|(variable, (key, value))| (key.as_str(), (variable.rules.as_slice(), value.as_str())))
        .collect();
    let validator = rule_validator::Validator::new(data).map_err(|err| vec![err.to_string()])?;
    validator.validate().map_err(|err| vec![err])?;

    Ok(values)
}

fn step_label(index: usize, step: &Step) -> String {
    match &step.name {
        Some(name) => format!("step {} ({name})", index + 1),
        None => format!("step {} ({})", index + 1, step.kind.type_name()),
    }
}

/// Renders every step whose condition holds; `None` marks a skipped step.
pub fn plan(renderer: &Renderer, steps: &[Step]) -> Result<Vec<Option<StepKind>>, Vec<String>> {
    let mut errors = Vec::new();
    let mut planned = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let label = step_label(index, step);
        if let Some(condition) = &step.condition {
            match renderer.render(condition) {
                Ok(rendered) if !is_truthy(&rendered) => {
                    planned.push(None);
                    continue;
                }
                Ok(_) => {}
                Err(err) => {
                    errors.push(format!("{label}: condition: {err}"));
                    planned.push(None);
                    continue;
                }
            }
        }

        let mut kind = step.kind.clone();
        for field in kind.fields_mut() {
            let result = renderer.render(field.value).and_then(|rendered| {
                if field.kind == FieldKind::Content && rendered.len() > MAX_FILE_BYTES {
                    return Err(format!(
                        "renders to more than {} MiB",
                        MAX_FILE_BYTES / 1024 / 1024
                    ));
                }
                check_value(field.kind, &rendered)
            });
            match result {
                Ok(value) => *field.value = value,
                Err(err) => errors.push(format!("{label}: {}: {err}", field.name)),
            }
        }
        if let StepKind::ReplaceInFile { search, regex, .. } = &kind {
            if search.is_empty() {
                errors.push(format!("{label}: search: renders to an empty string"));
            } else if *regex && let Err(err) = crate::model::search_regex(search) {
                errors.push(format!("{label}: search: invalid regex: {err}"));
            }
        }
        kind.clear_empty_options();
        planned.push(Some(kind));
    }

    if errors.is_empty() {
        Ok(planned)
    } else {
        Err(errors)
    }
}

/// Refuses rendered steps touching paths hidden from the (sub)user.
pub fn check_access(server: &mut Server, kind: &StepKind) -> Result<(), String> {
    let denied = |path: &str| Err(format!("`{}` is not accessible", absolute(path)));
    match kind {
        StepKind::WriteFile { path, .. }
        | StepKind::ReplaceInFile { path, .. }
        | StepKind::AppendFile { path, .. }
        | StepKind::PrependFile { path, .. } => {
            if server.is_ignored(absolute(path), false) {
                return denied(path);
            }
        }
        StepKind::CreateFolder { path } => {
            if server.is_ignored(absolute(path), true) {
                return denied(path);
            }
        }
        StepKind::PullFile {
            directory,
            file_name,
            ..
        } => {
            if server.is_ignored_subtree(absolute(directory)) {
                return denied(directory);
            }
            if let Some(file_name) = file_name {
                let path = format!("{directory}/{file_name}");
                if server.is_ignored(absolute(&path), false) {
                    return denied(&path);
                }
            }
        }
        StepKind::UnzipFile {
            path, destination, ..
        } => {
            if server.is_ignored(absolute(path), false) {
                return denied(path);
            }
            let destination = destination.as_deref().unwrap_or(split_path(path).0);
            if server.is_ignored_subtree(absolute(destination)) {
                return denied(destination);
            }
        }
        StepKind::Move { from, to } => {
            for path in [from, to] {
                if server.is_ignored_either(absolute(path)) {
                    return denied(path);
                }
            }
        }
        StepKind::MoveFolderContents { from, to } => {
            for path in [from, to] {
                if server.is_ignored(absolute(path), true) {
                    return denied(path);
                }
            }
        }
        StepKind::Delete { path } => {
            if server.is_ignored_either(absolute(path)) {
                return denied(path);
            }
        }
        StepKind::Power { .. } => {}
    }
    Ok(())
}

/// Checks and renders the install, records it and runs it in the background.
pub async fn start(
    state: &State,
    mut server: Server,
    user: &User,
    template: Template,
    submitted: Vec<SubmittedVariable>,
) -> Result<Outcome, anyhow::Error> {
    let values = match check_values(&template.variables, submitted) {
        Ok(values) => values,
        Err(errors) => return Ok(Outcome::Invalid(errors)),
    };

    let renderer = Renderer::new(
        ServerContext::load(state, &server).await?,
        UserContext {
            uuid: user.uuid,
            username: user.username.to_string(),
            email: user.email.to_string(),
        },
        PanelContext::load(state).await?,
        &values,
    );
    let planned = match plan(&renderer, &template.steps) {
        Ok(planned) => planned,
        Err(errors) => return Ok(Outcome::Invalid(errors)),
    };
    let errors: Vec<String> = planned
        .iter()
        .enumerate()
        .filter_map(|(index, kind)| {
            let kind = kind.as_ref()?;
            check_access(&mut server, kind)
                .err()
                .map(|err| format!("{}: {err}", step_label(index, &template.steps[index])))
        })
        .collect();
    if !errors.is_empty() {
        return Ok(Outcome::Invalid(errors));
    }

    let progress: Vec<StepProgress> = template
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| StepProgress {
            index,
            r#type: step.kind.type_name().to_string(),
            name: step.name.clone(),
            status: StepStatus::Pending,
            message: None,
        })
        .collect();

    Install::fail_stale(&state.database, Some(server.uuid)).await?;
    let Some(install) = Install::start(
        &state.database,
        server.uuid,
        template.uuid,
        &template.name,
        user.uuid,
        &progress,
    )
    .await?
    else {
        return Ok(Outcome::Busy);
    };

    crate::executor::spawn(state.clone(), server, user.uuid, install, planned, progress);
    Ok(Outcome::Started(install))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::PowerAction,
        render::{AllocationContext, Named},
    };
    use std::collections::BTreeMap;

    fn variable(key: &str, default: &str, rules: &[&str]) -> Variable {
        Variable {
            key: key.to_string(),
            label: key.to_string(),
            description: None,
            default_value: default.to_string(),
            rules: rules.iter().map(|rule| (*rule).into()).collect(),
        }
    }

    fn submitted(key: &str, value: &str) -> SubmittedVariable {
        SubmittedVariable {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn values_fall_back_to_defaults_and_follow_rules() {
        let variables = vec![
            variable("MOTD", "Welcome", &["required", "string", "max:32"]),
            variable("PLAYERS", "20", &["required", "integer", "between:1,100"]),
            variable("MODE", "survival", &["required", "in:survival,creative"]),
        ];
        assert_eq!(
            check_values(&variables, vec![submitted("PLAYERS", "50")]).unwrap(),
            vec![
                ("MOTD".to_string(), "Welcome".to_string()),
                ("PLAYERS".to_string(), "50".to_string()),
                ("MODE".to_string(), "survival".to_string()),
            ]
        );

        let errors = check_values(&variables, vec![submitted("PLAYERS", "500")]).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("PLAYERS: "), "{errors:?}");

        let errors = check_values(&variables, vec![submitted("MOTD", "")]).unwrap_err();
        assert!(errors[0].starts_with("MOTD: "), "{errors:?}");

        assert_eq!(
            check_values(&variables, vec![submitted("NOPE", "x")]).unwrap_err(),
            vec!["NOPE: unknown variable"]
        );
    }

    fn renderer() -> Renderer {
        Renderer::new(
            ServerContext {
                uuid: Uuid::nil(),
                uuid_short: "00000000".to_string(),
                name: "Server".to_string(),
                description: None,
                memory: 1024,
                swap: 0,
                disk: 0,
                cpu: 100,
                image: String::new(),
                startup: String::new(),
                ip: None,
                ip_alias: None,
                port: None,
                allocations: Vec::<AllocationContext>::new(),
                egg: Named {
                    uuid: Uuid::nil(),
                    name: "Egg".to_string(),
                },
                nest: Named {
                    uuid: Uuid::nil(),
                    name: "Nest".to_string(),
                },
                node: Named {
                    uuid: Uuid::nil(),
                    name: "Node".to_string(),
                },
                env: BTreeMap::new(),
                timezone: None,
            },
            UserContext {
                uuid: Uuid::nil(),
                username: "alice".to_string(),
                email: "a@example.com".to_string(),
            },
            PanelContext {
                name: "Panel".to_string(),
                url: "https://panel".to_string(),
            },
            &[
                ("DIR".to_string(), "plugins/../mods".to_string()),
                ("ON".to_string(), "0".to_string()),
            ],
        )
    }

    fn step(condition: Option<&str>, kind: StepKind) -> Step {
        Step {
            name: None,
            condition: condition.map(ToString::to_string),
            kind,
        }
    }

    #[test]
    fn plans_render_and_normalize_fields() {
        let planned = plan(
            &renderer(),
            &[
                step(
                    None,
                    StepKind::UnzipFile {
                        path: "/{{ DIR }}/pack.zip".to_string(),
                        destination: Some("{{ '' }}".to_string()),
                        delete_archive: false,
                    },
                ),
                step(
                    Some("{{ ON }}"),
                    StepKind::Delete {
                        path: "{{ undefined_thing }}".to_string(),
                    },
                ),
                step(
                    Some("{{ user.username == 'alice' }}"),
                    StepKind::Power {
                        action: PowerAction::Start,
                        wait: false,
                    },
                ),
            ],
        )
        .unwrap();

        assert_eq!(
            planned,
            vec![
                Some(StepKind::UnzipFile {
                    path: "mods/pack.zip".to_string(),
                    destination: None,
                    delete_archive: false,
                }),
                None,
                Some(StepKind::Power {
                    action: PowerAction::Start,
                    wait: false,
                }),
            ]
        );
    }

    #[test]
    fn plan_errors_name_step_and_field() {
        let errors = plan(
            &renderer(),
            &[
                step(
                    None,
                    StepKind::WriteFile {
                        path: "../{{ DIR }}".to_string(),
                        content: "{{ MISSING }}".to_string(),
                    },
                ),
                Step {
                    name: Some("Tweak".to_string()),
                    condition: None,
                    kind: StepKind::ReplaceInFile {
                        path: "a".to_string(),
                        search: "{{ '(' }}".to_string(),
                        replace: String::new(),
                        regex: true,
                    },
                },
            ],
        )
        .unwrap_err();

        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].starts_with("step 1 (write_file): path: `../plugins/../mods` escapes"));
        assert!(errors[1].starts_with("step 1 (write_file): content: template error: undefined"));
        assert!(errors[2].starts_with("step 2 (Tweak): search: invalid regex"));
    }

    #[test]
    fn power_permissions() {
        let steps = [
            step(
                None,
                StepKind::Power {
                    action: PowerAction::Kill,
                    wait: true,
                },
            ),
            step(
                None,
                StepKind::Power {
                    action: PowerAction::Stop,
                    wait: true,
                },
            ),
            step(
                None,
                StepKind::Power {
                    action: PowerAction::Restart,
                    wait: true,
                },
            ),
        ];
        assert_eq!(
            required_permissions(&steps),
            vec!["control.stop", "control.restart"]
        );
    }
}
