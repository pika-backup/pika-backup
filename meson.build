project(
    run_command(
        find_program(join_paths(meson.source_root(), 'build-aux/manifest.py')),
        'package', 'name'
    ).stdout(),
    'rust',
    version: run_command(
        find_program(join_paths(meson.source_root(), 'build-aux/manifest.py')),
        'package', 'version'
    ).stdout()
)

dependency('glib-2.0')
dependency('gio-2.0')
dependency('gtk+-3.0')

meson.add_dist_script('build-aux/dist-vendor.sh')

subdir('data')
subdir('src')
