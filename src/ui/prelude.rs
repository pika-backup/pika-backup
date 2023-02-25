pub use crate::borg::prelude::*;
pub use crate::prelude::*;
pub use crate::ui::error::{
    CombinedResult, CombinedResultExt, CombinedToError, Error, ErrorToMessage, Handler, Message,
    Result, UserCanceled,
};
pub use crate::ui::globals::*;
pub use crate::ui::utils::ext::*;
pub use crate::ui::utils::{Logable, LookupActiveConfigId, SummarizeOperations};

use arc_swap::ArcSwap;
pub use glib::clone;

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

impl<C: Clone> ArcSwapResultExt<C> for ArcSwap<crate::config::Writeable<C>> {
    fn update_result<F: Fn(&mut C) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());
        self.rcu(|current| {
            let mut new = C::clone(&current.current_config);
            result = updater(&mut new);

            crate::config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });

        result
    }
}
