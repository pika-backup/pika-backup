sources = [
    'main.rs',
]

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))

bin = meson.project_name()

rust_profile = '--release'
rust_target = 'release'
if get_option('profile') == 'dev'
  rust_profile = ''
  rust_target = 'debug'
endif

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
    join_paths(meson.build_root(), 'target'),
    rust_profile,
    join_paths(meson.build_root(), 'target', rust_target, bin),
    '@OUTPUT@',
  ]
)

cargo = find_program('cargo')

test('cargo fmt', cargo, args: ['fmt', '--check'])
test('cargo clippy', cargo, args: ['clippy'])
test('cargo test', cargo, args: ['test'])
