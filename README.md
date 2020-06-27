# Pika Backup

Simple backups based on borg

## Build

You need [Rust](https://rustup.rs/) to build Pika

```sh
$ apt install libgtk-3-dev
$ cargo run --release
```

or you can use flatpak

```sh
$ ./build-aux/flatpak-build.bash
```

## Debug

Run with `RUST_LOG` set to debug or trace. For example

```sh
$ RUST_LOG=trace cargo run
```