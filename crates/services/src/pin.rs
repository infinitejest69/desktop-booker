//! PIN hashing with Argon2id.
//!
//! A PIN is never stored, only a one-way hash with a random salt, in the
//! standard "PHC" text format (`$argon2id$v=19$...`). Checking a PIN hashes the
//! attempt with the stored salt and compares.

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("couldn't hash the PIN: {0}")]
pub struct PinError(String);

/// Hashes a PIN for storage. Each call uses a new random salt, so the same
/// PIN gives a different hash every time.
pub fn hash_pin(pin: &str) -> Result<String, PinError> {
    Argon2::default()
        .hash_password(pin.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|e| PinError(e.to_string()))
}

/// True if `pin` matches a hash made by [`hash_pin`]. A malformed hash is
/// treated as "doesn't match" rather than an error.
pub fn verify_pin(pin: &str, stored: &str) -> bool {
    PasswordHash::new(stored).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(pin.as_bytes(), &hash)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_pin_verifies_and_wrong_pin_does_not() {
        let hash = hash_pin("2468").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_pin("2468", &hash));
        assert!(!verify_pin("2469", &hash));
    }

    #[test]
    fn hashes_are_salted() {
        assert_ne!(hash_pin("1234").unwrap(), hash_pin("1234").unwrap());
    }

    #[test]
    fn garbage_hash_never_verifies() {
        assert!(!verify_pin("1234", "not a hash"));
        assert!(!verify_pin("1234", ""));
    }
}
