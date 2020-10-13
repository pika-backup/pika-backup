project(
    run_command(
        find_program(join_paths(meson.source_root(), 'build-aux/meson-cargo-manifest.py')),
        'package', 'name'
    ).stdout(),
    'rust',
    version: run_command(
        find_program(join_paths(meson.source_root(), 'build-aux/meson-cargo-manifest.py')),
        'package', 'version'
    ).stdout()
)

dependency('dbus-1')
dependency('gdk-pixbuf-2.0')
dependency('gio-2.0')
dependency('glib-2.0')
dependency('gtk+-3.0')
dependency('libhandy-1')

meson.add_dist_script('build-aux/meson-dist-vendor.sh')

subdir('data')
subdir('src')
