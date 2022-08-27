use cmd_lib::run_fun;

mod common;
use common::*;

#[async_std::test]
async fn predefined() -> Result<(), Box<dyn std::error::Error>> {
    let borg_base = tempdir()?;
    let home = tempdir()?;

    let _env_vars = vec![
        tmp_env::set_var("HOME", home.path()),
        tmp_env::set_var("BORG_BASE_DIR", borg_base.path()),
    ];

    let mut excluded = Excluded::new(&home);
    excluded.add(".cache");
    excluded.add(".local/share/flatpak/repo/");
    excluded.add(".local/share/Trash");
    excluded.add(".var/app/example.app/cache");

    let mut included = Included::new(&home);
    included.add("");
    included.add("Documents");
    included.add(".config");
    included.add(".local/share/flatpak/overrides");
    included.add(".var/app/example.app/config/cache");
    included.add(".var/app/example.app/config");

    let (mut config, _repo_dir) = tmpdir_config();

    config.include = BTreeSet::from([PathBuf::new()]);
    config.exclude = BTreeSet::from([
        config::Exclude::from_predefined(config::exclude::Predefined::Caches),
        config::Exclude::from_predefined(config::exclude::Predefined::FlatpakApps),
        config::Exclude::from_predefined(config::exclude::Predefined::Trash),
    ]);

    excluded.test(&config.exclude);
    included.test(&config.exclude);

    borg::CommandOnlyRepo::new(config.repo.clone())
        .init()
        .await?;
    let stats = borg::Command::<borg::task::Create>::new(config.clone())
        .run()
        .await?;

    let archive = format!("{}::{}", config.repo, stats.archive.name.as_str());

    let included_repo = run_fun!(
        borg list "$archive"
    )?;

    assert_eq!(
        included_repo.matches("include").count(),
        included.paths.len()
    );

    let list = run_fun!(
        borg list "$archive"
    )?;

    assert!(!list.contains("exclude"));

    Ok(())
}
