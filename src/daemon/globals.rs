use std::rc::Rc;

thread_local!(
    static GIO_APPLICATION: Rc<gio::Application> = Rc::new(gio::Application::new(
        Some(&crate::daemon_app_id()),
        gio::ApplicationFlags::IS_SERVICE,
    ));
);

pub fn gio_app() -> Rc<gio::Application> {
    GIO_APPLICATION.with(|x| x.clone())
}
