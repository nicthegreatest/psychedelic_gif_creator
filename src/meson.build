conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version() + version_suffix)
conf.set_quoted('NAME_SUFFIX', name_suffix)
conf.set_quoted('APP_ID', app_id)
conf.set_quoted('SCHEMA_ID', base_id)
conf.set_quoted('PROFILE', profile)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  join_paths(meson.project_build_root(), 'src', 'config.rs'),
  join_paths(meson.project_source_root(), 'src', 'config.rs'),
  check: true
)

sources += files(
  'application.rs',
  'config.rs',
  'main.rs',
  'window.rs',
)

cargo_script = find_program(join_paths(meson.project_source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTPUT@',
    app_id_suffix,
    meson.project_name(),
  ]
)

cargo = find_program('cargo')
test('cargo test', cargo,
  args: ['test', '--workspace', '--locked', '--manifest-path', meson.project_source_root() / 'Cargo.toml'],
  # CARGO_HOME comes from the environment, as in cargo.sh.
  env: {'CARGO_TARGET_DIR': meson.project_build_root() / 'target'},
  timeout: 900,
)
