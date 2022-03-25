podir = '../po'

plain_id = run_command('cat', 'APPLICATION_ID', check: true).stdout()
application_id = plain_id + get_option('app_id_suffix')
daemon_id = plain_id + '.Monitor' + get_option('app_id_suffix')

# Desktop files

i18n.merge_file(
    type: 'desktop',
    input: configure_file(
        input: 'app.desktop.in',
        output: 'tmp.app.desktop',
        configuration: { 'app_id': application_id }
    ),
    output: '@0@.desktop'.format(application_id),
    po_dir: podir,
    install: true,
    install_dir: usrdir / 'applications'
)

daemon_desktop_target = i18n.merge_file(
    type: 'desktop',
    input: configure_file(
        input: 'app.Monitor.desktop.in',
        output: 'tmp.app.Monitor.desktop',
        configuration: { 'app_id': application_id }
    ),
    output: '@0@.desktop'.format(daemon_id),
    po_dir: podir,
    install: true,
    install_dir: usrdir / 'applications'
)

# Autostart

install_data(
    daemon_desktop_target[0].full_path(),
    install_dir: etcdir / 'xdg' / 'autostart'
)

# Appdata

i18n.merge_file(
    input: configure_file(
        input: 'app.metainfo.xml.in',
        output: 'tmp.app.metainfo.xml',
        configuration: { 'app_id': application_id }
    ),
    output: '@0@.metainfo.xml'.format(application_id),
    po_dir: podir,
    install: true,
    install_dir: usrdir / 'metainfo'
)

# D-Bus activation

configure_file(
    input: 'app.service.in',
    output: '@0@.service'.format(application_id),
    configuration: { 'app_id': application_id, 'bindir': bindir },
    install_dir: usrdir / 'dbus-1' / 'services'
)

configure_file(
    input: 'app.service.in',
    output: '@0@.Api.service'.format(application_id),
    configuration: { 'app_id': daemon_id, 'bindir': bindir },
    install_dir: usrdir / 'dbus-1' / 'services'
)

configure_file(
    input: 'app.Monitor.service.in',
    output: '@0@.service'.format(daemon_id),
    configuration: { 'app_id': daemon_id, 'bindir': bindir },
    install_dir: usrdir / 'dbus-1' / 'services'
)

# App icons

install_data(
    'app-symbolic.svg',
    rename: '@0@-symbolic.svg'.format(application_id),
    install_dir: usrdir / 'icons' / 'hicolor' / 'symbolic' / 'apps'
)

install_data(
    '@0@.svg'.format('app' + get_option('app_id_suffix')),
    rename: '@0@.svg'.format(application_id),
    install_dir: usrdir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)

# Post install

meson.add_install_script(
    find_program('update-desktop-database').path(),
    usrdir / 'applications'
)

meson.add_install_script(
    find_program('gtk-update-icon-cache').path(),
    '-qtf',
    usrdir / 'icons' / 'hicolor'
)
