//! Template payloads: limits, variable keys and rules, step templates.
use crate::{
    model::{FieldKind, Step, Variable, normalize_path},
    render,
};
use garde::Validate;
use serde::Deserialize;
use std::{collections::HashSet, sync::LazyLock};
use utoipa::ToSchema;
use uuid::Uuid;

pub const MAX_STEPS: usize = 100;
pub const MAX_VARIABLES: usize = 50;
/// Content, search and replace fields.
pub const MAX_CONTENT_BYTES: usize = 1024 * 1024;
/// Paths, urls, file names and conditions.
const MAX_FIELD_BYTES: usize = 4096;
const MAX_DEFAULT_VALUE_CHARS: usize = 16384;
const MAX_RULES: usize = 50;

/// Keys that would shadow the template context or a function.
const RESERVED_KEYS: &[&str] = &[
    "server",
    "user",
    "panel",
    "vars",
    "loop",
    "self",
    "random_string",
    "uuid",
    "now",
    "range",
    "dict",
    "debug",
    "namespace",
    "true",
    "false",
    "none",
    "True",
    "False",
    "None",
];

static KEY: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("^[A-Za-z_][A-Za-z0-9_]{0,63}$").expect("key regex"));

#[derive(ToSchema, Validate, Deserialize)]
pub struct TemplatePayload {
    #[garde(length(chars, min = 1, max = 255))]
    #[schema(min_length = 1, max_length = 255)]
    pub name: String,
    #[garde(inner(length(chars, max = 4096)))]
    #[schema(max_length = 4096)]
    #[serde(default)]
    pub description: Option<String>,
    #[garde(skip)]
    pub enabled: bool,
    /// Empty means every egg.
    #[garde(length(max = 1000))]
    #[serde(default)]
    pub egg_uuids: Vec<Uuid>,
    #[garde(skip)]
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[garde(skip)]
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// A checked and normalized template.
#[derive(Debug)]
pub struct TemplateData {
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub egg_uuids: Vec<Uuid>,
    pub variables: Vec<Variable>,
    pub steps: Vec<Step>,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

impl TemplatePayload {
    pub fn into_data(self) -> Result<TemplateData, Vec<String>> {
        let mut errors = Vec::new();
        let name = self.name.trim().to_string();
        if name.is_empty() {
            errors.push("name: must not be empty".to_string());
        }
        let variables = check_variables(self.variables, &mut errors);
        let steps = check_steps(self.steps, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut seen = HashSet::new();
        Ok(TemplateData {
            name,
            description: non_empty(self.description),
            enabled: self.enabled,
            egg_uuids: self
                .egg_uuids
                .into_iter()
                .filter(|uuid| seen.insert(*uuid))
                .collect(),
            variables,
            steps,
        })
    }
}

fn check_variables(variables: Vec<Variable>, errors: &mut Vec<String>) -> Vec<Variable> {
    if variables.len() > MAX_VARIABLES {
        errors.push(format!(
            "variables: at most {MAX_VARIABLES} variables are allowed"
        ));
        return variables;
    }

    let mut keys = HashSet::new();
    variables
        .into_iter()
        .enumerate()
        .map(|(index, variable)| {
            let at = format!("variables[{index}]");
            let variable = Variable {
                key: variable.key.trim().to_string(),
                label: variable.label.trim().to_string(),
                description: non_empty(variable.description),
                default_value: variable.default_value,
                rules: variable
                    .rules
                    .into_iter()
                    .map(|rule| rule.trim().into())
                    .filter(|rule: &compact_str::CompactString| !rule.is_empty())
                    .collect(),
            };

            if !KEY.is_match(&variable.key) {
                errors.push(format!(
                    "{at}.key: `{}` must start with a letter or underscore and contain only letters, digits and underscores (at most 64)",
                    variable.key
                ));
            } else if RESERVED_KEYS.contains(&variable.key.as_str()) {
                errors.push(format!("{at}.key: `{}` is reserved", variable.key));
            } else if !keys.insert(variable.key.clone()) {
                errors.push(format!("{at}.key: `{}` is used twice", variable.key));
            }
            let label_chars = variable.label.chars().count();
            if label_chars == 0 || label_chars > 128 {
                errors.push(format!("{at}.label: must be 1 to 128 characters"));
            }
            if variable
                .description
                .as_ref()
                .is_some_and(|description| description.chars().count() > 1024)
            {
                errors.push(format!("{at}.description: must be at most 1024 characters"));
            }
            if variable.default_value.chars().count() > MAX_DEFAULT_VALUE_CHARS {
                errors.push(format!(
                    "{at}.default_value: must be at most {MAX_DEFAULT_VALUE_CHARS} characters"
                ));
            }
            if variable.rules.len() > MAX_RULES {
                errors.push(format!("{at}.rules: at most {MAX_RULES} rules are allowed"));
            } else if let Err(err) = rule_validator::validate_rules(&variable.rules, &()) {
                errors.push(format!("{at}.rules: {err}"));
            }
            variable
        })
        .collect()
}

/// Whether a field contains template syntax, i.e. is only known after rendering.
fn is_literal(value: &str) -> bool {
    !(value.contains("{{") || value.contains("{%") || value.contains("{#"))
}

/// Checks a rendered or literal field value and returns it normalized.
pub fn check_value(kind: FieldKind, value: &str) -> Result<String, String> {
    match kind {
        FieldKind::Path { allow_root } => {
            let path = normalize_path(value)?;
            if path.is_empty() && !allow_root {
                return Err("must not be empty or the server root".to_string());
            }
            Ok(path)
        }
        FieldKind::OptionalPath => {
            if value.trim().is_empty() {
                Ok(String::new())
            } else {
                normalize_path(value)
            }
        }
        FieldKind::FileName => {
            let name = value.trim();
            if name.contains('/') || name == "." || name == ".." || name.contains('\0') {
                return Err(format!("`{name}` is not a single file name"));
            }
            Ok(name.to_string())
        }
        FieldKind::Url => {
            let url = value.trim();
            match url.split_once("://") {
                Some((scheme, rest))
                    if (scheme.eq_ignore_ascii_case("http")
                        || scheme.eq_ignore_ascii_case("https"))
                        && !rest.is_empty() =>
                {
                    Ok(url.to_string())
                }
                _ => Err(format!("`{url}` is not an http(s) url")),
            }
        }
        FieldKind::Content => Ok(value.to_string()),
    }
}

fn check_steps(steps: Vec<Step>, errors: &mut Vec<String>) -> Vec<Step> {
    if steps.len() > MAX_STEPS {
        errors.push(format!("steps: at most {MAX_STEPS} steps are allowed"));
        return steps;
    }

    steps
        .into_iter()
        .enumerate()
        .map(|(index, mut step)| {
            let at = format!("steps[{index}]");
            step.name = non_empty(step.name);
            step.condition = step.condition.filter(|c| !c.trim().is_empty());
            step.kind.clear_empty_options();

            if step
                .name
                .as_ref()
                .is_some_and(|name| name.chars().count() > 128)
            {
                errors.push(format!("{at}.name: must be at most 128 characters"));
            }
            if let Some(condition) = &step.condition {
                if condition.len() > MAX_FIELD_BYTES {
                    errors.push(format!(
                        "{at}.condition: must be at most {MAX_FIELD_BYTES} bytes"
                    ));
                } else if let Err(err) = render::check_syntax(condition) {
                    errors.push(format!("{at}.condition: {err}"));
                }
            }

            let regex = match &step.kind {
                crate::model::StepKind::ReplaceInFile { regex, .. } => *regex,
                _ => false,
            };
            for field in step.kind.fields_mut() {
                let limit = match field.kind {
                    FieldKind::Content => MAX_CONTENT_BYTES,
                    _ => MAX_FIELD_BYTES,
                };
                if field.value.len() > limit {
                    errors.push(format!(
                        "{at}.{}: must be at most {limit} bytes",
                        field.name
                    ));
                    continue;
                }
                if let Err(err) = render::check_syntax(field.value) {
                    errors.push(format!("{at}.{}: {err}", field.name));
                    continue;
                }
                if !is_literal(field.value) {
                    continue;
                }
                if regex
                    && field.name == "search"
                    && let Err(err) = crate::model::search_regex(field.value)
                {
                    errors.push(format!("{at}.search: invalid regex: {err}"));
                }
                match check_value(field.kind, field.value) {
                    Ok(value) if field.kind != FieldKind::Content => *field.value = value,
                    Ok(_) => {}
                    Err(err) => errors.push(format!("{at}.{}: {err}", field.name)),
                }
            }
            if let crate::model::StepKind::ReplaceInFile { search, .. } = &step.kind
                && search.is_empty()
            {
                errors.push(format!("{at}.search: must not be empty"));
            }
            step.kind.clear_empty_options();
            step
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PowerAction, StepKind};

    fn variable(key: &str) -> Variable {
        Variable {
            key: key.to_string(),
            label: "Label".to_string(),
            description: None,
            default_value: String::new(),
            rules: vec!["required".into(), "string".into(), "max:32".into()],
        }
    }

    fn step(kind: StepKind) -> Step {
        Step {
            name: None,
            condition: None,
            kind,
        }
    }

    fn payload(variables: Vec<Variable>, steps: Vec<Step>) -> TemplatePayload {
        TemplatePayload {
            name: " Paper config ".to_string(),
            description: Some("  ".to_string()),
            enabled: true,
            egg_uuids: vec![Uuid::nil(), Uuid::nil()],
            variables,
            steps,
        }
    }

    #[test]
    fn valid_payload_is_normalized() {
        let data = payload(
            vec![variable("MOTD")],
            vec![
                Step {
                    name: Some("  ".to_string()),
                    condition: Some(" ".to_string()),
                    kind: StepKind::WriteFile {
                        path: "/config/../server.properties".to_string(),
                        content: "motd={{ MOTD }}\n".to_string(),
                    },
                },
                step(StepKind::UnzipFile {
                    path: "{{ server.uuid_short }}/a.zip".to_string(),
                    destination: Some(" ".to_string()),
                    delete_archive: true,
                }),
                step(StepKind::Power {
                    action: PowerAction::Restart,
                    wait: true,
                }),
            ],
        )
        .into_data()
        .unwrap();

        assert_eq!(data.name, "Paper config");
        assert_eq!(data.description, None);
        assert_eq!(data.egg_uuids, vec![Uuid::nil()]);
        assert_eq!(data.steps[0].name, None);
        assert_eq!(data.steps[0].condition, None);
        assert_eq!(
            data.steps[0].kind,
            StepKind::WriteFile {
                path: "server.properties".to_string(),
                content: "motd={{ MOTD }}\n".to_string(),
            }
        );
        assert_eq!(
            data.steps[1].kind,
            StepKind::UnzipFile {
                path: "{{ server.uuid_short }}/a.zip".to_string(),
                destination: None,
                delete_archive: true,
            }
        );
    }

    #[test]
    fn variable_keys_and_rules_are_checked() {
        let mut bad_rules = variable("PORT");
        bad_rules.rules = vec!["between:1".into()];
        let errors = payload(
            vec![
                variable("server"),
                variable("1abc"),
                variable("MOTD"),
                variable("MOTD"),
                variable("random_string"),
                bad_rules,
            ],
            Vec::new(),
        )
        .into_data()
        .unwrap_err();

        assert_eq!(errors.len(), 5, "{errors:?}");
        assert_eq!(errors[0], "variables[0].key: `server` is reserved");
        assert!(errors[1].starts_with("variables[1].key: `1abc` must start"));
        assert_eq!(errors[2], "variables[3].key: `MOTD` is used twice");
        assert_eq!(errors[3], "variables[4].key: `random_string` is reserved");
        assert!(errors[4].starts_with("variables[5].rules: "), "{}", errors[4]);
    }

    #[test]
    fn template_syntax_errors_name_the_field() {
        let errors = payload(
            Vec::new(),
            vec![
                step(StepKind::CreateFolder {
                    path: "plugins".to_string(),
                }),
                Step {
                    name: None,
                    condition: Some("{% if %}".to_string()),
                    kind: StepKind::Delete {
                        path: "x".to_string(),
                    },
                },
                step(StepKind::ReplaceInFile {
                    path: "a.txt".to_string(),
                    search: "(".to_string(),
                    replace: "{{ unclosed".to_string(),
                    regex: true,
                }),
            ],
        )
        .into_data()
        .unwrap_err();

        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].starts_with("steps[1].condition: template error"));
        assert!(errors[1].starts_with("steps[2].search: invalid regex"));
        assert!(errors[2].starts_with("steps[2].replace: template error"));
    }

