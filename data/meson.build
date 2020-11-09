datadir = get_option('prefix') / get_option('datadir')
podir = '../po'

application_id = run_command('cat', 'APPLICATION_ID').stdout() + get_option('app_id_suffix')

i18n.merge_file(
    '.desktop',
    type: 'desktop',
    input: configure_file(
        input: 'app.desktop.in',
        output: 'tmp.app.desktop',
        configuration: { 'app_id': application_id }
    ),
    output: '@0@.desktop'.format(application_id),
    po_dir: podir,
    install: true,
    install_dir: datadir / 'applications'
)

meson.add_install_script(
    find_program('update-desktop-database').path(),
    datadir / 'applications'
)

i18n.merge_file(
    '.metainfo.xml',
    input: configure_file(
        input: 'app.metainfo.xml.in',
        output: 'tmp.app.metainfo.xml',
        configuration: { 'app_id': application_id }
    ),
    output: '@0@.metainfo.xml'.format(application_id),
    po_dir: podir,
    install: true,
    install_dir: datadir / 'metainfo'
)

install_data(
    'app-symbolic.svg',
    rename: '@0@-symbolic.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

install_data(
    '@0@.svg'.format('app' + get_option('app_id_suffix')),
    rename: '@0@.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

meson.add_install_script(
    find_program('gtk-update-icon-cache').path(),
    '-qtf',
    datadir / 'icons' / 'hicolor'
)
