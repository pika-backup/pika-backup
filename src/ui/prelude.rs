pub use crate::borg::prelude::*;
pub use crate::prelude::*;
pub use crate::ui::error::{
    CombinedResult, CombinedToError, Error, ErrorToMessage, Handler, Message, Result, UserCanceled,
};
pub use crate::ui::globals::*;
pub use crate::ui::utils::ext::*;
pub use crate::ui::utils::LookupActiveConfigId;

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