    #[test]
    fn literal_paths_and_urls_are_checked() {
        let errors = payload(
            Vec::new(),
            vec![
                step(StepKind::Delete {
                    path: "/".to_string(),
                }),
                step(StepKind::Move {
                    from: "a".to_string(),
                    to: "../b".to_string(),
                }),
                step(StepKind::PullFile {
                    url: "ftp://example.com/a.zip".to_string(),
                    directory: "/".to_string(),
                    file_name: Some("a/b.zip".to_string()),
                }),
            ],
        )
        .into_data()
        .unwrap_err();

        assert_eq!(
            errors,
            vec![
                "steps[0].path: must not be empty or the server root",
                "steps[1].to: `../b` escapes the server root",
                "steps[2].url: `ftp://example.com/a.zip` is not an http(s) url",
                "steps[2].file_name: `a/b.zip` is not a single file name",
            ]
        );
    }

    #[test]
    fn limits_are_enforced() {
        let errors = payload(
            (0..=MAX_VARIABLES).map(|i| variable(&format!("V{i}"))).collect(),
            vec![step(StepKind::WriteFile {
                path: "a".to_string(),
                content: "x".repeat(MAX_CONTENT_BYTES + 1),
            })],
        )
        .into_data()
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                "variables: at most 50 variables are allowed".to_string(),
                format!("steps[0].content: must be at most {MAX_CONTENT_BYTES} bytes"),
            ]
        );

        let steps = (0..=MAX_STEPS)
            .map(|_| {
                step(StepKind::Delete {
                    path: "a".to_string(),
                })
            })
            .collect();
        assert_eq!(
            payload(Vec::new(), steps).into_data().unwrap_err(),
            vec!["steps: at most 100 steps are allowed"]
        );
    }
}
