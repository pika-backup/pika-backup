use cmd_lib::run_fun;

mod test_common;
use common::borg::CommandRun;
use macro_rules_attribute::apply;
use smol_macros::test;
use test_common::*;

#[apply(test!)]
async fn predefined() -> Result<(), Box<dyn std::error::Error>> {
    let borg_base = tempdir()?;
    let home = tempdir()?;

    let _env_vars = [
        tmp_env::set_var("HOME", home.path()),
        tmp_env::set_var("BORG_BASE_DIR", borg_base.path()),
    ];

    let mut excluded = Excluded::new(&home);
    // Caches
    excluded.add(".cache");
    excluded.add(".var/app/example.app/cache");
    // Flatpak apps
    excluded.add(".local/share/flatpak/repo/");
    // Trash
    excluded.add(".local/share/Trash");
    // VMs
    excluded.add(".local/share/gnome-boxes");
    excluded.add(".var/app/org.gnome.Boxes");
    excluded.add(".var/app/org.gnome.BoxesDevel");
    excluded.add(".local/share/bottles");
    excluded.add(".var/app/com.usebottles.bottles");
    excluded.add(".local/share/libvirt");
    excluded.add(".local/share/libvirt");
    excluded.add(".local/share/containers");

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
        config::Exclude::from_predefined(config::exclude::Predefined::VmsContainers),
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
