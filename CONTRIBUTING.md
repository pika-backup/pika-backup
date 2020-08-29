# Contributing to Pika Backup

Contributions of all kind and with all levels of experience are very welcome. Please note that the [GNOME Code of Conduct](https://wiki.gnome.org/Foundation/CodeOfConduct) applies to this project.

## Peculiarities

Binding to objects defined in `.ui`-files are auto generated via

```
$ ./build-aux/generate_ui_bindings.py
```

You have to execute this script after adding, changing or removing ids from `.ui`-files. The bindings can be found in `src/ui/builder.rs`. Using only those bindings allows to catch all errors in accessing builder elements on compile time.

## Debugging

The STDIO log level can be adjusted by setting the `RUST_LOG` to debug or trace. For example

```
$ RUST_LOG=trace cargo run
```

There is also the option to log to syslog

```
$ cargo run --syslog
```

However, flatpaks do not support syslog by default.