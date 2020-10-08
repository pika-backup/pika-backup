datadir = get_option('prefix') / get_option('datadir')

application_id = run_command('cat', 'APPLICATION_ID').stdout()

install_data(
    'app.desktop',
    rename: '@0@.desktop'.format(application_id),
    install_dir: datadir / 'applications'
)

meson.add_install_script(
    find_program('update-desktop-database').path(),
    datadir / 'applications'
)

install_data(
    'app.metainfo.xml',
    rename: '@0@.metainfo.xml'.format(application_id),
    install_dir: datadir / 'metainfo'
)

install_data(
    'app-symbolic.svg',
    rename: '@0@-symbolic.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

app_icon = 'app'
if get_option('profile') == 'dev'
  app_icon += '.Devel'
endif

install_data(
    '@0@.svg'.format(app_icon),
    rename: '@0@.svg'.format(application_id),
    install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

meson.add_install_script(
    find_program('gtk-update-icon-cache').path(),
    '-qtf',
    datadir / 'icons' / 'hicolor'
)
