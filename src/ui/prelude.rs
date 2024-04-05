pub use crate::borg::prelude::*;
use crate::config::ArcSwapWriteable;
pub use crate::ui::error::{
    CombinedResult, CombinedResultExt, CombinedToError, Error, ErrorToMessage, Handler, Message,
    Result,
};
pub use crate::ui::globals::*;
pub use crate::ui::status::QuitGuard;
pub use crate::ui::utils::ext::*;
pub use crate::ui::utils::{Logable, LookupActiveConfigId, SummarizeOperations};

use arc_swap::ArcSwap;
pub use glib::clone;

use super::widget::AppWindow;
use super::App;

pub trait ArcSwapResultExt<T> {
    // Update the inner value with the provided closure
    fn try_update<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()>;
}

impl<T> ArcSwapResultExt<T> for ArcSwap<T>
where
    T: Clone,
{
    fn try_update<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());
        self.rcu(|current| {
            let mut new = T::clone(current);
            result = updater(&mut new);
            new
        });

        result
    }
}

impl<C> ArcSwapResultExt<C> for ArcSwap<crate::config::Writeable<C>>
where
    C: crate::config::ConfigType
        + crate::config::Loadable
        + std::cmp::PartialEq
        + serde::Serialize
        + Default
        + Clone,
{
    /// Update the inner value with the provided closure. Saves the writeable afterwards.
    fn try_update<F: Fn(&mut C) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());

        self.rcu(|current| {
            let mut new = C::clone(&current.current_config);
            result = updater(&mut new);

            crate::config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });

        self.write_file()
            .err_to_msg(gettext("Could not write configuration file."))?;

        result
    }
}

pub trait HasAppWindow {
    fn app_window(&self) -> AppWindow;

    fn app(&self) -> App {
        self.app_window().app()
    }
}
