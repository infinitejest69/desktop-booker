//! Sending email through the hotel's own SMTP account (e.g. Gmail), with
//! the password kept in the operating system's keychain (Windows
//! Credential Manager, macOS Keychain, the Secret Service on Linux).

use std::time::Duration;

use booker_core::EmailSettings;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// The keychain entry's "service"; the SMTP user name is its "user".
const SERVICE: &str = "DesktopBooker SMTP";

fn entry(username: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, username.trim())
        .map_err(|e| format!("Couldn't open the system keychain: {e}"))
}

/// Saves (or replaces) the SMTP password in the keychain.
pub fn save_password(username: &str, password: &str) -> Result<(), String> {
    entry(username)?
        .set_password(password)
        .map_err(|e| format!("Couldn't save the password in the system keychain: {e}"))
}

/// Whether a password is saved for this user name.
pub fn has_password(username: &str) -> bool {
    !username.trim().is_empty()
        && entry(username)
            .and_then(|e| e.get_password().map_err(|e| e.to_string()))
            .is_ok()
}

fn load_password(username: &str) -> Result<String, String> {
    entry(username)?.get_password().map_err(|_| {
        "No password is saved for this email account. Enter it in My hotel, Email.".to_string()
    })
}

/// The message itself, plain text, from the hotel to one guest.
pub fn build_message(
    settings: &EmailSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<Message, String> {
    let from_address = settings
        .from_address
        .trim()
        .parse()
        .map_err(|_| "The hotel's from address isn't a valid email address.".to_string())?;
    let name = settings.from_name.trim();
    let from = Mailbox::new((!name.is_empty()).then(|| name.to_string()), from_address);
    let to: Mailbox = to
        .trim()
        .parse()
        .map_err(|_| format!("{to:?} isn't a valid email address."))?;
    Message::builder()
        .from(from.clone())
        .reply_to(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| format!("Couldn't build the email: {e}"))
}

/// Sends one email. Port 465 uses TLS from the start; anything else
/// (587 for Gmail) upgrades with STARTTLS.
pub async fn send(
    settings: &EmailSettings,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let message = build_message(settings, to, subject, body)?;
    let password = load_password(&settings.username)?;
    let host = settings.host.trim();
    let builder = if settings.port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
    }
    .map_err(|e| format!("Couldn't reach the mail server {host}: {e}"))?;
    let transport = builder
        .port(settings.port)
        .credentials(Credentials::new(
            settings.username.trim().to_string(),
            password,
        ))
        .timeout(Some(Duration::from_secs(30)))
        .build();
    transport.send(message).await.map(|_| ()).map_err(|e| {
        let text = e.to_string();
        if text.contains("535") || text.to_lowercase().contains("authentication") {
            "The mail server refused the user name or password. For Gmail, use an app password \
             (myaccount.google.com/apppasswords), not your normal password."
                .to_string()
        } else {
            format!("The email couldn't be sent: {text}")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gmail() -> EmailSettings {
        EmailSettings {
            host: "smtp.gmail.com".into(),
            port: 587,
            username: "grange.hotel@example.com".into(),
            from_address: "grange.hotel@example.com".into(),
            from_name: "The Grange".into(),
        }
    }

    #[test]
    fn a_message_has_the_hotel_as_sender_and_reply_to() {
        let m = build_message(&gmail(), "jo@example.com", "Your booking", "Hello").unwrap();
        let text = String::from_utf8(m.formatted()).unwrap();
        assert!(text.contains("From: \"The Grange\" <grange.hotel@example.com>"));
        assert!(text.contains("Reply-To: \"The Grange\" <grange.hotel@example.com>"));
        assert!(text.contains("To: jo@example.com"));
        assert!(text.contains("Subject: Your booking"));
    }

    #[test]
    fn a_bad_address_is_explained() {
        let err = build_message(&gmail(), "not an address", "Hi", "Hi").unwrap_err();
        assert!(err.contains("isn't a valid email address"));
    }
}
