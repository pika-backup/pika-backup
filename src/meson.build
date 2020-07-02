sources = [
    'main.rs',
]

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))

bin = meson.project_name()

cargo_options = '--release'
rust_target = 'release'
if get_option('profile') == 'dev'
  cargo_options = ''
  rust_target = 'debug'
endif

cargo_target_dir = join_paths(meson.build_root(), 'target')

cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: bin,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.source_root(),
    cargo_target_dir,
    cargo_options,
    join_paths(meson.build_root(), 'target', rust_target, bin),
    '@OUTPUT@',
  ]
)

cargo = find_program('cargo')

test('cargo fmt', cargo, args: ['fmt', '--', '--check'])
test('cargo clippy', cargo, args: ['clippy'], is_parallel: false, timeout: 1800)
test('cargo test',
  cargo,
  args: [
    'test',
    '--manifest-path', join_paths(meson.source_root(), 'Cargo.toml'),
    '--target-dir', cargo_target_dir,
    cargo_options
  ],
  is_parallel: false,
  timeout: 1800
)
