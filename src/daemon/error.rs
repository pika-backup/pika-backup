use crate::daemon::prelude::*;
use gio::prelude::*;

use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Loggable<T> {
    fn handle<D: Display>(self, msg: D) -> Option<T>;
}

impl<T, E: Display> Loggable<T> for std::result::Result<T, E> {
    fn handle<D: Display>(self, msg: D) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                error!("Error: {}: {}", msg, err);

                let notification = gio::Notification::new(&msg.to_string());
                notification.set_body(Some(&err.to_string()));
                gio_app().send_notification(None, &notification);

                None
            }
        }
    }
}
