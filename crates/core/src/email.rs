//! Emails to guests: SMTP settings, templates with placeholders, and the
//! log of what was sent. Sending itself is in `booker-services`; the SMTP
//! password lives in the operating system's keychain, never the database.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::booking::BookingDetail;
use crate::ids::BookingId;
use crate::money::Money;
use crate::validation::{ValidationError, ValidationResult};

/// The hotel's outgoing mail server. Gmail: smtp.gmail.com, port 587,
/// STARTTLS, the Gmail address as user name and an *app password*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    /// The address guests see and reply to.
    pub from_address: String,
    /// e.g. "The Grange".
    pub from_name: String,
}

impl Default for EmailSettings {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 587,
            username: String::new(),
            from_address: String::new(),
            from_name: String::new(),
        }
    }
}

/// A very small sanity check for an email address: something@something.something.
pub fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    match s.split_once('@') {
        Some((user, domain)) => {
            !user.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !s.contains(char::is_whitespace)
        }
        None => false,
    }
}

pub fn validate_email_settings(s: &EmailSettings) -> ValidationResult {
    if s.host.trim().is_empty() {
        return Err(ValidationError::Empty {
            field: "Mail server",
        });
    }
    if s.username.trim().is_empty() {
        return Err(ValidationError::Empty { field: "User name" });
    }
    if !looks_like_email(&s.from_address) {
        return Err(ValidationError::InvalidEmail);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum EmailKind {
    Confirmation,
    Amendment,
    Cancellation,
}

impl EmailKind {
    pub const ALL: [EmailKind; 3] = [Self::Confirmation, Self::Amendment, Self::Cancellation];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmation => "confirmation",
            Self::Amendment => "amendment",
            Self::Cancellation => "cancellation",
        }
    }
}

impl std::str::FromStr for EmailKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| format!("unknown email kind {s:?}"))
    }
}

/// An email's wording. `{guest_name}`, `{reference}` and the other
/// [`PLACEHOLDERS`] are filled in when it's sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailTemplate {
    pub kind: EmailKind,
    pub subject: String,
    pub body: String,
}

/// The placeholders a template can use, for the settings page.
pub const PLACEHOLDERS: [&str; 10] = [
    "guest_name",
    "first_name",
    "reference",
    "arrival",
    "departure",
    "nights",
    "rooms",
    "total",
    "hotel_name",
    "hotel_phone",
];

impl EmailTemplate {
    /// The wording used until the hotel writes its own.
    pub fn default_for(kind: EmailKind) -> Self {
        let (subject, body) = match kind {
            EmailKind::Confirmation => (
                "Your booking at {hotel_name}: {reference}",
                "Dear {first_name},\n\nThank you for booking with us. We look forward to welcoming you.\n\nBooking: {reference}\nArriving: {arrival}\nLeaving: {departure} ({nights})\nRoom: {rooms}\nTotal: {total}\n\nIf anything changes, just reply to this email or call us on {hotel_phone}.\n\nKind regards,\n{hotel_name}",
            ),
            EmailKind::Amendment => (
                "Your booking at {hotel_name} has changed: {reference}",
                "Dear {first_name},\n\nWe've updated your booking. Here are the new details:\n\nBooking: {reference}\nArriving: {arrival}\nLeaving: {departure} ({nights})\nRoom: {rooms}\nTotal: {total}\n\nPlease let us know if anything isn't right.\n\nKind regards,\n{hotel_name}",
            ),
            EmailKind::Cancellation => (
                "Your booking at {hotel_name} is cancelled: {reference}",
                "Dear {first_name},\n\nAs requested, we've cancelled booking {reference} (arriving {arrival}).\n\nWe hope to welcome you another time.\n\nKind regards,\n{hotel_name}",
            ),
        };
        Self {
            kind,
            subject: subject.into(),
            body: body.into(),
        }
    }
}

/// Replaces each `{name}` with its value. Unknown placeholders are left as
/// they are, so a typo shows up in the test email rather than vanishing.
pub fn fill(text: &str, values: &[(&str, String)]) -> String {
    let mut out = text.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// The values for a booking's placeholders.
pub fn booking_values(
    detail: &BookingDetail,
    room_names: &[String],
    hotel_name: &str,
    hotel_phone: &str,
    money: impl Fn(Money) -> String,
) -> Vec<(&'static str, String)> {
    let arrival = detail.booking.rooms.iter().map(|r| r.stay.start).min();
    let departure = detail.booking.rooms.iter().map(|r| r.stay.end).max();
    let nights = match (arrival, departure) {
        (Some(a), Some(d)) => (d - a).num_days(),
        _ => 0,
    };
    let day = |d: Option<chrono::NaiveDate>| {
        d.map(|d| d.format("%A %-d %B %Y").to_string())
            .unwrap_or_default()
    };
    let first = detail.guest.first_name.trim();
    vec![
        ("guest_name", detail.guest.display_name()),
        (
            "first_name",
            if first.is_empty() {
                detail.guest.display_name()
            } else {
                first.to_string()
            },
        ),
        ("reference", detail.booking.reference.clone()),
        ("arrival", day(arrival)),
        ("departure", day(departure)),
        (
            "nights",
            format!("{nights} night{}", if nights == 1 { "" } else { "s" }),
        ),
        ("rooms", room_names.join(", ")),
        ("total", money(detail.total)),
        ("hotel_name", hotel_name.to_string()),
        ("hotel_phone", hotel_phone.to_string()),
    ]
}

/// One email sent (or tried).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EmailLogEntry {
    pub at: NaiveDateTime,
    pub to: String,
    pub subject: String,
    pub kind: Option<EmailKind>,
    pub booking_id: Option<BookingId>,
    pub sent: bool,
    /// Why it failed, if it did.
    pub error: String,
    pub sent_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_and_unknown_ones_left() {
        let text = "Dear {first_name}, booking {reference}. {oops}";
        let out = fill(
            text,
            &[
                ("first_name", "Jo".into()),
                ("reference", "2026-0001".into()),
            ],
        );
        assert_eq!(out, "Dear Jo, booking 2026-0001. {oops}");
    }

    #[test]
    fn default_templates_use_only_known_placeholders() {
        for kind in EmailKind::ALL {
            let t = EmailTemplate::default_for(kind);
            let values: Vec<(&str, String)> =
                PLACEHOLDERS.iter().map(|p| (*p, "x".to_string())).collect();
            let filled = fill(&format!("{} {}", t.subject, t.body), &values);
            assert!(!filled.contains('{'), "{kind:?} left a placeholder");
        }
    }

    #[test]
    fn settings_need_a_server_user_and_from_address() {
        let gmail = EmailSettings {
            host: "smtp.gmail.com".into(),
            port: 587,
            username: "grange.hotel@example.com".into(),
            from_address: "grange.hotel@example.com".into(),
            from_name: "The Grange".into(),
        };
        assert_eq!(validate_email_settings(&gmail), Ok(()));
        let bad = EmailSettings {
            from_address: "not an email".into(),
            ..gmail.clone()
        };
        assert_eq!(
            validate_email_settings(&bad),
            Err(ValidationError::InvalidEmail)
        );
        assert!(looks_like_email("a@b.co"));
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("@b.com"));
    }
}
