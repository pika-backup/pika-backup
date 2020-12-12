use pika_backup::shared;

#[test]
fn config_v0() {
    assert!(shared::Settings::from_path(std::path::Path::new("tests/config_v0.json")).is_ok());
}
