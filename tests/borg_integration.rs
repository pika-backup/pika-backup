mod common;

#[macro_use]
extern crate matches;

use pika_backup::{borg, borg::prelude::*, config};

// Currently, there are no init tasks
fn init() {}

#[test]
fn simple_backup() {
    init();
    let borg = borg::Borg::new(config());
    assert_matches!(borg.init(), Ok(borg::List { .. }));
    assert_matches!(borg.create(status()), Ok(_));
}

#[test]
fn backup_communication() -> borg::Result<()> {
    let borg = borg::Borg::new(config());
    let communication = status();

    borg.init()?;
    borg.create(communication.clone())?;

    assert_matches!(communication.status.get().run, borg::Run::Running);

    Ok(())
}

#[test]
fn encrypted_backup() {
    init();
    let mut config = config();
    config.encrypted = true;

    let mut borg = borg::Borg::new(config);
    borg.set_password(b"x".to_vec().into());

    assert_matches!(borg.init(), Ok(borg::List { .. }));

    borg.unset_password();
    assert_matches!(borg.create(status()), Err(borg::Error::PasswordMissing));
}

#[test]
fn failed_ssh_connection() {
    init();
    let repo = config::remote::Repository::from_uri("ssh://backup.server.invalid/repo".to_string())
        .into_config();

    let result = borg::BorgOnlyRepo::new(repo).peek();
    assert_matches!(
        result,
        Err(borg::Error::Failed(
            borg::error::Failure::ConnectionClosedWithHint_(_)
        ))
    );
}

#[test]
fn failed_repo() {
    init();
    let result = borg::Borg::new(config()).peek();
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
