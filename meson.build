project(
    run_command(
        find_program(meson.source_root() / 'build-aux/meson-cargo-manifest.py'),
        'package', 'name'
    ).stdout(),
    'rust',
    version: run_command(
        find_program(meson.source_root() / 'build-aux/meson-cargo-manifest.py'),
        'package', 'version'
    ).stdout()
)

i18n = import ('i18n')

dependency('dbus-1')
dependency('gtk+-3.0')
dependency('libhandy-1')

bindir = get_option('prefix') / get_option('bindir')
etcdir = get_option('prefix') / get_option('sysconfdir')
usrdir = get_option('prefix') / get_option('datadir')
message(etcdir)

meson.add_dist_script('build-aux/meson-dist-vendor.sh')

subdir('data')
subdir('po')
subdir('src')
