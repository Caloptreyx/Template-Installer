use indexmap::IndexMap;
use shared::{
    State,
    extensions::{Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder},
    permissions::PermissionGroup,
};

mod db;
mod executor;
mod installer;
mod model;
mod render;
mod routes;
mod validate;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize(&mut self, state: State) {
        // installs that were running when the panel stopped never finish
        tokio::spawn(async move {
            match db::Install::fail_stale(&state.database, None).await {
                Ok(0) => {}
                Ok(count) => tracing::info!("marked {count} interrupted template installs as failed"),
                Err(err) => tracing::warn!("failed to mark interrupted template installs: {err}"),
            }
        });
    }

    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| {
                router.nest(
                    "/extensions/dev.caloptreyx.templates",
                    routes::admin::router(&state),
                )
            })
            .add_client_server_api_router(|router| {
                router.nest("/templates", routes::server::router(&state))
            })
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        mut builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder.server_permissions.insert(
            "templates",
            PermissionGroup {
                description: "Permissions that control the ability to install templates on this server.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the templates available for the server and its install history.",
                    ),
                    (
                        "install",
                        "Allows installing templates, which change the server's files. Templates with power steps also require the matching control permission.",
                    ),
                ]),
            },
        );

        builder.admin_permissions.insert(
            "templates",
            PermissionGroup {
                description: "Permissions that control the ability to manage the template installer.",
                permissions: IndexMap::from([
                    ("read", "Allows viewing and exporting templates."),
                    (
                        "manage",
                        "Allows creating, changing, importing and deleting templates.",
                    ),
                ]),
            },
        );

        builder
    }
}
