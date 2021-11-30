pub trait Loadable: Sized {
    fn from_file() -> Result<Self, std::io::Error>;
}

impl<C: ConfigType + serde::de::DeserializeOwned + Default> Loadable for C {
    fn from_file() -> Result<Self, std::io::Error> {
        let path = Self::path();
        let file = std::fs::File::open(&path);

        info!("Loading file {:?}", path);

        if let Err(err) = &file {
            if matches!(err.kind(), std::io::ErrorKind::NotFound) {
                info!("File not found. Using default value.");
                return Ok(Default::default());
            }
        }

        Ok(serde_json::de::from_reader(file?)?)
    }
}

pub trait ConfigType {
    fn path() -> std::path::PathBuf;
}
