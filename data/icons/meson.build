scalable_dir = 'hicolor' / 'scalable' / 'apps'
symbolic_dir = 'hicolor' / 'symbolic' / 'apps'

# Icon from gif_creator_v2 (resources/psychedelic-gif-creator.svg).
install_data(
  scalable_dir / base_id + '.svg',
  install_dir: get_option('datadir') / 'icons' / scalable_dir,
  rename: app_id + '.svg',
)

# One-colour version of the same tunnel motif, for GNOME's symbolic icon slots.
install_data(
  symbolic_dir / base_id + '-symbolic.svg',
  install_dir: get_option('datadir') / 'icons' / symbolic_dir,
  rename: app_id + '-symbolic.svg',
)
