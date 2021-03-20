#[macro_use]
extern crate matches;

use pika_backup::borg;
mod common;
use common::*;

#[test]
fn borg_bin_missing() {
    std::env::set_var("PATH", "");
    let result =
        borg::Borg::new(config(Path::new("/tmp/test_borg_bin_missing"))).create(Default::default());
    assert_matches!(result, Err(borg::Error::Io(std::io::Error { .. })));
}
