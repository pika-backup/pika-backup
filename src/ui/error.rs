use crate::prelude::*;
use crate::ui;
use gtk::prelude::*;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Message(err: crate::ui::utils::Message) { from() }
        UserAborted { from (UserAborted) }

    }
}

#[derive(Debug)]
pub struct UserAborted {}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct Handler<W: IsA<gtk::Window> + IsA<gtk::Widget>> {
    transient_for: Option<W>,
    auto_visibility: Option<W>,
}

impl<W: IsA<gtk::Window> + IsA<gtk::Widget>> Handler<W> {
    pub fn new() -> Self {
        Self {
            transient_for: None,
            auto_visibility: None,
        }
    }

    pub fn error_transient_for(mut self, window: W) -> Self {
        self.transient_for = Some(window);
        self
    }

    pub fn dialog_auto_visibility(mut self, window: W) -> Self {
        self.auto_visibility = Some(window);
        self
    }

    pub fn spawn<F: std::future::Future<Output = Result<()>> + 'static>(&self, f: F) {
        let transient_for = self.transient_for.clone();
        let auto_visibility = self.auto_visibility.clone();

        glib::MainContext::default().spawn_local(async move {
            match f.await {
                Err(Error::Message(err)) => {
                    if let Some(auto_visibility) = auto_visibility {
                        auto_visibility.show();
                        ui::page_pending::back();
                    }

                    if let Some(transient_for) = transient_for {
                        err.show_transient_for(&transient_for);
                    } else {
                        err.show();
                    }
                }
                Err(Error::UserAborted) => {
                    if let Some(auto_visibility) = auto_visibility {
                        ui::page_pending::back();
                        auto_visibility.show();
                    }
                }
                Ok(()) => {
                    if let Some(auto_visibility) = auto_visibility {
                        auto_visibility.close();
                    }
                }
            }
        });
    }
}
