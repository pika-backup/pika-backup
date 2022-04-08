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

    fn get_result_mut(
        &mut self,
        key: &config::ConfigId,
    ) -> Result<&mut T, config::error::BackupNotFound> {
        self.current_config.get_result_mut(key)
    }

    fn get_result(&self, key: &config::ConfigId) -> Result<&T, config::error::BackupNotFound> {
        self.current_config.get_result(key)
    }
}

impl<C> Writeable<C>
where
    C: ConfigType + super::Loadable + std::cmp::PartialEq + serde::Serialize + Default + Clone,
{
    pub fn is_changed(&self) -> bool {
        self.current_config != self.written_config
    }

    pub fn write_file(&mut self) -> Result<(), std::io::Error> {
        let path = C::path();
        debug!("Request to rewrite {:?}", path);

        if self.is_changed() {
            let dir = path.parent().map(|x| x.to_path_buf()).unwrap_or_default();
            let config_file = tempfile::NamedTempFile::new_in(dir)?;
            debug!("Writing new file to {:?}", config_file);
            serde_json::ser::to_writer_pretty(&config_file, &self.current_config)?;

            debug!("Moving new file to {:?}", path);
            config_file.persist(&path)?;
            self.written_config = self.current_config.clone();
        } else {
            debug!("Not rewriting because data is unchanged.");
        }

        Ok(())
    }
}

pub trait ArcSwapWriteable {
    fn write_file(&self) -> Result<(), std::io::Error>;
}

impl<C> ArcSwapWriteable for ArcSwap<Writeable<C>>
where
    C: ConfigType + super::Loadable + std::cmp::PartialEq + serde::Serialize + Default + Clone,
{
    fn write_file(&self) -> Result<(), std::io::Error> {
        let mut cell = once_cell::sync::OnceCell::new();

        if self.load().is_changed() {
            self.rcu(|current| {
                let mut new = Writeable {
                    current_config: current.current_config.clone(),
                    written_config: current.written_config.clone(),
                };

                let _set = cell.set(new.write_file());

                new
            });
        } else {
            let _set = cell.set(Ok(()));
        }

        cell.take().unwrap()
    }
}
