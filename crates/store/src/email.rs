//! Email settings, templates and the log of emails sent.

use booker_core::{
    BookingId, EmailKind, EmailLogEntry, EmailSettings, EmailTemplate, validate_email_settings,
};
use chrono::NaiveDateTime;

use crate::audit::{Change, json, record};
use crate::bookings::parse;
use crate::repo::EmailRepo;
use crate::rows::{int, uuid};
use crate::{SqliteStore, StoreError, StoreResult};

const STAMP: &str = "%Y-%m-%d %H:%M:%S";

/// A row of `email_log`, in column order.
type LogRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    bool,
    String,
    String,
);

impl EmailRepo for SqliteStore {
    async fn get_email_settings(&self) -> StoreResult<EmailSettings> {
        let row: Option<(String, i64, String, String, String)> = sqlx::query_as(
            "SELECT host, port, username, from_address, from_name FROM email_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some((host, port, username, from_address, from_name)) = row else {
            return Ok(EmailSettings::default());
        };
        Ok(EmailSettings {
            host,
            port: int(port, "email_settings.port")?,
            username,
            from_address,
            from_name,
        })
    }

    async fn save_email_settings(&self, settings: &EmailSettings) -> StoreResult<()> {
        validate_email_settings(settings)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO email_settings (id, host, port, username, from_address, from_name)
             VALUES (1, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET host = excluded.host, port = excluded.port,
                username = excluded.username, from_address = excluded.from_address,
                from_name = excluded.from_name,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(settings.host.trim())
        .bind(i64::from(settings.port))
        .bind(settings.username.trim())
        .bind(settings.from_address.trim())
        .bind(settings.from_name.trim())
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "email_settings",
                entity_id: "1".into(),
                action: "updated",
                summary: "Changed the email settings".into(),
                before: None,
                after: json(settings),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_email_templates(&self) -> StoreResult<Vec<EmailTemplate>> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT kind, subject, body FROM email_templates")
                .fetch_all(&self.pool)
                .await?;
        EmailKind::ALL
            .into_iter()
            .map(
                |kind| match rows.iter().find(|(k, ..)| k == kind.as_str()) {
                    Some((k, subject, body)) => Ok(EmailTemplate {
                        kind: parse(k)?,
                        subject: subject.clone(),
                        body: body.clone(),
                    }),
                    None => Ok(EmailTemplate::default_for(kind)),
                },
            )
            .collect()
    }

    async fn save_email_template(&self, template: &EmailTemplate) -> StoreResult<()> {
        if template.subject.trim().is_empty() {
            return Err(booker_core::ValidationError::Empty { field: "Subject" }.into());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO email_templates (id, kind, subject, body) VALUES (?, ?, ?, ?)
             ON CONFLICT (kind) DO UPDATE SET subject = excluded.subject, body = excluded.body,
                updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(template.kind.as_str())
        .bind(template.subject.trim())
        .bind(template.body.trim())
        .execute(&mut *tx)
        .await?;
        record(
            &mut tx,
            Change {
                entity: "email_template",
                entity_id: template.kind.as_str().into(),
                action: "updated",
                summary: format!("Changed the {} email", template.kind.as_str()),
                before: None,
                after: json(template),
                actor: self.actor.as_ref(),
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn log_email(&self, entry: &EmailLogEntry) -> StoreResult<()> {
        sqlx::query(
            "INSERT INTO email_log (id, at, recipient, subject, kind, booking_id, sent, error,
                                    sent_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(entry.at.format(STAMP).to_string())
        .bind(entry.to.trim())
        .bind(&entry.subject)
        .bind(entry.kind.map(EmailKind::as_str))
        .bind(entry.booking_id.map(|b| b.to_string()))
        .bind(entry.sent)
        .bind(&entry.error)
        .bind(&entry.sent_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_email_log(&self, booking: Option<BookingId>) -> StoreResult<Vec<EmailLogEntry>> {
        let rows: Vec<LogRow> = match booking {
            Some(b) => {
                sqlx::query_as(
                    "SELECT at, recipient, subject, kind, booking_id, sent, error, sent_by
                     FROM email_log WHERE booking_id = ? ORDER BY at DESC, id DESC",
                )
                .bind(b.to_string())
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT at, recipient, subject, kind, booking_id, sent, error, sent_by
                     FROM email_log ORDER BY at DESC, id DESC LIMIT 200",
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.into_iter()
            .map(
                |(at, to, subject, kind, booking_id, sent, error, sent_by)| {
                    Ok(EmailLogEntry {
                        at: NaiveDateTime::parse_from_str(&at, STAMP)
                            .map_err(|e| StoreError::Corrupt(format!("bad time {at:?}: {e}")))?,
                        to,
                        subject,
                        kind: kind.as_deref().map(parse).transpose()?,
                        booking_id: booking_id.as_deref().map(uuid).transpose()?.map(BookingId),
                        sent,
                        error,
                        sent_by,
                    })
                },
            )
            .collect()
    }
}
