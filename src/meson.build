cargo_bin = find_program('cargo')

target_dir = meson.build_root() / 'src'

cargo_options = [
  '--target-dir', target_dir,
  '--manifest-path', meson.source_root() / 'Cargo.toml',
]

if get_option('profile') == 'release'
  cargo_options += [ '--release' ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_env = [
  'CARGO_HOME=' + meson.build_root() / 'cargo-home',
  'LOCALEDIR=' + get_option('prefix') / get_option('localedir'),
  'APPLICATION_ID_SUFFIX=' + get_option('app_id_suffix'),
]

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: rust_target,
  console: true,
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_options,
  ],
)

copy_binary = custom_target(
  'cp-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: bindir,
  output: meson.project_name(),
  command: [
    'cp',
    target_dir / rust_target / meson.project_name(),
    '@OUTPUT@',
  ],
)

copy_daemon_binary = custom_target(
  'cp-daemon-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: bindir,
  output: meson.project_name() + 'd',
  command: [
    'cp',
    target_dir / rust_target / meson.project_name() + 'd',
    '@OUTPUT@',
  ],
)

test('cargo-test',
  cargo_bin,
  args: [
    'test',
    cargo_options,
    '--',
    '--test-threads=1',
  ],
  is_parallel: false,
  timeout: 1800,
)
