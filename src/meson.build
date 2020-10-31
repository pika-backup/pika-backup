sources = [
    'main.rs',
]

cargo_script = find_program(meson.source_root() / 'build-aux/meson-cargo.sh')

bin = meson.project_name()

cargo_options = '--release'
rust_target = 'release'
if get_option('profile') == 'dev'
  cargo_options = ''
  rust_target = 'debug'
endif

cargo_target_dir = join_paths(meson.build_root(), 'target')

env = [
  [ 'MESON_SOURCE_ROOT', meson.source_root() ],
  [ 'CARGO_TARGET_DIR', cargo_target_dir ],
  [ 'CARGO_OPTIONS', cargo_options ],
  [ 'CARGO_OUTPUT', meson.build_root() / 'target' / rust_target / bin ],
  [ 'OUTPUT', '@OUTPUT@' ],
  [ 'LOCALEDIR', get_option('prefix') / get_option('localedir') ]
]

command = [ cargo_script ]

foreach p : env
  command += 'export @0@="@1@";'.format(p[0], p[1])
endforeach

cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: bin,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: command
)

cargo = find_program('cargo')

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
