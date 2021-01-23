use pika_backup::config;

#[test]
fn config_v0() {
    assert!(config::Settings::from_path(std::path::Path::new("tests/config_v0.json")).is_ok());
}
