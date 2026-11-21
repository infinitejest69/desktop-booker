use booker_store::StoreError;
use serde::Serialize;

/// What the frontend receives when a command fails.
///
/// Serialised as `{ "kind": "validation", "message": "…" }`. `message` is
/// always safe to show to staff; internal details go to the log instead.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// A business rule was broken (e.g. overlapping rooms).
    Validation,
    NotFound,
    /// Something unexpected (database trouble etc.).
    Internal,
}

/// Lets commands use `?` on store calls: `StoreError` → `CommandError`.
impl From<StoreError> for CommandError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Validation(e) => Self {
                kind: ErrorKind::Validation,
                message: e.to_string(),
            },
            StoreError::NotFound { .. } => Self {
                kind: ErrorKind::NotFound,
                message: err.to_string(),
            },
            other => {
                eprintln!("[desktop-booker] internal error: {other:?}");
                Self {
                    kind: ErrorKind::Internal,
                    message: "Something went wrong saving or loading data. Please try again."
                        .into(),
                }
            }
        }
    }
}

pub type CmdResult<T> = Result<T, CommandError>;
