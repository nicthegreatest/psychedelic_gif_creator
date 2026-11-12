subdir('icons')

data_conf = configuration_data()
data_conf.set('app_id', app_id)
data_conf.set('name_suffix', name_suffix)

# No translations yet (English only), so plain configure_file instead of i18n.merge_file.
desktop_file = configure_file(
  input: base_id + '.desktop.in',
  output: app_id + '.desktop',
  configuration: data_conf,
  install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils,
    args: [desktop_file]
  )
endif

appstream_file = configure_file(
  input: base_id + '.metainfo.xml.in',
  output: app_id + '.metainfo.xml',
  configuration: data_conf,
  install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appdata file', appstreamcli,
    args: [
      'validate', '--no-net', '--explain',
      appstream_file,
    ]
  )
endif

install_data(base_id + '.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)
gnome.compile_schemas()

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()]
  )
endif
