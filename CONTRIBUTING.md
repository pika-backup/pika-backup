# Contributing to Pika Backup

Contributions of all kind and with all levels of experience are very welcome. Please note that the [GNOME Code of Conduct](https://wiki.gnome.org/Foundation/CodeOfConduct) applies to this project.

## Start using GNOME Builder

You can clone the project using GNOME Builder.

![builder_setup](/uploads/f5b239c191c15922a615a28a55110b1c/builder_setup.png)

GNOME Builder will suggest to install the missing dependencies. After doing so, there currently remains one dependency that has to be installed manually. (The reason is that GNOME Builder cannot determine the required version of the dependency yet.)

```sh
$ flatpak install -y --user org.freedesktop.Sdk.Extension.rust-stable//20.08
```

Afterwards you should be ready to build and start Pika Backup via the "Run" button or by pressing *Ctrl+F5*.

## Peculiarities

Binding to objects defined in `.ui`-files are auto generated via

```sh
$ ./build-aux/generate-ui-bindings.py
```

You have to execute this script after adding, changing or removing ids from `.ui`-files. The bindings can be found in `src/ui/builder.rs`. Using only those bindings allows to catch all errors in accessing builder elements on compile time.

## Debugging

The STDIO log level can be adjusted by setting the `RUST_LOG` to debug or trace. For example

```sh
$ RUST_LOG=trace cargo run
```

There is also the option to log to syslog

```sh
$ cargo run --syslog
```

However, flatpaks do not support syslog by default.
