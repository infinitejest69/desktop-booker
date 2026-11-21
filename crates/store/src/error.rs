use booker_core::ValidationError;
use thiserror::Error;

/// Everything that can go wrong in the store layer.
///
/// `#[from]` lets the `?` operator convert a lower-level error (e.g.
/// `sqlx::Error`) into a `StoreError` automatically.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A business rule was broken; the message is safe to show to staff.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("{what} not found")]
    NotFound { what: &'static str },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    /// Data in the file doesn't match what the app expects (e.g. a bad UUID).
    #[error("unexpected data in database: {0}")]
    Corrupt(String),
}

/// Shorthand used throughout the crate.
pub type StoreResult<T> = Result<T, StoreError>;
