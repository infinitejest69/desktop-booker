//! Writing and reading the audit log.

use booker_core::{AuditEntry, AuditId, UserId};
use serde::Serialize;
use sqlx::SqliteConnection;

use crate::rows::uuid;
use crate::sqlite::Actor;
use crate::{SqliteStore, StoreError, StoreResult};

/// The record as JSON text for `before`/`after`. A record that can't be
/// serialised is logged without its data rather than failing the change.
pub(crate) fn json<T: Serialize>(value: &T) -> Option<String> {
    serde_json::to_string(value).ok()
}

/// What one change was, for [`record`].
pub(crate) struct Change<'a> {
    pub entity: &'static str,
    pub entity_id: String,
    pub action: &'static str,
    pub summary: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub actor: Option<&'a Actor>,
}

/// Appends one entry, inside the caller's transaction, so the change and its
/// log entry are saved together or not at all.
pub(crate) async fn record(conn: &mut SqliteConnection, change: Change<'_>) -> StoreResult<()> {
    let (user_id, user_name) = match change.actor {
        Some(a) => (a.user_id.map(|id| id.to_string()), a.name.clone()),
        None => (None, "System".to_string()),
    };
    sqlx::query(
        "INSERT INTO audit_log (id, user_id, user_name, entity, entity_id, action, summary,
                                before, after)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(AuditId::new().to_string())
    .bind(user_id)
    .bind(user_name)
    .bind(change.entity)
    .bind(change.entity_id)
    .bind(change.action)
    .bind(change.summary)
    .bind(change.before)
    .bind(change.after)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    at: String,
    user_id: Option<String>,
    user_name: String,
    entity: String,
    entity_id: String,
    action: String,
    summary: String,
    before: Option<String>,
    after: Option<String>,
}

impl SqliteStore {
    /// Newest first, optionally only one user's or one kind of record's.
    pub async fn list_audit(
        &self,
        user: Option<UserId>,
        entity: Option<&str>,
        limit: u32,
    ) -> StoreResult<Vec<AuditEntry>> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT id, at, user_id, user_name, entity, entity_id, action, summary, before, after
             FROM audit_log
             WHERE (?1 IS NULL OR user_id = ?1) AND (?2 IS NULL OR entity = ?2)
             ORDER BY at DESC, id DESC LIMIT ?3",
        )
        .bind(user.map(|u| u.to_string()))
        .bind(entity)
        .bind(i64::from(limit.clamp(1, 1000)))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|r| {
                Ok(AuditEntry {
                    id: AuditId(uuid(&r.id)?),
                    at: r.at,
                    user_id: r.user_id.as_deref().map(uuid).transpose()?.map(UserId),
                    user_name: r.user_name,
                    entity: r.entity,
                    entity_id: r.entity_id,
                    action: r.action,
                    summary: r.summary,
                    before: r.before,
                    after: r.after,
                })
            })
            .collect::<Result<_, StoreError>>()
    }
}
