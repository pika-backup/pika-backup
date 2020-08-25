# Pika Backup

<p>
      Doing backups the easy way. Plugin your USB drive and let the Pika do the rest for you.
    </p>
    <p><b>Features</b></p>
    <ul>
      <li>Setup new backup repositories or uses existing ones</li>
      <li>Create backups locally and remote</li>
      <li>Save time and disk space because Pika Backup does not need to copy known data again</li>
      <li>Encrypt your backups</li>
      <li>List created archives and browse through their contents</li>
      <li>Recover files or folders via your file browser</li>
    </ul>
    <p>
      Pika Backup is powered by the well-tested borg-backup software.
    </p>
    <p><b>Limitations</b></p>
    <p>
      Currently, scheduled backups support are not supported. Excluding files from a backup via regular expressions and alike is not implemented yet. Remote backup locations must support SSH and need to have a borg-backup binary installed.
    </p>

![Pika Backup Setup](/uploads/596347a2e99be37c3f8a035b75cea8ea/pika-pile-1.png)

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

**Debug**

Run with `RUST_LOG` set to debug or trace. For example

```sh
$ RUST_LOG=trace cargo run
```