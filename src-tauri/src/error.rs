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
    /// Seconds until trying again makes sense (e.g. after too many wrong PINs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// A business rule was broken (e.g. overlapping rooms).
    Validation,
    NotFound,
    /// Something unexpected (database trouble etc.).
    Internal,
    /// Nobody is signed in (or the app locked itself): show the lock screen.
    Locked,
    /// The signed-in user's role doesn't allow this.
    Forbidden,
}

impl CommandError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Validation,
            retry_after: None,
            message: message.into(),
        }
    }

    /// Too many wrong PINs: the lock screen disables sign-in and counts down.
    pub fn locked_out(seconds: u64) -> Self {
        Self {
            kind: ErrorKind::Validation,
            message: "Too many wrong PINs.".into(),
            retry_after: Some(seconds),
        }
    }

    pub fn locked() -> Self {
        Self {
            kind: ErrorKind::Locked,
            retry_after: None,
            message: "Please sign in to continue.".into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Forbidden,
            retry_after: None,
            message: message.into(),
        }
    }

    pub fn internal(detail: impl std::fmt::Display) -> Self {
        eprintln!("[desktop-booker] internal error: {detail}");
        Self {
            kind: ErrorKind::Internal,
            retry_after: None,
            message: "Something went wrong. Please try again.".into(),
        }
    }
}

/// Lets commands use `?` on store calls: `StoreError` → `CommandError`.
impl From<StoreError> for CommandError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Validation(e) => Self {
                kind: ErrorKind::Validation,
                retry_after: None,
                message: e.to_string(),
            },
            StoreError::NotFound { .. } => Self {
                kind: ErrorKind::NotFound,
                retry_after: None,
                message: err.to_string(),
            },
            other => {
                eprintln!("[desktop-booker] internal error: {other:?}");
                Self {
                    kind: ErrorKind::Internal,
                    retry_after: None,
                    message: "Something went wrong saving or loading data. Please try again."
                        .into(),
                }
            }
        }
    }
}

pub type CmdResult<T> = Result<T, CommandError>;
