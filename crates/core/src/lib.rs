//! Pure domain logic for DesktopBooker.
//!
//! This crate must stay free of I/O (no database, files, or network) so it
//! can be unit-tested quickly and reused by every front end (desktop, LAN
//! server, Android).

/// Crate version, handy for an "About" screen.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!version().is_empty());
    }
}
