use super::text_enum;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::database::Database;
use sqlx::types::Json;
use utoipa::ToSchema;
use uuid::Uuid;

text_enum!(InstallStatus {
    Running => "running",
    Success => "success",
    Failed => "failed",
});

const TABLE: &str = "dev_caloptreyx_templates_installs";
const RUNNING_INDEX: &str = "dev_caloptreyx_templates_installs_running_idx";
/// Installs kept per server.
const KEEP_PER_SERVER: i64 = 50;
/// A running install without progress for this long was interrupted.
const STALE_AFTER: &str = "interval '15 minutes'";
pub const INTERRUPTED: &str = "interrupted";

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Skipped,
    Failed,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StepProgress {
    pub index: usize,
    pub r#type: String,
    pub name: Option<String>,
    pub status: StepStatus,
    pub message: Option<String>,
}

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct Install {
    pub uuid: Uuid,
    pub template_uuid: Option<Uuid>,
    pub template_name: String,
    pub user_uuid: Option<Uuid>,
    pub status: InstallStatus,
    pub steps: Json<Vec<StepProgress>>,
    pub error: Option<String>,
    pub created: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub username: Option<String>,
}

#[derive(ToSchema, Serialize)]
pub struct ApiInstallUser {
    pub uuid: Uuid,
    pub username: String,
}

#[derive(ToSchema, Serialize)]
pub struct ApiInstall {
    pub uuid: Uuid,
    pub template_uuid: Option<Uuid>,
    pub template_name: String,
    pub user: Option<ApiInstallUser>,
    pub status: InstallStatus,
    pub steps: Vec<StepProgress>,
    pub error: Option<String>,
    pub created: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
}

const SELECT: &str = "SELECT i.*, u.username
    FROM dev_caloptreyx_templates_installs i
    LEFT JOIN users u ON u.uuid = i.user_uuid";

impl Install {
    /// Marks running installs of `server` (all servers for `None`) that made
    /// no progress for a while as failed.
    pub async fn fail_stale(db: &Database, server: Option<Uuid>) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {TABLE} SET
                status = 'failed',
                error = '{INTERRUPTED}',
                finished = now(),
                updated = now(),
                steps = COALESCE((
                    SELECT jsonb_agg(
                        CASE WHEN step->>'status' = 'running'
                            THEN jsonb_set(step, '{{status}}', '\"failed\"')
                                 || jsonb_build_object('message', '{INTERRUPTED}')
                            ELSE step END
                        ORDER BY position)
                    FROM jsonb_array_elements(steps) WITH ORDINALITY AS t(step, position)
                ), '[]'::jsonb)
             WHERE status = 'running' AND updated < now() - {STALE_AFTER}
                AND ($1::uuid IS NULL OR server_uuid = $1)"
        )))
        .bind(server)
        .execute(db.write())
        .await?
        .rows_affected())
    }

    /// Inserts a running install; `None` if the server already runs one.
    /// Only the newest installs of the server are kept.
    pub async fn start(
        db: &Database,
        server: Uuid,
        template: Uuid,
        template_name: &str,
        user: Uuid,
        steps: &[StepProgress],
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let inserted = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {TABLE} (server_uuid, template_uuid, template_name, user_uuid, status, steps)
             VALUES ($1, $2, $3, $4, 'running', $5)
             RETURNING uuid"
        )))
        .bind(server)
        .bind(template)
        .bind(template_name)
        .bind(user)
        .bind(Json(steps))
        .fetch_one(db.write())
        .await;
        let uuid: Uuid = match inserted {
            Ok(uuid) => uuid,
            Err(sqlx::Error::Database(err)) if err.constraint() == Some(RUNNING_INDEX) => {
                return Ok(None);
            }
            Err(err) => return Err(err),
        };

        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {TABLE} WHERE server_uuid = $1 AND status <> 'running' AND uuid NOT IN (
                SELECT uuid FROM {TABLE} WHERE server_uuid = $1 ORDER BY created DESC LIMIT $2
             )"
        )))
        .bind(server)
        .bind(KEEP_PER_SERVER)
        .execute(db.write())
        .await?;

        Ok(Some(uuid))
    }

    pub async fn set_progress(
        db: &Database,
        uuid: Uuid,
        steps: &[StepProgress],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {TABLE} SET steps = $2, updated = now() WHERE uuid = $1"
        )))
        .bind(uuid)
        .bind(Json(steps))
        .execute(db.write())
        .await?;
        Ok(())
    }

    /// Keeps a long step from looking interrupted.
    pub async fn touch(db: &Database, uuid: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {TABLE} SET updated = now() WHERE uuid = $1 AND status = 'running'"
        )))
        .bind(uuid)
        .execute(db.write())
        .await?;
        Ok(())
    }

    pub async fn finish(
        db: &Database,
        uuid: Uuid,
        status: InstallStatus,
        steps: &[StepProgress],
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {TABLE} SET status = $2, steps = $3, error = $4, updated = now(), finished = now()
             WHERE uuid = $1"
        )))
        .bind(uuid)
        .bind(status)
        .bind(Json(steps))
        .bind(error)
        .execute(db.write())
        .await?;
        Ok(())
    }

    pub async fn by_server_uuid_uuid(
        db: &Database,
        server: Uuid,
        uuid: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} WHERE i.server_uuid = $1 AND i.uuid = $2"
        )))
        .bind(server)
        .bind(uuid)
        .fetch_optional(db.read())
        .await
    }

    /// Newest first.
    pub async fn recent_by_server_uuid(
        db: &Database,
        server: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} WHERE i.server_uuid = $1 ORDER BY i.created DESC LIMIT $2"
        )))
        .bind(server)
        .bind(limit)
        .fetch_all(db.read())
        .await
    }

    pub fn into_api(self) -> ApiInstall {
        ApiInstall {
            uuid: self.uuid,
            template_uuid: self.template_uuid,
            template_name: self.template_name,
            user: self.user_uuid.map(|uuid| ApiInstallUser {
                uuid,
                username: self.username.unwrap_or_default(),
            }),
            status: self.status,
            steps: self.steps.0,
            error: self.error,
            created: self.created,
            finished: self.finished,
        }
    }
}
