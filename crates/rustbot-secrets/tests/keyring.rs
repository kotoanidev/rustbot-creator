//! The bot token in the real system keyring.

use uuid::Uuid;

/// Needs a running Secret Service, which CI containers do not have:
/// `cargo test -p rustbot-secrets -- --ignored`. It removes what it stores.
#[test]
#[ignore = "uses the system keyring"]
fn bot_token_round_trip() {
    let project = Uuid::new_v4();
    assert_eq!(rustbot_secrets::bot_token(project).unwrap(), None);

    rustbot_secrets::set_bot_token(project, "  first.token \n").unwrap();
    assert_eq!(
        rustbot_secrets::bot_token(project).unwrap().as_deref(),
        Some("first.token")
    );
    rustbot_secrets::set_bot_token(project, "second.token").unwrap();
    assert_eq!(
        rustbot_secrets::bot_token(project).unwrap().as_deref(),
        Some("second.token")
    );

    rustbot_secrets::delete_bot_token(project).unwrap();
    assert_eq!(rustbot_secrets::bot_token(project).unwrap(), None);
    rustbot_secrets::delete_bot_token(project).unwrap();
}

#[test]
#[ignore = "uses the system keyring"]
fn cloud_session_round_trip() {
    // A real session is never touched.
    if rustbot_secrets::cloud_session().unwrap().is_some() {
        return;
    }
    rustbot_secrets::set_cloud_session("session").unwrap();
    assert_eq!(
        rustbot_secrets::cloud_session().unwrap().as_deref(),
        Some("session")
    );
    rustbot_secrets::delete_cloud_session().unwrap();
    assert_eq!(rustbot_secrets::cloud_session().unwrap(), None);
    rustbot_secrets::delete_cloud_session().unwrap();
}
