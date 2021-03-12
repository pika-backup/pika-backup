use chrono::prelude::*;
use gtk::prelude::*;

#[async_trait(?Send)]
pub trait DialogExtFuture {
    async fn run_future(&self) -> gtk::ResponseType;
}

#[async_trait(?Send)]
impl<O: IsA<gtk::Dialog> + IsA<gtk::Widget>> DialogExtFuture for O {
    async fn run_future(&self) -> gtk::ResponseType {
        let (sender, receiver) = futures::channel::oneshot::channel();

        let sender = std::cell::Cell::new(Some(sender));

        let response_handler = self.connect_response(move |_, response_type| {
            if let Some(m) = sender.replace(None) {
                let _result = m.send(response_type);
            }
        });

        let delete_handler = self.connect_delete_event(|_, _| Inhibit(true));

        self.show();

        let result = receiver.await.unwrap_or(gtk::ResponseType::None);
        self.disconnect(response_handler);
        self.disconnect(delete_handler);

        result
    }
}

pub trait CronoExt {
    fn to_glib(&self) -> glib::DateTime;
    fn to_locale(&self) -> String;
}

impl CronoExt for NaiveDateTime {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp())
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap_or_else(|| self.format("%c").to_string())
    }
}

impl CronoExt for DateTime<Local> {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp())
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap_or_else(|| self.format("%c").to_string())
    }
}

pub trait WidgetExtExt {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: gtk::WidgetExt> WidgetExtExt for W {
    fn add_css_class(&self, class: &str) {
        self.get_style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.get_style_context().remove_class(class);
    }
}
