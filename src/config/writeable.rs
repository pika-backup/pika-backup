use super::{ConfigType, Loadable};

use crate::config;
use arc_swap::ArcSwap;

#[derive(Default)]
pub struct Writeable<C> {
    pub current_config: C,
    pub written_config: C,
}

impl<C> std::ops::Deref for Writeable<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.current_config
    }
}

impl<C> std::ops::DerefMut for Writeable<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current_config
    }
}

impl<C: Loadable + Clone> Loadable for Writeable<C> {
    fn from_file() -> Result<Self, std::io::Error> {
        let config = C::from_file()?;
        Ok(Self {
            current_config: config.clone(),
            written_config: config,
        })
    }
}

impl<T, C: crate::utils::LookupConfigId<Item = T>> crate::utils::LookupConfigId for Writeable<C> {
    type Item = T;

    fn try_get_mut(
        &mut self,
        key: &config::ConfigId,
    ) -> Result<&mut T, config::error::BackupNotFound> {
        self.current_config.try_get_mut(key)
    }

    fn try_get(&self, key: &config::ConfigId) -> Result<&T, config::error::BackupNotFound> {
        self.current_config.try_get(key)
    }
}

impl<C> Writeable<C>
where
    C: ConfigType
        + super::Loadable
        + std::cmp::PartialEq
        + serde::Serialize
        + Send
        + Default
        + Clone
        + 'static,
{
    pub fn is_changed(&self) -> bool {
        self.current_config != self.written_config
    }

    pub async fn write_file(&mut self) -> Result<(), std::io::Error> {
        let path = C::path();
        debug!("Request to rewrite {:?}", path);

        if self.is_changed() {
            let dir = path.parent().map(|x| x.to_path_buf()).unwrap_or_default();

            std::fs::create_dir_all(&dir)?;

            let current_config = self.current_config.clone();
            smol::unblock(move || {
                let config_file = tempfile::NamedTempFile::new_in(dir)?;
                debug!("Writing new file to {:?}", config_file);

                serde_json::ser::to_writer_pretty(&config_file, &current_config)?;

                debug!("Moving new file to {:?}", path);
                config_file.persist(&path)?;

                Ok::<(), std::io::Error>(())
            })
            .await?;

            self.written_config = self.current_config.clone();
        } else {
            debug!("Not rewriting because data is unchanged.");
        }

        Ok(())
    }
}

pub(crate) trait ArcSwapWriteable {
    async fn write_file(&self) -> Result<(), std::io::Error>;
}

impl<C> ArcSwapWriteable for ArcSwap<Writeable<C>>
where
    C: ConfigType
        + super::Loadable
        + std::cmp::PartialEq
        + serde::Serialize
        + Send
        + Default
        + Clone
        + 'static,
{
    /// Write the file asynchronously
    ///
    /// After this function has completed there are one of two possible outcomes:
    /// - The file was written successfully, and written_config contains the data that is currently on disk
    /// - An error occurred and the config is unchanged
    async fn write_file(&self) -> Result<(), std::io::Error> {
        let mut cur = self.load();

        // Algorithm from arc_swap::ArcSwapAny::rcu
        loop {
            let mut new = Writeable {
                current_config: cur.current_config.clone(),
                written_config: cur.written_config.clone(),
            };

            // First we try to write the file and store the result
            let result = new.write_file().await;
            let prev = self.compare_and_swap(&*cur, new.into());
            if ***prev == ***cur {
                // Swap was successful
                return result;
            } else {
                // We need to retry
                cur = prev;
            }
        }
    }
}
