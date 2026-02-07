mod test_common;
use macro_rules_attribute::apply;
use matches::assert_matches;
use smol_macros::test;
pub use test_common::borg::CommandRun;
use test_common::borg::prelude::*;
use test_common::{borg, config};

// Currently, there are no init tasks
fn init() {}

#[apply(test!)]
async fn simple_backup() {
    init();

    let config = tmp_config();

    let init = borg::CommandOnlyRepo::new(config.repo.clone());
    assert_matches!(init.clone().init().await, Ok(()));
    assert_matches!(init.peek().await, Ok(borg::List { .. }));

    let create = borg::Command::<borg::task::Create>::new(config);
    assert_matches!(create.run().await, Ok(_));
}

#[apply(test!)]
async fn backup_communication() -> borg::Result<()> {
    let config = tmp_config();

    let init = borg::CommandOnlyRepo::new(config.repo.clone());
    init.init().await?;

    let create = borg::Command::<borg::task::List>::new(config);
    let communication = create.communication.clone();
    create.run().await?;

    // TODO: We would probably need one file to get this into 'running'
    assert_matches!(communication.status(), borg::RunStatus::Init);

    Ok(())
}

#[apply(test!)]
async fn encrypted_backup() {
    init();
    let mut config = tmp_config();
    config.encrypted = true;

    let mut init = borg::CommandOnlyRepo::new(config.repo.clone());
    init.set_password(config::Password::new("x".into()));

    assert_matches!(init.clone().init().await, Ok(()));
    assert_matches!(init.peek().await, Ok(borg::List { .. }));

    let create = borg::Command::<borg::task::Create>::new(config);

    let result = create.run().await;

    assert!(result.is_err());
}

#[apply(test!)]
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
#[apply(test!)]
async fn failed_repo() {
    init();
    let result = borg::CommandOnlyRepo::new(tmp_config().repo).peek().await;
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::RepositoryDoesNotExist
        ))
    );
}

fn tmp_config() -> config::Backup {
    let uuid = glib::uuid_string_random().to_string();
    let mut config = test_common::config(std::path::Path::new(&format!("/tmp/{}", &uuid)));
    config.include.insert("/dev/null".into());
    config
}
