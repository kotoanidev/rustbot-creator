//! Stores bot tokens and other secrets in the OS keyring (D-020): the Secret Service on
//! Linux (KDE Wallet or GNOME Keyring), never a project file or git.
//!
//! Entries live under the service [`SERVICE`], named after what they hold, e.g.
//! `<project id>/discord-token` (D-050).

use uuid::Uuid;

/// The keyring service of every entry: the app id (D-061).
pub const SERVICE: &str = "io.github.kotoanidev.RustBotCreator";

/// Why the keyring could not be used, e.g. no Secret Service is running or the user
/// refused to unlock it.
#[derive(Debug, thiserror::Error)]
#[error("the system keyring: {0}")]
pub struct Error(#[from] keyring::Error);

fn bot_token_entry(project: Uuid) -> Result<keyring::Entry, Error> {
    Ok(keyring::Entry::new(
        SERVICE,
        &format!("{project}/discord-token"),
    )?)
}

/// The project's bot token, or `None` if none is stored.
///
/// # Errors
/// The keyring could not be used.
pub fn bot_token(project: Uuid) -> Result<Option<String>, Error> {
    match bot_token_entry(project)?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Stores the project's bot token, replacing any stored one. Surrounding whitespace from
/// pasting is dropped.
///
/// # Errors
/// The keyring could not be used.
pub fn set_bot_token(project: Uuid, token: &str) -> Result<(), Error> {
    Ok(bot_token_entry(project)?.set_password(token.trim())?)
}

/// Forgets the project's bot token. Nothing stored is not an error.
///
/// # Errors
/// The keyring could not be used.
pub fn delete_bot_token(project: Uuid) -> Result<(), Error> {
    match bot_token_entry(project)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn cloud_session_entry() -> Result<keyring::Entry, Error> {
    Ok(keyring::Entry::new(SERVICE, "cloud/session")?)
}

/// The RustBot Cloud session (D-160), or `None` when signed out.
///
/// # Errors
/// The keyring could not be used.
pub fn cloud_session() -> Result<Option<String>, Error> {
    match cloud_session_entry()?.get_password() {
        Ok(session) => Ok(Some(session)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Stores the RustBot Cloud session, replacing any stored one.
///
/// # Errors
/// The keyring could not be used.
pub fn set_cloud_session(session: &str) -> Result<(), Error> {
    Ok(cloud_session_entry()?.set_password(session)?)
}

/// Forgets the RustBot Cloud session. Nothing stored is not an error.
///
/// # Errors
/// The keyring could not be used.
pub fn delete_cloud_session() -> Result<(), Error> {
    match cloud_session_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
