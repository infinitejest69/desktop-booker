//! Who is signed in, when they last did something, and wrong-PIN lockouts.
//!
//! Tauri gives every command a shared `&Session`, and commands can run at the
//! same time, so the state sits behind a `Mutex`: `lock()` hands out exclusive
//! access, and the guard it returns releases the lock when it's dropped at the
//! end of the block. Nothing here is saved; restarting the app signs everyone
//! out.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use booker_core::{Permission, User, UserId};
use booker_store::SqliteStore;

use crate::error::{CmdResult, CommandError};

/// Wrong PINs allowed before that user is locked out for a while.
const MAX_ATTEMPTS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
}

struct State {
    user: Option<User>,
    last_active: Instant,
    idle_lock: Option<Duration>,
    attempts: HashMap<UserId, Attempts>,
}

pub struct Session {
    state: Mutex<State>,
}

/// Whole seconds, rounded up so "0 seconds left" never shows while still locked.
fn seconds_left(d: Duration) -> u64 {
    d.as_secs() + u64::from(d.subsec_nanos() > 0)
}

/// Minutes from settings → how long until the app locks (0 = never).
fn idle_duration(minutes: u16) -> Option<Duration> {
    (minutes > 0).then(|| Duration::from_secs(u64::from(minutes) * 60))
}

impl Session {
    pub fn new(idle_lock_minutes: u16) -> Self {
        Self {
            state: Mutex::new(State {
                user: None,
                last_active: Instant::now(),
                idle_lock: idle_duration(idle_lock_minutes),
                attempts: HashMap::new(),
            }),
        }
    }

    /// `lock()` only fails if another thread panicked while holding the lock;
    /// the data is still usable, so carry on with it rather than crash.
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_idle_lock(&self, minutes: u16) {
        self.state().idle_lock = idle_duration(minutes);
    }

    /// The signed-in user, signing them out first if they've been idle too long.
    /// Counts as activity, so each command keeps the session alive.
    pub fn current_user(&self) -> Option<User> {
        let mut s = self.state();
        let now = Instant::now();
        if let Some(limit) = s.idle_lock
            && now.duration_since(s.last_active) > limit
        {
            s.user = None;
        }
        if s.user.is_some() {
            s.last_active = now;
        }
        s.user.clone()
    }

    /// The store, acting as the signed-in user, if they may do `permission`.
    pub fn require(&self, store: &SqliteStore, permission: Permission) -> CmdResult<SqliteStore> {
        let user = self.current_user().ok_or_else(CommandError::locked)?;
        if !user.can(permission) {
            return Err(CommandError::forbidden(match permission {
                Permission::OverridePrices => "Only an admin can set a price by hand.",
                Permission::CancelBookings => "Only an admin can cancel bookings.",
                Permission::ManageUsers => "Only an admin can manage users.",
                Permission::ViewAudit => "Only an admin can see the audit log.",
                Permission::ViewReports => "Only an admin can see reports.",
                _ => "Only an admin can change the hotel's setup.",
            }));
        }
        Ok(store.as_user(&user))
    }

    /// Fails while this user is locked out after too many wrong PINs, saying
    /// how many seconds are left. Trying during a lockout doesn't extend it.
    pub fn check_not_locked_out(&self, id: UserId) -> CmdResult<()> {
        let s = self.state();
        if let Some(until) = s.attempts.get(&id).and_then(|a| a.locked_until) {
            let now = Instant::now();
            if now < until {
                return Err(CommandError::locked_out(seconds_left(until - now)));
            }
        }
        Ok(())
    }

