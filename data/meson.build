datadir = get_option('prefix') / get_option('datadir')

application_id = 'de.hemio.PikaPileBackup'
# Read more https://mesonbuild.com/Reference-manual.html#install_data

install_data(
    '@0@.desktop'.format(application_id),
    install_dir: datadir / 'applications'
)

install_data(
    '@0@.metainfo.xml'.format(application_id),
    install_dir: datadir / 'metainfo'
)

install_data(
    '@0@.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)
