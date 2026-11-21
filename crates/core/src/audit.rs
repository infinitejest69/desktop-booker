//! The audit log: who changed what, and when.

use serde::{Deserialize, Serialize};

use crate::ids::{AuditId, UserId};

/// One recorded change. `before` and `after` hold the record as JSON text,
/// so the log keeps working when the shapes of records change later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct AuditEntry {
    pub id: AuditId,
    /// ISO-8601 UTC timestamp.
    pub at: String,
    pub user_id: Option<UserId>,
    pub user_name: String,
    /// What kind of record changed, e.g. "booking", "rate_rule".
    pub entity: String,
    pub entity_id: String,
    /// What happened, e.g. "created", "updated", "deleted", "checked_in".
    pub action: String,
    /// A sentence for people, e.g. "Checked in booking 2026-0012".
    pub summary: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