    /// Records a wrong PIN. Returns the error to show: how many tries are left,
    /// or, on the fifth in a row, the start of a lockout.
    pub fn failed_attempt(&self, id: UserId) -> CommandError {
        let mut s = self.state();
        let a = s.attempts.entry(id).or_default();
        a.failures += 1;
        if a.failures >= MAX_ATTEMPTS {
            a.failures = 0;
            a.locked_until = Some(Instant::now() + LOCKOUT);
            return CommandError::locked_out(LOCKOUT.as_secs());
        }
        let left = MAX_ATTEMPTS - a.failures;
        CommandError::validation(if left == 1 {
            "Wrong PIN. One more wrong try locks sign-in for 30 seconds.".to_string()
        } else {
            format!("Wrong PIN. {left} tries left.")
        })
    }

    pub fn sign_in(&self, user: User) {
        let mut s = self.state();
        s.attempts.remove(&user.id);
        s.user = Some(user);
        s.last_active = Instant::now();
    }

    pub fn sign_out(&self) {
        self.state().user = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booker_core::Role;

    fn user(role: Role) -> User {
        User {
            id: UserId::new(),
            name: "Sam".into(),
            role,
            active: true,
        }
    }

    #[test]
    fn signed_out_until_someone_signs_in() {
        let s = Session::new(15);
        assert!(s.current_user().is_none());
        let u = user(Role::Staff);
        s.sign_in(u.clone());
        assert_eq!(s.current_user(), Some(u));
        s.sign_out();
        assert!(s.current_user().is_none());
    }

    #[test]
    fn idle_too_long_signs_out() {
        let s = Session::new(15);
        s.sign_in(user(Role::Staff));
        s.state().last_active = Instant::now() - Duration::from_secs(16 * 60);
        assert!(s.current_user().is_none());
    }

    #[test]
    fn zero_minutes_never_locks() {
        let s = Session::new(0);
        s.sign_in(user(Role::Staff));
        s.state().last_active = Instant::now() - Duration::from_secs(24 * 3600);
        assert!(s.current_user().is_some());
    }

    fn store() -> SqliteStore {
        tauri::async_runtime::block_on(SqliteStore::open_in_memory()).unwrap()
    }

    #[test]
    fn staff_cannot_change_setup_or_prices_but_can_book() {
        let s = Session::new(15);
        let db = store();
        assert!(matches!(
            s.require(&db, Permission::ViewHotel),
            Err(CommandError {
                kind: crate::error::ErrorKind::Locked,
                ..
            })
        ));
        s.sign_in(user(Role::Staff));
        assert!(s.require(&db, Permission::MakeBookings).is_ok());
        assert!(s.require(&db, Permission::CancelBookings).is_ok());
        for p in [
            Permission::ManageSetup,
            Permission::OverridePrices,
            Permission::ManageUsers,
        ] {
            assert!(matches!(
                s.require(&db, p),
                Err(CommandError {
                    kind: crate::error::ErrorKind::Forbidden,
                    ..
                })
            ));
        }
        s.sign_in(user(Role::Admin));
        assert!(s.require(&db, Permission::ManageSetup).is_ok());
    }

    #[test]
    fn five_wrong_pins_lock_that_user_out() {
        let s = Session::new(15);
        let id = UserId::new();
        for left in [4, 3, 2] {
            let e = s.failed_attempt(id);
            assert_eq!(e.retry_after, None);
            assert!(
                e.message.contains(&format!("{left} tries left")),
                "{}",
                e.message
            );
            assert!(s.check_not_locked_out(id).is_ok());
        }
        assert!(s.failed_attempt(id).message.contains("One more wrong try"));
        // The fifth wrong PIN starts the lockout straight away.
        assert_eq!(s.failed_attempt(id).retry_after, Some(30));
        let during = s.check_not_locked_out(id).unwrap_err();
        assert!(matches!(during.retry_after, Some(29 | 30)));
        // Trying again during the lockout doesn't restart it.
        s.state().attempts.get_mut(&id).unwrap().locked_until =
            Some(Instant::now() + Duration::from_secs(10));
        assert!(s.check_not_locked_out(id).unwrap_err().retry_after.unwrap() <= 10);
        assert!(
            s.check_not_locked_out(UserId::new()).is_ok(),
            "other users unaffected"
        );
    }
}
