use chrono::prelude::*;
use gtk::prelude::*;

use gtk::NativeDialog;
use gtk::ResponseType;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;

pub trait NativeDialogExtManual {
    fn run_future<'a>(&'a self) -> Pin<Box<dyn Future<Output = ResponseType> + 'a>>;
}

impl<O: IsA<NativeDialog>> NativeDialogExtManual for O {
    fn run_future<'a>(&'a self) -> Pin<Box<dyn Future<Output = ResponseType> + 'a>> {
        Box::pin(async move {
            let (sender, receiver) = futures::channel::oneshot::channel();

            let sender = Cell::new(Some(sender));

            let response_handler = self.connect_response(move |_, response_type| {
                if let Some(m) = sender.replace(None) {
                    let _result = m.send(response_type);
                }
            });

            self.show();

            if let Ok(response) = receiver.await {
                if response != ResponseType::DeleteEvent {
                    self.disconnect(response_handler);
                }
                response
            } else {
                ResponseType::None
            }
        })
    }
}

pub trait CronoExt {
    fn to_locale(&self) -> Option<String>;
}

impl CronoExt for NaiveDateTime {
    fn to_locale(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(&self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        Some(gdt.ok()?.format("%c").ok()?.to_string())
    }
}

pub trait WidgetExtExt {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: WidgetExt> WidgetExtExt for W {
    fn add_css_class(&self, class: &str) {
        self.style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.style_context().remove_class(class);
    }
}
