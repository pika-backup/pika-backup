#[macro_use]
extern crate matches;

use pika_backup::borg;
mod common;
use common::*;

#[async_std::test]
async fn borg_bin_missing() {
    std::env::set_var("PATH", "");
    let result =
        borg::Command::<borg::task::Create>::new(config(Path::new("/tmp/test_borg_bin_missing")))
            .run()
            .await;
    assert_matches!(result, Err(borg::Error::Io(std::io::Error { .. })));
}
