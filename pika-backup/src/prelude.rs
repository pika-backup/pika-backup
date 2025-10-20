use arc_swap::ArcSwap;
pub use common::borg::prelude::*;
use common::config::ArcSwapWriteable;
pub use glib::clone;

pub use crate::error::{
    CombinedResult, CombinedResultExt, CombinedToError, Error, ErrorToMessage, Handler, Message,
    Result,
};
pub use crate::globals::*;
pub use crate::status::QuitGuard;
pub use crate::utils::ext::*;
pub use crate::utils::{Logable, LookupActiveConfigId, SummarizeOperations};
pub use crate::widget::{DialogPage, DialogPagePropertiesExt, SpinnerPagePropertiesExt};

pub trait ArcSwapResultExt<T> {
    // Update the inner value with the provided closure
    async fn try_update<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()>;
}

impl<T> ArcSwapResultExt<T> for ArcSwap<T>
where
    T: Clone,
{
    async fn try_update<F: Fn(&mut T) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());
        self.rcu(|current| {
            let mut new = T::clone(current);
            result = updater(&mut new);
            new
        });

        result
    }
}

impl<C> ArcSwapResultExt<C> for ArcSwap<common::config::Writeable<C>>
where
    C: common::config::ConfigType
        + common::config::Loadable
        + std::cmp::PartialEq
        + serde::Serialize
        + Send
        + Default
        + Clone
        + 'static,
{
    /// Update the inner value with the provided closure. Saves the writeable
    /// afterwards.
    async fn try_update<F: Fn(&mut C) -> Result<()>>(&self, updater: F) -> Result<()> {
        let mut result = Ok(());

        self.rcu(|current| {
            let mut new = C::clone(&current.current_config);
            result = updater(&mut new);

            common::config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });

        self.write_file()
            .await
            .err_to_msg(gettext("Could not write configuration file."))?;

        result
    }
}

#[allow(dead_code)]
pub trait HasAppWindow {
    fn app_window(&self) -> super::widget::AppWindow;

    fn app(&self) -> super::App {
        self.app_window().app()
    }
}
