pub use crate::prelude::*;
pub use crate::ui::error::{Error, ErrorToMessage, Handler, Message, Result, UserAborted};
pub use crate::ui::globals::*;
pub use crate::ui::utils::ext::*;
pub use crate::ui::utils::BackupMapActive;

pub use gettextrs::gettext;

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}

use arc_swap::ArcSwap;

pub trait ArcSwapResultExt<T> {
    fn update_result<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()>;
}

impl<T> ArcSwapResultExt<T> for ArcSwap<T>
where
    T: Clone,
{
    fn update_result<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());
        self.rcu(|current| {
            let mut new = T::clone(current);
            result = updater(&mut new);
            new
        });

        result
    }
}
