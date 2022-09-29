cargo_bin = find_program('cargo')

target_dir = meson.project_build_root() / 'src'

if get_option('profile') == 'release'
  rust_target = 'release'
  profile = 'release'
else
  rust_target = 'debug'
  profile = 'dev'
endif

cargo_options = [
  '--target-dir', target_dir,
  '--profile', profile,
]

cargo_env = {
  'CARGO_HOME': meson.project_build_root() / 'cargo',
  'LOCALEDIR': get_option('prefix') / get_option('localedir'),
  'APPLICATION_ID_SUFFIX': get_option('app_id_suffix'),
  'GIT_DESCRIBE': run_command(
      'git',
      '-C', meson.project_source_root(),
      'rev-parse', '--short', 'HEAD',
      check: false
    ).stdout().strip(),
}

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: rust_target,
  console: true,
  env: cargo_env,
  command: [
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
  output: meson.project_name() + '-monitor',
  command: [
    'cp',
    target_dir / rust_target / meson.project_name() + '-monitor',
    '@OUTPUT@',
  ],
)

cargo_doc = custom_target(
  'cargo-doc',
  build_by_default: false,
  build_always_stale: true,
  output: 'doc',
  console: true,
  env: cargo_env,
  command: [
    cargo_bin, 'doc',
    cargo_options + ['--no-deps', '--document-private-items'],
  ],
)

test(
  'cargo-test',
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
