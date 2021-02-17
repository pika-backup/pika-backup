# Contributing to Pika Backup

Contributions of all kind and with all levels of experience are very welcome. Please note that the [GNOME Code of Conduct](https://wiki.gnome.org/Foundation/CodeOfConduct) applies to this project.

The translation of Pika Backup is managed by the [GNOME Translation Project](https://wiki.gnome.org/TranslationProject) and the respective [language teams](https://l10n.gnome.org/teams/). The translation status is available on the [module page](https://l10n.gnome.org/module/pika-backup/).

## Start using GNOME Builder

You can clone the project using GNOME Builder.

![builder_setup](/uploads/f5b239c191c15922a615a28a55110b1c/builder_setup.png)

GNOME Builder will suggest to install the missing dependencies. After doing so, there currently remains one dependency that has to be installed manually. (The reason is that GNOME Builder cannot determine the required version of the dependency yet.)

```sh
$ flatpak install -y --user org.freedesktop.Sdk.Extension.rust-stable//20.08
```

Afterwards you should be ready to build and start Pika Backup via the "Run" button or by pressing *Ctrl+F5*.

## Peculiarities

### GtkBuilder files

Binding to objects defined in `.ui`-files are auto generated via

```sh
$ ./build-aux/generate-ui-bindings.py
```

You have to execute this script after adding, changing or removing ids from `.ui`-files. The bindings can be found in `src/ui/builder.rs`. Using only those bindings allows to catch all errors in accessing builder elements on compile time.

### Flatpak manifests

The `org.gnome.World.PikaBackup.Devel.json` manifest is generated via `generate-manifest.sh`. Please adjust the `org.gnome.World.PikaBackup.yml` manifest and generate the devel version from it.

Outside of GNOME Builder the flatpak manifests depend on the generated `generated-sources.json` file. After changes of the `Cargo.lock` file this file must also be updated via executing `generate-manifest.sh`.

## Debugging

The log level can be adjusted by setting the `G_MESSAGES_DEBUG` to `all`. For example

```sh
$ G_MESSAGES_DEBUG=all cargo run
```

See ["Running GLib Applications"](https://developer.gnome.org/glib/stable/glib-running.html) for more options.

## Building manually

Building via [cargo](https://rustup.rs/) not involving meson is supported.

```
$ apt install libgtk-3-dev borg-backup
$ cargo test
$ cargo run
```

Using meson also installs a `.desktop`-file etc.

```
$ meson --sysconfdir /etc builddir
$ ninja install -C builddir
```
