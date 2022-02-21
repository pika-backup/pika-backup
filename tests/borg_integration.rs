mod common;

#[macro_use]
extern crate matches;

use pika_backup::{borg, borg::prelude::*, config};

// Currently, there are no init tasks
fn init() {}

#[async_std::test]
async fn simple_backup() {
    init();
    let borg = borg::Borg::new(config());
    assert_matches!(borg.clone().init().await, Ok(borg::List { .. }));
    assert_matches!(borg.create(status()).await, Ok(_));
}

#[async_std::test]
async fn backup_communication() -> borg::Result<()> {
    let borg = borg::Borg::new(config());
    let communication = status();

    borg.clone().init().await?;
    borg.create(communication.clone()).await?;

    assert_matches!(communication.status.get().run, borg::Run::Running);

    Ok(())
}

#[async_std::test]
async fn encrypted_backup() {
    init();
    let mut config = config();
    config.encrypted = true;

    let mut borg = borg::Borg::new(config);
    borg.set_password(config::Password::new("x".into()));

    assert_matches!(borg.clone().init().await, Ok(borg::List { .. }));

    borg.unset_password();

    let result = borg.create(status()).await;

    assert!(result.is_err());
}

#[async_std::test]
async fn failed_ssh_connection() {
    init();
    let repo = config::remote::Repository::from_uri("ssh://backup.server.invalid/repo".to_string())
        .into_config();

    let result = borg::BorgOnlyRepo::new(repo).peek().await;
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::ConnectionClosedWithHint_(_)
        ))
    );
}

#[async_std::test]
async fn failed_repo() {
    init();
    let result = borg::Borg::new(config()).peek().await;
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::RepositoryDoesNotExist
        ))
    );
}

fn status() -> borg::Communication {
    Default::default()
}

fn config() -> config::Backup {
    let uuid = glib::uuid_string_random().to_string();
    let mut config = common::config(std::path::Path::new(&format!("/tmp/{}", &uuid)));
    config.include.insert("/dev/null".into());
    config
}
