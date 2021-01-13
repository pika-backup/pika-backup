pub use crate::ui::utils::BackupMap;
pub use gettextrs::gettext;

use arc_swap::ArcSwap;

use chrono::prelude::*;
use gtk::prelude::*;

pub fn spawn_local<F: std::future::Future<Output = ()> + 'static>(f: F) {
    glib::MainContext::default().spawn_local(f);
}

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

pub trait Humanize {
    fn humanize(self) -> String;
}

impl Humanize for std::time::Duration {
    fn humanize(self) -> String {
        if let Ok(duration) = chrono::Duration::from_std(self) {
            duration.humanize()
        } else {
            String::from("Too large")
        }
    }
}

impl Humanize for chrono::Duration {
    fn humanize(self) -> String {
        chrono_humanize::HumanTime::from(self).to_string()
    }
}

pub trait ArcSwapAdditions<T> {
    fn update<F: Fn(&mut T)>(&self, updater: F);
    fn get(&self) -> T;
}

impl<T> ArcSwapAdditions<T> for ArcSwap<T>
where
    T: Clone,
{
    fn update<F: Fn(&mut T)>(&self, updater: F) {
        self.rcu(|current| {
            let mut new = T::clone(current);
            updater(&mut new);
            new
        });
    }

    fn get(&self) -> T {
        T::clone(&self.load_full())
    }
}

impl<T> ArcSwapAdditions<T> for once_cell::sync::Lazy<ArcSwap<T>>
where
    T: Clone,
{
    fn update<F: Fn(&mut T)>(&self, updater: F) {
        (**self).rcu(|current| {
            let mut new = T::clone(current);
            updater(&mut new);
            new
        });
    }

    fn get(&self) -> T {
        T::clone(&(**self).load_full())
    }
}

pub trait CronoAdditions {
    fn to_glib(&self) -> glib::DateTime;
    fn to_locale(&self) -> String;
}

impl CronoAdditions for NaiveDateTime {
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

impl CronoAdditions for DateTime<Local> {
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

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}

pub trait WidgetEnh {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: gtk::WidgetExt> WidgetEnh for W {
    fn add_css_class(&self, class: &str) {
        self.get_style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.get_style_context().remove_class(class);
    }
}
