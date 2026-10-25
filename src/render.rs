//! Renders step fields with minijinja against the install context.
use minijinja::{
    AutoEscape, Environment, Error, ErrorKind, UndefinedBehavior,
    syntax::SyntaxConfig,
    value::{Kwargs, Serde},
};
use rand::RngExt;
use serde::Serialize;
use shared::{
    State,
    models::{server::Server, server_allocation::ServerAllocation, server_variable::ServerVariable},
};
use std::{collections::BTreeMap, fmt::Write, sync::LazyLock};
use uuid::Uuid;

const MAX_RANDOM_LENGTH: usize = 1024;

static ENV: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    // file contents keep their final newline
    env.set_syntax(
        SyntaxConfig::builder()
            .keep_trailing_newline(true)
            .build()
            .expect("valid template syntax config"),
    );
    env.add_function("random_string", random_string);
    env.add_function("uuid", || Uuid::new_v4().to_string());
    env.add_function("now", now);
    env
});

/// `random_string(16)` or `random_string(length=16)`: alphanumeric characters.
fn random_string(length: Option<usize>, kwargs: Kwargs) -> Result<String, Error> {
    let length = match length {
        Some(length) => length,
        None => kwargs.get::<Option<usize>>("length")?.unwrap_or(16),
    };
    kwargs.assert_all_used()?;
    if length > MAX_RANDOM_LENGTH {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("random_string length must be at most {MAX_RANDOM_LENGTH}"),
        ));
    }
    Ok(rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect())
}

/// `now()` or `now(format="%Y-%m-%d")`: the current UTC time.
fn now(format: Option<String>, kwargs: Kwargs) -> Result<String, Error> {
    let format = match format {
        Some(format) => Some(format),
        None => kwargs.get::<Option<String>>("format")?,
    };
    kwargs.assert_all_used()?;
    let format = format.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S");

    let mut out = String::new();
    write!(out, "{}", chrono::Utc::now().format(format)).map_err(|_| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("invalid time format `{format}`"),
        )
    })?;
    Ok(out)
}

fn describe(err: &Error) -> String {
    let mut message = format!("template error: {}", err.kind());
    if let Some(detail) = err.detail() {
        let _ = write!(message, ": {detail}");
    }
    if let Some(line) = err.line() {
        let _ = write!(message, " (line {line})");
    }
    message
}

/// Checks that a template compiles.
pub fn check_syntax(template: &str) -> Result<(), String> {
    ENV.template_from_str(template)
        .map(|_| ())
        .map_err(|err| describe(&err))
}

