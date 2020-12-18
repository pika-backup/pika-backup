cargo_options = []
cargo_options += [ '--manifest-path', meson.source_root() / 'Cargo.toml' ]
cargo_options += [ '--target-dir', meson.build_root() / 'src' ]

if get_option('profile') == 'release'
  cargo_options += [ '--release' ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_env = [ 'CARGO_HOME=' + meson.build_root() / 'cargo-home',
  'LOCALEDIR=' + get_option('prefix') / get_option('localedir'),
  'APPLICATION_ID_SUFFIX=' + get_option('app_id_suffix')
]

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'release',
  console: true,
  command: [ 'env',
    cargo_env,
    'cargo', 'build',
    cargo_options
  ]
)

binary_file = 'src' / rust_target / meson.project_name()
copy_binary = custom_target(
  'copy-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: get_option('bindir'),
  output: meson.project_name(),
  command: [ 'cp', binary_file, '@OUTPUT@' ]
)

cargo = find_program('cargo')

test('cargo clippy', cargo, args: ['clippy'], is_parallel: false, timeout: 1800)
test('cargo test',
  cargo,
  args: [
    'test',
    cargo_options
  ],
  is_parallel: false,
  timeout: 1800
)
