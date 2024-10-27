# Contributing to Pika Backup

Contributions of all kind and with all levels of experience are very welcome. Please note that the [GNOME Code of Conduct](https://conduct.gnome.org/) applies to this project.

You can learn more about how to contribute on [welcome.gnome.org](https://welcome.gnome.org/app/PikaBackup/).

## Resources

- [User Help Pages](https://world.pages.gitlab.gnome.org/pika-backup/help/C/index.html)
- [Translation status](https://l10n.gnome.org/module/pika-backup/)
- [Code documentation](https://world.pages.gitlab.gnome.org/pika-backup/code-doc/pika_backup/)
- [Pika Backup on welcome.gnome.org](https://welcome.gnome.org/app/PikaBackup/)

## Translation

The translation of Pika Backup is managed by the [GNOME Translation Project](https://wiki.gnome.org/TranslationProject) and the respective [language teams](https://l10n.gnome.org/teams/). The translation status is available on the [module page](https://l10n.gnome.org/module/pika-backup/).

## Nightly Builds

When reporting bugs or feature requests, you can try out the [nightly build](https://welcome.gnome.org/app/PikaBackup/#installing-a-nightly-build) first to see if your issue has already been resolved.

## Debugging

The log level can be adjusted by setting the `G_MESSAGES_DEBUG` to `all`. For example

```sh
$ G_MESSAGES_PREFIXED="" G_MESSAGES_DEBUG=all cargo run
```

Currently, `pika-backup` and `pika-backup-trace` are used as logging domains. You can use `G_MESSAGES_DEBUG=pika-backup` to get debug, but no trace output.

See ["Running GLib Applications"](https://developer.gnome.org/glib/stable/glib-running.html) for more options.

## Peculiarities

### Help format

The help pages are currently written in [ducktype](http://projectmallard.org/ducktype/1.0/index.html). The files are stored in `help/C/duck` and the corresponding `.page`-files can be generated via `make -C help/C/`. Afterwards, you can preview the generated help pages via `yelp help/C/index.page`. The generated `.page`-files have to be committed to the repository as well. The `ducktype` program required for running `make` is probably packaged in you distro and is also [availabe on GitHub](https://github.com/projectmallard/mallard-ducktype).

### Flatpak manifests

The `org.gnome.World.PikaBackup.Devel.json` manifest is generated via `generate-manifest.sh`. Please adjust the `org.gnome.World.PikaBackup.yml` manifest and generate the devel version from it.
