mod common;
pub use pika_backup::borg::CommandRun;

#[macro_use]
extern crate matches;

use pika_backup::{borg, borg::prelude::*, config};

// Currently, there are no init tasks
fn init() {}

#[async_std::test]
async fn simple_backup() {
    init();

    let config = config();

    let init = borg::CommandOnlyRepo::new(config.repo.clone());
    assert_matches!(init.init().await, Ok(borg::List { .. }));

    let create = borg::Command::<borg::task::Create>::new(config);
    assert_matches!(create.run().await, Ok(_));
}

#[async_std::test]
async fn backup_communication() -> borg::Result<()> {
    let config = config();

    let init = borg::CommandOnlyRepo::new(config.repo.clone());
    init.init().await?;

    let create = borg::Command::<borg::task::List>::new(config);
    let communication = create.communication.clone();
    create.run().await?;

    // TODO: We would probably need one file to get this into 'running'
    assert_matches!(communication.status(), borg::Run::Init);

    Ok(())
}

#[async_std::test]
async fn encrypted_backup() {
    init();
    let mut config = config();
    config.encrypted = true;

    let mut init = borg::CommandOnlyRepo::new(config.repo.clone());
    init.set_password(config::Password::new("x".into()));

    assert_matches!(init.init().await, Ok(borg::List { .. }));

    let create = borg::Command::<borg::task::Create>::new(config);

    let result = create.run().await;

    assert!(result.is_err());
}

#[async_std::test]
async fn failed_ssh_connection() {
    init();
    let repo = config::remote::Repository::from_uri("ssh://backup.server.invalid/repo".to_string())
        .into_config();

    let result = borg::CommandOnlyRepo::new(repo).peek().await;
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::ConnectionClosedWithHint_(_)
        )) | Err(borg::Error::Failed(
            borg::error::Failure::ConnectionClosedWithHint
        ))
    );
}

// TODO: peek with what task
#[async_std::test]
async fn failed_repo() {
    init();
    let result = borg::CommandOnlyRepo::new(config().repo).peek().await;
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::RepositoryDoesNotExist
        ))
    );
}

fn config() -> config::Backup {
    let uuid = glib::uuid_string_random().to_string();
    let mut config = common::config(std::path::Path::new(&format!("/tmp/{}", &uuid)));
    config.include.insert("/dev/null".into());
    config
}
