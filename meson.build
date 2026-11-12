project('gif-creator',
        'rust',
          version: '1.0.0',
    meson_version: '>= 0.59.0',
          license: 'GPL-3.0-or-later',
)

dependency('gtk4', version: '>= 4.16.0')
dependency('libadwaita-1', version: '>= 1.8.0')

gnome = import('gnome')

if get_option('profile') == 'development'
  profile = 'Devel'
  app_id_suffix = '.Devel'
  name_suffix = ' (Devel)'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD', check: false).stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-' + vcs_tag
  endif
else
  profile = ''
  app_id_suffix = ''
  name_suffix = ''
  version_suffix = ''
endif

base_id = 'dev.nic.GifCreator'
app_id = base_id + app_id_suffix

sources = files(
  'Cargo.lock',
  'Cargo.toml',
  'crates/gifcore/Cargo.toml',
  'crates/gifcore/src/lib.rs',
)

subdir('data')
subdir('src')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)

meson.add_dist_script(
  'build-aux/dist-vendor.sh',
  meson.project_build_root() / 'meson-dist' / meson.project_name() + '-' + meson.project_version(),
  meson.project_source_root()
)
