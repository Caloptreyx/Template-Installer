use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod list {
    use crate::db::{ApiTemplate, Template};
    use axum::extract::Query;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{Pagination, PaginationParamsWithSearch, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(inline)]
        templates: Pagination<ApiTemplate>,
    }

    #[utoipa::path(get, path = "/templates", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        ("page" = i64, Query, description = "The page number", example = "1"),
        ("per_page" = i64, Query, description = "The number of items per page", example = "10"),
        ("search" = Option<String>, Query, description = "Search by template name or description"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Query(pagination): Query<PaginationParamsWithSearch>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&pagination) {
            return response.ok();
        }
        permissions.has_admin_permission("templates.read")?;

        let (total, rows) = Template::all_with_pagination(
            &state.database,
            pagination.page,
            pagination.per_page,
            pagination.search.as_deref(),
        )
        .await?;

        ApiResponse::new_serialized(Response {
            templates: Pagination {
                total,
                per_page: pagination.per_page,
                page: pagination.page,
                data: rows.into_iter().map(Template::into_api).collect(),
            },
        })
        .ok()
    }
}

mod create {
    use crate::{
        db::{ApiTemplate, Template},
        validate::TemplatePayload,
    };
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        template: ApiTemplate,
    }

    #[utoipa::path(post, path = "/templates", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), request_body = TemplatePayload)]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<TemplatePayload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_admin_permission("templates.manage")?;

        let data = match data.into_data() {
            Ok(data) => data,
            Err(errors) => return crate::routes::bad_request(errors).ok(),
        };
        let template = Template::create(&state.database, data).await?;

        activity_logger
            .log(
                "templates:template.create",
                serde_json::json!({ "uuid": template.uuid, "name": template.name }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            template: template.into_api(),
        })
        .ok()
    }
}

mod import {
    use crate::{
        db::{ApiTemplate, Template},
        model::{EXPORT_FORMAT, EXPORT_VERSION, ExportDocument},
        validate::TemplatePayload,
    };
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        template: ApiTemplate,
    }

    /// Creates a disabled template for all eggs from an export document.
    #[utoipa::path(post, path = "/templates/import", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), request_body = ExportDocument)]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(document): shared::Payload<ExportDocument>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("templates.manage")?;

        if document.format != EXPORT_FORMAT || document.version != EXPORT_VERSION {
            return crate::routes::bad_request(vec![format!(
                "unsupported export document `{}` version {} (expected `{EXPORT_FORMAT}` version {EXPORT_VERSION})",
                document.format, document.version
            )])
            .ok();
        }
        let payload = TemplatePayload {
            name: document.template.name,
            description: document.template.description,
            enabled: false,
            egg_uuids: Vec::new(),
            variables: document.template.variables,
            steps: document.template.steps,
        };
        if let Some(response) = crate::routes::validation_error(&payload) {
            return response.ok();
        }
        let data = match payload.into_data() {
            Ok(data) => data,
            Err(errors) => return crate::routes::bad_request(errors).ok(),
        };
        let template = Template::create(&state.database, data).await?;

        activity_logger
            .log(
                "templates:template.import",
                serde_json::json!({ "uuid": template.uuid, "name": template.name }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            template: template.into_api(),
        })
        .ok()
    }
}

mod get {
    use crate::db::{ApiTemplate, Template};
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        template: ApiTemplate,
    }

    #[utoipa::path(get, path = "/templates/{template}", responses(
        (status = OK, body = inline(Response)),
        (status = NOT_FOUND, body = ApiError),
    ), params(("template" = uuid::Uuid, description = "The template ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path(template): Path<uuid::Uuid>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("templates.read")?;

        let template = Template::by_uuid(&state.database, template)
            .await?
            .ok_or_else(|| crate::routes::not_found("template"))?;

        ApiResponse::new_serialized(Response {
            template: template.into_api(),
        })
        .ok()
    }
}

mod update {
    use crate::{
        db::{ApiTemplate, Template},
        validate::TemplatePayload,
    };
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        template: ApiTemplate,
    }

    /// Replaces the whole template.
    #[utoipa::path(patch, path = "/templates/{template}", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
    ), params(("template" = uuid::Uuid, description = "The template ID")), request_body = TemplatePayload)]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        Path(template): Path<uuid::Uuid>,
        shared::Payload(data): shared::Payload<TemplatePayload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_admin_permission("templates.manage")?;

        let data = match data.into_data() {
            Ok(data) => data,
            Err(errors) => return crate::routes::bad_request(errors).ok(),
        };
        let template = Template::update(&state.database, template, data)
            .await?
            .ok_or_else(|| crate::routes::not_found("template"))?;

        activity_logger
            .log(
                "templates:template.update",
                serde_json::json!({
                    "uuid": template.uuid,
                    "name": template.name,
                    "enabled": template.enabled,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            template: template.into_api(),
        })
        .ok()
    }
}

mod delete {
    use crate::db::Template;
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/templates/{template}", responses(
        (status = OK, body = inline(Response)),
        (status = NOT_FOUND, body = ApiError),
    ), params(("template" = uuid::Uuid, description = "The template ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        Path(template): Path<uuid::Uuid>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("templates.manage")?;

        let template = Template::by_uuid(&state.database, template)
            .await?
            .ok_or_else(|| crate::routes::not_found("template"))?;
        Template::delete(&state.database, template.uuid).await?;

        activity_logger
            .log(
                "templates:template.delete",
                serde_json::json!({ "uuid": template.uuid, "name": template.name }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

mod export {
    use crate::{db::Template, model::ExportDocument};
    use axum::extract::Path;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };

    #[utoipa::path(get, path = "/templates/{template}/export", responses(
        (status = OK, body = ExportDocument),
        (status = NOT_FOUND, body = ApiError),
    ), params(("template" = uuid::Uuid, description = "The template ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Path(template): Path<uuid::Uuid>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("templates.read")?;

        let template = Template::by_uuid(&state.database, template)
            .await?
            .ok_or_else(|| crate::routes::not_found("template"))?;

        ApiResponse::new_serialized(template.into_export()).ok()
    }
}

mod eggs {
    use crate::db::ApiEgg;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        eggs: Vec<ApiEgg>,
    }

    #[utoipa::path(get, path = "/eggs", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("templates.read")?;

        ApiResponse::new_serialized(Response {
            eggs: ApiEgg::all(&state.database).await?,
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(list::route))
        .routes(routes!(create::route))
        .routes(routes!(import::route))
        .routes(routes!(get::route))
        .routes(routes!(update::route))
        .routes(routes!(delete::route))
        .routes(routes!(export::route))
        .routes(routes!(eggs::route))
        .with_state(state.clone())
}
