use crate::{
    model::{EXPORT_FORMAT, EXPORT_VERSION, ExportDocument, ExportTemplate, Step, Variable},
    validate::TemplateData,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use shared::database::Database;
use sqlx::types::Json;
use utoipa::ToSchema;
use uuid::Uuid;

const TABLE: &str = "dev_caloptreyx_templates_templates";

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct Template {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub egg_uuids: Vec<Uuid>,
    pub variables: Json<Vec<Variable>>,
    pub steps: Json<Vec<Step>>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(ToSchema, Serialize)]
pub struct ApiTemplate {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub egg_uuids: Vec<Uuid>,
    pub variables: Vec<Variable>,
    pub steps: Vec<Step>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(ToSchema, Serialize)]
pub struct ApiClientStep {
    pub r#type: &'static str,
    pub name: Option<String>,
}

/// What users see of a template: no step contents.
#[derive(ToSchema, Serialize)]
pub struct ApiClientTemplate {
    pub uuid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub variables: Vec<Variable>,
    pub steps: Vec<ApiClientStep>,
}

#[derive(ToSchema, Serialize)]
pub struct ApiNamed {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(ToSchema, Serialize)]
pub struct ApiEgg {
    pub uuid: Uuid,
    pub name: String,
    pub nest: ApiNamed,
}

impl ApiEgg {
    /// Every egg of every nest, sorted by nest and egg name.
    pub async fn all(db: &Database) -> Result<Vec<Self>, sqlx::Error> {
        let rows: Vec<(Uuid, String, Uuid, String)> = sqlx::query_as(
            "SELECT e.uuid, e.name, n.uuid, n.name
             FROM nest_eggs e
             JOIN nests n ON n.uuid = e.nest_uuid
             ORDER BY lower(n.name), n.uuid, lower(e.name), e.uuid",
        )
        .fetch_all(db.read())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(uuid, name, nest_uuid, nest_name)| Self {
                uuid,
                name,
                nest: ApiNamed {
                    uuid: nest_uuid,
                    name: nest_name,
                },
            })
            .collect())
    }
}

impl Template {
    pub async fn by_uuid(db: &Database, uuid: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM {TABLE} WHERE uuid = $1"
        )))
        .bind(uuid)
        .fetch_optional(db.read())
        .await
    }

    /// The template if it is enabled and offered for `egg`.
    pub async fn available_by_uuid(
        db: &Database,
        egg: Uuid,
        uuid: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM {TABLE}
             WHERE uuid = $2 AND enabled AND (cardinality(egg_uuids) = 0 OR $1 = ANY(egg_uuids))"
        )))
        .bind(egg)
        .bind(uuid)
        .fetch_optional(db.read())
        .await
    }

    /// Enabled templates offered for `egg`, by name.
    pub async fn all_available(db: &Database, egg: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT * FROM {TABLE}
             WHERE enabled AND (cardinality(egg_uuids) = 0 OR $1 = ANY(egg_uuids))
             ORDER BY lower(name), created"
        )))
        .bind(egg)
        .fetch_all(db.read())
        .await
    }

    pub async fn all_with_pagination(
        db: &Database,
        page: i64,
        per_page: i64,
        search: Option<&str>,
    ) -> Result<(i64, Vec<Self>), sqlx::Error> {
        let offset = (page - 1) * per_page;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT *, COUNT(*) OVER() AS total_count FROM {TABLE}
             WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%'
                    OR description ILIKE '%' || $1 || '%')
             ORDER BY lower(name), created
             LIMIT $2 OFFSET $3"
        )))
        .bind(search)
        .bind(per_page)
        .bind(offset)
        .fetch_all(db.read())
        .await?;

        use sqlx::{FromRow, Row};
        let total = rows
            .first()
            .map(|row| row.try_get::<i64, _>("total_count"))
            .transpose()?
            .unwrap_or(0);
        let data = rows
            .iter()
            .map(Self::from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((total, data))
    }

    pub async fn create(db: &Database, data: TemplateData) -> Result<Self, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {TABLE} (name, description, enabled, egg_uuids, variables, steps)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *"
        )))
        .bind(data.name)
        .bind(data.description)
        .bind(data.enabled)
        .bind(data.egg_uuids)
        .bind(Json(data.variables))
        .bind(Json(data.steps))
        .fetch_one(db.write())
        .await
    }

    /// Replaces everything but the uuid and creation time.
    pub async fn update(
        db: &Database,
        uuid: Uuid,
        data: TemplateData,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "UPDATE {TABLE} SET
                name = $2, description = $3, enabled = $4, egg_uuids = $5,
                variables = $6, steps = $7, updated = now()
             WHERE uuid = $1
             RETURNING *"
        )))
        .bind(uuid)
        .bind(data.name)
        .bind(data.description)
        .bind(data.enabled)
        .bind(data.egg_uuids)
        .bind(Json(data.variables))
        .bind(Json(data.steps))
        .fetch_optional(db.write())
        .await
    }

    pub async fn delete(db: &Database, uuid: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {TABLE} WHERE uuid = $1"
        )))
        .bind(uuid)
        .execute(db.write())
        .await?;
        Ok(())
    }

    pub fn into_api(self) -> ApiTemplate {
        ApiTemplate {
            uuid: self.uuid,
            name: self.name,
            description: self.description,
            enabled: self.enabled,
            egg_uuids: self.egg_uuids,
            variables: self.variables.0,
            steps: self.steps.0,
            created: self.created,
            updated: self.updated,
        }
    }

    pub fn into_client_api(self) -> ApiClientTemplate {
        ApiClientTemplate {
            uuid: self.uuid,
            name: self.name,
            description: self.description,
            variables: self.variables.0,
            steps: self
                .steps
                .0
                .into_iter()
                .map(|step| ApiClientStep {
                    r#type: step.kind.type_name(),
                    name: step.name,
                })
                .collect(),
        }
    }

    pub fn into_export(self) -> ExportDocument {
        ExportDocument {
            format: EXPORT_FORMAT.to_string(),
            version: EXPORT_VERSION,
            exported_at: Some(Utc::now()),
            template: ExportTemplate {
                name: self.name,
                description: self.description,
                variables: self.variables.0,
                steps: self.steps.0,
            },
        }
    }
}