#[derive(Serialize, Clone, Debug)]
pub struct Named {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct AllocationContext {
    pub ip: String,
    pub ip_alias: Option<String>,
    pub port: i32,
    pub primary: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ServerContext {
    pub uuid: Uuid,
    pub uuid_short: String,
    pub name: String,
    pub description: Option<String>,
    pub memory: i64,
    pub swap: i64,
    pub disk: i64,
    pub cpu: i32,
    pub image: String,
    pub startup: String,
    pub ip: Option<String>,
    pub ip_alias: Option<String>,
    pub port: Option<i32>,
    pub allocations: Vec<AllocationContext>,
    pub egg: Named,
    pub nest: Named,
    pub node: Named,
    pub env: BTreeMap<String, String>,
    pub timezone: Option<String>,
}

impl ServerContext {
    pub async fn load(state: &State, server: &Server) -> Result<Self, anyhow::Error> {
        let node = server.node.fetch_cached(&state.database).await?;
        let allocations: Vec<AllocationContext> =
            ServerAllocation::all_by_server_uuid(&state.database, server.uuid)
                .await?
                .into_iter()
                .map(|allocation| AllocationContext {
                    ip: allocation.allocation.ip.ip().to_string(),
                    ip_alias: allocation.allocation.ip_alias.map(Into::into),
                    port: allocation.allocation.port,
                    primary: server
                        .allocation
                        .as_ref()
                        .is_some_and(|primary| primary.uuid == allocation.uuid),
                })
                .collect();
        let env = ServerVariable::all_by_server_uuid_egg_uuid(
            &state.database,
            server.uuid,
            server.egg.uuid,
        )
        .await?
        .into_iter()
        .filter(|variable| variable.variable.user_viewable)
        .map(|variable| (variable.variable.env_variable.to_string(), variable.value))
        .collect();
        let primary = allocations.iter().find(|allocation| allocation.primary);

        Ok(Self {
            uuid: server.uuid,
            uuid_short: format!("{:08x}", server.uuid_short),
            name: server.name.to_string(),
            description: server.description.as_ref().map(ToString::to_string),
            memory: server.memory,
            swap: server.swap,
            disk: server.disk,
            cpu: server.cpu,
            image: server.image.to_string(),
            startup: server.startup.to_string(),
            ip: primary.map(|allocation| allocation.ip.clone()),
            ip_alias: primary.and_then(|allocation| allocation.ip_alias.clone()),
            port: primary.map(|allocation| allocation.port),
            egg: Named {
                uuid: server.egg.uuid,
                name: server.egg.name.to_string(),
            },
            nest: Named {
                uuid: server.nest.uuid,
                name: server.nest.name.to_string(),
            },
            node: Named {
                uuid: node.uuid,
                name: node.name.to_string(),
            },
            env,
            timezone: server.timezone.as_ref().map(ToString::to_string),
            allocations,
        })
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct UserContext {
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct PanelContext {
    pub name: String,
    pub url: String,
}

impl PanelContext {
    pub async fn load(state: &State) -> Result<Self, anyhow::Error> {
        let settings = state.settings.get().await?;
        Ok(Self {
            name: settings.app.name.to_string(),
            url: settings.app.url.to_string(),
        })
    }
}

/// The values every template of one install is rendered with.
pub struct Renderer {
    context: minijinja::Value,
}

impl Renderer {
    /// `variables` are the checked user values, in template order.
    pub fn new(
        server: ServerContext,
        user: UserContext,
        panel: PanelContext,
        variables: &[(String, String)],
    ) -> Self {
        let mut context: BTreeMap<String, minijinja::Value> = variables
            .iter()
            .map(|(key, value)| (key.clone(), minijinja::Value::from(value.as_str())))
            .collect();
        let vars: BTreeMap<&str, &str> = variables
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        context.insert("vars".to_string(), minijinja::Value::from(Serde(vars)));
        context.insert("server".to_string(), minijinja::Value::from(Serde(server)));
        context.insert("user".to_string(), minijinja::Value::from(Serde(user)));
        context.insert("panel".to_string(), minijinja::Value::from(Serde(panel)));

        Self {
            context: minijinja::Value::from(context),
        }
    }

    pub fn render(&self, template: &str) -> Result<String, String> {
        ENV.render_str(template, &self.context)
            .map_err(|err| describe(&err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> Renderer {
        let allocation = AllocationContext {
            ip: "10.0.0.5".to_string(),
            ip_alias: Some("play.example.com".to_string()),
            port: 25565,
            primary: true,
        };
        Renderer::new(
            ServerContext {
                uuid: Uuid::nil(),
                uuid_short: "0000abcd".to_string(),
                name: "Survival".to_string(),
                description: None,
                memory: 4096,
                swap: 0,
                disk: 10240,
                cpu: 200,
                image: "ghcr.io/java:21".to_string(),
                startup: "java -jar server.jar".to_string(),
                ip: Some(allocation.ip.clone()),
                ip_alias: allocation.ip_alias.clone(),
                port: Some(allocation.port),
                allocations: vec![
                    allocation,
                    AllocationContext {
                        ip: "10.0.0.5".to_string(),
                        ip_alias: None,
                        port: 25566,
                        primary: false,
                    },
                ],
                egg: Named {
                    uuid: Uuid::nil(),
                    name: "Paper".to_string(),
                },
                nest: Named {
                    uuid: Uuid::nil(),
                    name: "Minecraft".to_string(),
                },
                node: Named {
                    uuid: Uuid::nil(),
                    name: "node-1".to_string(),
                },
                env: BTreeMap::from([("SERVER_JARFILE".to_string(), "server.jar".to_string())]),
                timezone: None,
            },
            UserContext {
                uuid: Uuid::nil(),
                username: "alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            PanelContext {
                name: "Calagopus".to_string(),
                url: "https://panel.example.com".to_string(),
            },
            &[
                ("MOTD".to_string(), "Hello <world> & \"friends\"".to_string()),
                ("PVP".to_string(), "1".to_string()),
            ],
        )
    }

    #[test]
    fn renders_context_without_escaping() {
        let r = renderer();
        assert_eq!(
            r.render("motd={{ MOTD }}\npvp={{ vars.PVP }}\n").unwrap(),
            "motd=Hello <world> & \"friends\"\npvp=1\n"
        );
        assert_eq!(
            r.render("{{ server.ip }}:{{ server.port }} {{ server.ip_alias }} {{ server.uuid_short }}")
                .unwrap(),
            "10.0.0.5:25565 play.example.com 0000abcd"
        );
        assert_eq!(
            r.render("{% for a in server.allocations if not a.primary %}{{ a.port }}{% endfor %}")
                .unwrap(),
            "25566"
        );
        assert_eq!(
            r.render("{{ server.env.SERVER_JARFILE }} {{ server.memory * 2 }} {{ user.username }} {{ panel.url }} {{ server.egg.name }}/{{ server.nest.name }}@{{ server.node.name }}")
                .unwrap(),
            "server.jar 8192 alice https://panel.example.com Paper/Minecraft@node-1"
        );
        // booleans render Python-style, which conditions accept either way
        assert_eq!(r.render("{{ server.timezone is none }}").unwrap(), "True");
        assert!(!crate::model::is_truthy(&r.render("{{ vars.PVP == '0' }}").unwrap()));
    }

    #[test]
    fn unknown_variables_fail() {
        let r = renderer();
        let err = r.render("{{ MOTDD }}").unwrap_err();
        assert!(err.starts_with("template error: undefined value"), "{err}");
        assert!(r.render("{{ server.nope }}").is_err());
    }

    #[test]
    fn functions() {
        let r = renderer();
        let value = r.render("{{ random_string() }}").unwrap();
        assert_eq!(value.len(), 16);
        assert!(value.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(r.render("{{ random_string(length=40) }}").unwrap().len(), 40);
        assert_eq!(r.render("{{ random_string(5) }}").unwrap().len(), 5);
        assert!(r.render("{{ random_string(100000) }}").is_err());
        assert!(Uuid::parse_str(&r.render("{{ uuid() }}").unwrap()).is_ok());
        assert_eq!(r.render("{{ now(format='%Y') }}").unwrap().len(), 4);
        assert!(
            chrono::NaiveDateTime::parse_from_str(&r.render("{{ now() }}").unwrap(), "%Y-%m-%d %H:%M:%S")
                .is_ok()
        );
    }

    #[test]
    fn syntax_check() {
        assert!(check_syntax("{{ anything.at.all }}").is_ok());
        assert!(check_syntax("{% if x %}").unwrap_err().starts_with("template error"));
    }
}
