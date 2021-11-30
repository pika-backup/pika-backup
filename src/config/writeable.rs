use super::{ConfigType, Loadable};

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

impl<C: Loadable + Clone> Loadable for Writeable<C> {
    fn from_file() -> Result<Self, std::io::Error> {
        let config = C::from_file()?;
        Ok(Writeable {
            current_config: config.clone(),
            written_config: config,
        })
    }
}

impl<
        C: ConfigType + super::Loadable + std::cmp::PartialEq + serde::Serialize + Default + Clone,
    > Writeable<C>
{
    pub fn is_changed(&self) -> bool {
        self.current_config != self.written_config
    }

    pub fn write_file(&self) -> Result<(), std::io::Error> {
        let path = C::path();
        debug!("Request to rewrite {:?}", path);

        if self.is_changed() {
            let dir = path.parent().map(|x| x.to_path_buf()).unwrap_or_default();
            let config_file = tempfile::NamedTempFile::new_in(dir)?;
            debug!("Writing new file to {:?}", config_file);
            serde_json::ser::to_writer_pretty(&config_file, &self.current_config)?;

            debug!("Moving new file to {:?}", path);
            config_file.persist(&path)?;
        } else {
            debug!("Not rewriting because data is unchanged.");
        }

        Ok(())
    }
}
