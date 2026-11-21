//! Staff accounts. PINs arrive here already hashed (see `booker-services`).

use booker_core::{Role, User, UserId, validate_user};
use sqlx::SqliteConnection;

use crate::audit::{Change, json, record};
use crate::rows::uuid;
use crate::{SqliteStore, StoreError, StoreResult};

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    name: String,
    role: String,
    active: bool,
}

impl TryFrom<UserRow> for User {
    type Error = StoreError;
    fn try_from(r: UserRow) -> StoreResult<Self> {
        Ok(User {
            id: UserId(uuid(&r.id)?),
            name: r.name,
            role: r.role.parse().map_err(StoreError::Corrupt)?,
            active: r.active,
        })
    }
}

async fn load_users(conn: &mut SqliteConnection) -> StoreResult<Vec<User>> {
    let rows: Vec<UserRow> = sqlx::query_as(
        "SELECT id, name, role, active FROM users ORDER BY active DESC, lower(name)",
    )
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(User::try_from).collect()
}

impl SqliteStore {
    pub async fn list_users(&self) -> StoreResult<Vec<User>> {
        let mut conn = self.pool.acquire().await?;
        load_users(&mut conn).await
    }

    /// The stored PIN hash for an active user, if there is one.
    pub async fn pin_hash(&self, id: UserId) -> StoreResult<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT pin_hash FROM users WHERE id = ? AND active = 1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Creates or updates a user. A new user needs `pin_hash`; for an existing
    /// one, `None` keeps the current PIN.
    pub async fn save_user(&self, user: &User, pin_hash: Option<&str>) -> StoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let all = load_users(&mut tx).await?;
        validate_user(user, &all)?;
        let before = all.iter().find(|u| u.id == user.id).cloned();
        match (&before, pin_hash) {
            (None, None) => {
                return Err(StoreError::NotFound {
                    what: "PIN for the new user",
                });
            }
            (None, Some(hash)) => {
                sqlx::query(
                    "INSERT INTO users (id, name, role, active, pin_hash) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(user.id.to_string())
                .bind(user.name.trim())
                .bind(user.role.as_str())
                .bind(user.active)
                .bind(hash)
                .execute(&mut *tx)
                .await?;
            }
            (Some(_), hash) => {
                sqlx::query(
                    "UPDATE users SET name = ?, role = ?, active = ?,
                        pin_hash = coalesce(?, pin_hash),
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), updated_by = ?
                     WHERE id = ?",
                )
                .bind(user.name.trim())
                .bind(user.role.as_str())
                .bind(user.active)
                .bind(hash)
                .bind(self.actor.as_ref().map(|a| a.name.clone()))
                .bind(user.id.to_string())
                .execute(&mut *tx)
                .await?;
            }
        }
        let role = if user.role == Role::Admin {
            "admin"
        } else {
            "staff member"
        };
        let summary = match (&before, pin_hash) {
            (None, _) => format!("Added {role} {}", user.name.trim()),
            (Some(_), Some(_)) => format!("Changed details and PIN for {}", user.name.trim()),
            (Some(_), None) => format!("Changed details for {}", user.name.trim()),
        };
        record(
            &mut tx,
            Change {
                entity: "user",
                entity_id: user.id.to_string(),
                action: if before.is_some() {
                    "updated"
                } else {
                    "created"
                },
                summary,
                before: before.as_ref().and_then(json),
                after: json(user),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
