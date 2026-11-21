//! Staff accounts, roles and what each role may do.

use serde::{Deserialize, Serialize};

use crate::ids::UserId;
use crate::validation::{ValidationError, ValidationResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Role {
    Admin,
    Staff,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Staff => "staff",
        }
    }

    /// The whole permission table in one place. Admins may do everything;
    /// staff run the front desk but don't change setup, prices or people.
    pub const fn allows(self, permission: Permission) -> bool {
        match self {
            Role::Admin => true,
            Role::Staff => matches!(
                permission,
                Permission::ViewHotel | Permission::MakeBookings | Permission::CancelBookings
            ),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "admin" => Ok(Role::Admin),
            "staff" => Ok(Role::Staff),
            other => Err(format!("unknown role {other:?}")),
        }
    }
}

/// Something a signed-in person might try to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Permission {
    /// See rooms, the chart, bookings and the dashboard.
    ViewHotel,
    /// Create and edit bookings, check in and out.
    MakeBookings,
    /// Cancel bookings and mark no-shows.
    CancelBookings,
    /// Set a room's price by hand on a booking.
    OverridePrices,
    /// Hotel details, floors, room types, layout, rates and board.
    ManageSetup,
    /// Add staff, change roles and PINs.
    ManageUsers,
    /// Read the audit log.
    ViewAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub role: Role,
    /// Inactive users can't sign in but stay in the audit log.
    pub active: bool,
}

impl User {
    pub fn can(&self, permission: Permission) -> bool {
        self.active && self.role.allows(permission)
    }
}

/// What the app needs to know before showing anything: whether to run first-time
/// setup, show the lock screen, or carry on as the signed-in user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SessionStatus {
    /// No users exist yet: create the first admin.
    pub needs_setup: bool,
    /// The signed-in user, or `None` when locked.
    pub user: Option<User>,
    /// Active users to choose from on the lock screen.
    pub users: Vec<User>,
    pub hotel_name: String,
    /// Lock after this many idle minutes (0 = never).
    pub idle_lock_minutes: u16,
}

pub const PIN_MIN: usize = 4;
pub const PIN_MAX: usize = 8;

/// A PIN is 4–8 digits.
pub fn validate_pin(pin: &str) -> ValidationResult {
    let ok = (PIN_MIN..=PIN_MAX).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(ValidationError::InvalidPin)
    }
}

/// `others` = every existing user (the one being saved may be among them).
pub fn validate_user(user: &User, others: &[User]) -> ValidationResult {
    if user.name.trim().is_empty() {
        return Err(ValidationError::Empty { field: "Name" });
    }
    if let Some(dup) = others
        .iter()
        .find(|o| o.id != user.id && o.name.trim().eq_ignore_ascii_case(user.name.trim()))
    {
        return Err(ValidationError::DuplicateName {
            name: dup.name.clone(),
        });
    }
    // Never leave the hotel without an active admin who can manage users.
    let admins_after = others
        .iter()
        .filter(|o| o.id != user.id)
        .chain(std::iter::once(user))
        .filter(|u| u.active && u.role == Role::Admin)
        .count();
    if admins_after == 0 {
        return Err(ValidationError::LastAdmin);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(name: &str, role: Role) -> User {
        User {
            id: UserId::new(),
            name: name.into(),
            role,
            active: true,
        }
    }

    #[test]
    fn staff_run_the_desk_but_not_setup_prices_or_people() {
        for p in [
            Permission::ViewHotel,
            Permission::MakeBookings,
            Permission::CancelBookings,
        ] {
            assert!(Role::Staff.allows(p), "{p:?}");
        }
        for p in [
            Permission::OverridePrices,
            Permission::ManageSetup,
            Permission::ManageUsers,
            Permission::ViewAudit,
        ] {
            assert!(!Role::Staff.allows(p), "{p:?}");
            assert!(Role::Admin.allows(p), "{p:?}");
        }
    }

    #[test]
    fn inactive_users_can_do_nothing() {
        let mut u = user("Andy", Role::Admin);
        u.active = false;
        assert!(!u.can(Permission::ViewHotel));
    }

    #[test]
    fn pins_are_four_to_eight_digits() {
        for good in ["1234", "00000000", "987654"] {
            assert_eq!(validate_pin(good), Ok(()), "{good}");
        }
        for bad in ["123", "123456789", "12a4", " 1234", ""] {
            assert_eq!(validate_pin(bad), Err(ValidationError::InvalidPin), "{bad}");
        }
    }

    #[test]
    fn names_are_unique_and_required() {
        let a = user("Andy", Role::Admin);
        let b = user(" andy ", Role::Staff);
        assert!(matches!(
            validate_user(&b, std::slice::from_ref(&a)),
            Err(ValidationError::DuplicateName { .. })
        ));
        assert_eq!(
            validate_user(&user(" ", Role::Admin), &[]),
            Err(ValidationError::Empty { field: "Name" })
        );
    }

    #[test]
    fn the_last_active_admin_cannot_be_removed() {
        let mut admin = user("Andy", Role::Admin);
        let staff = user("Sam", Role::Staff);
        let all = vec![admin.clone(), staff.clone()];
        admin.role = Role::Staff;
        assert_eq!(validate_user(&admin, &all), Err(ValidationError::LastAdmin));
        admin.role = Role::Admin;
        admin.active = false;
        assert_eq!(validate_user(&admin, &all), Err(ValidationError::LastAdmin));
        // A first admin on an empty hotel is fine; a first staff member isn't.
        assert_eq!(validate_user(&user("Andy", Role::Admin), &[]), Ok(()));
        assert_eq!(validate_user(&staff, &[]), Err(ValidationError::LastAdmin));
    }

    #[test]
    fn roles_round_trip() {
        for r in [Role::Admin, Role::Staff] {
            assert_eq!(r.as_str().parse::<Role>(), Ok(r));
        }
    }
}
