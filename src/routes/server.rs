use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Installs listed per server.
const RECENT_INSTALLS: i64 = 20;

mod list {
    use crate::db::{ApiClientTemplate, Template};
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        templates: Vec<ApiClientTemplate>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("templates.read")?;

        let templates = Template::all_available(&state.database, server.egg.uuid).await?;

        ApiResponse::new_serialized(Response {
            templates: templates
                .into_iter()
                .map(Template::into_client_api)
                .collect(),
        })
        .ok()
    }
}

mod install {
    use crate::{
        db::{ApiInstall, Install, Template},
        installer::{self, Outcome, SubmittedVariable},
    };
    use axum::{extract::Path, http::StatusCode};
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        #[serde(default)]
        variables: Vec<SubmittedVariable>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        install: ApiInstall,
    }

    /// Starts installing the template; the steps run in the background.
    #[utoipa::path(post, path = "/{template}/install", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("template" = uuid::Uuid, description = "The template ID"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, template)): Path<(String, uuid::Uuid)>,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("templates.install")?;

        let template = Template::available_by_uuid(&state.database, server.egg.uuid, template)
            .await?
            .ok_or_else(|| crate::routes::not_found("template"))?;
        for permission in installer::required_permissions(&template.steps) {
            permissions.has_server_permission(permission)?;
        }

        let server_uuid = server.uuid;
        let log = serde_json::json!({ "template_uuid": template.uuid, "template_name": template.name });
        let install =
            match installer::start(&state, server.0, &user, template, data.variables).await? {
                Outcome::Started(install) => install,
                Outcome::Invalid(errors) => return crate::routes::bad_request(errors).ok(),
                Outcome::Busy => {
                    return ApiResponse::error("another template install is running on this server")
                        .with_status(StatusCode::CONFLICT)
                        .ok();
                }
            };

        let mut log = log;
        log["install_uuid"] = serde_json::json!(install);
        activity_logger.log("server:templates.install", log).await;

        let install = Install::by_server_uuid_uuid(&state.database, server_uuid, install)
            .await?
            .ok_or_else(|| crate::routes::not_found("install"))?;
        ApiResponse::new_serialized(Response {
            install: install.into_api(),
        })
        .ok()
    }
}

mod installs {
    use crate::db::{ApiInstall, Install};
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        installs: Vec<ApiInstall>,
    }

    /// The newest installs of the server.
    #[utoipa::path(get, path = "/installs", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("templates.read")?;

        Install::fail_stale(&state.database, Some(server.uuid)).await?;
        let installs =
            Install::recent_by_server_uuid(&state.database, server.uuid, super::RECENT_INSTALLS)
                .await?;

        ApiResponse::new_serialized(Response {
            installs: installs.into_iter().map(Install::into_api).collect(),
        })
        .ok()
    }
}

mod install_get {
    use crate::db::{ApiInstall, Install};
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        install: ApiInstall,
    }

    #[utoipa::path(get, path = "/installs/{install}", responses(
        (status = OK, body = inline(Response)),
        (status = NOT_FOUND, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("install" = uuid::Uuid, description = "The install ID"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        Path((_server, install)): Path<(String, uuid::Uuid)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("templates.read")?;

        Install::fail_stale(&state.database, Some(server.uuid)).await?;
        let install = Install::by_server_uuid_uuid(&state.database, server.uuid, install)
            .await?
            .ok_or_else(|| crate::routes::not_found("install"))?;

        ApiResponse::new_serialized(Response {
            install: install.into_api(),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(list::route))
        .routes(routes!(install::route))
        .routes(routes!(installs::route))
        .routes(routes!(install_get::route))
        .with_state(state.clone())
}
