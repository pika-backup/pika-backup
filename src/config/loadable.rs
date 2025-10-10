use crate::prelude::*;
use gio::prelude::*;

use arc_swap::ArcSwap;
use std::cell::Cell;
use std::sync::LazyLock;

pub trait Loadable: Sized {
    fn from_file() -> Result<Self, std::io::Error>;
}

impl<C: ConfigType + ConfigVersion + serde::de::DeserializeOwned + Default> Loadable for C {
    fn from_file() -> Result<Self, std::io::Error> {
        let path = Self::path();
        info!("Loading file {:?}", path);

        let file_result = std::fs::File::open(&path);
        if let Err(err) = &file_result
            && matches!(err.kind(), std::io::ErrorKind::NotFound)
        {
            info!("File not found. Using default value.");
            return Ok(Default::default());
        }

        let file = file_result?;

        // Deserialize the file as an untyped json value
        let json: serde_json::Value = serde_json::from_reader(file)?;

        // Check the config version to figure out if we are compatible
        let version = Self::extract_version(&json);
        if Self::version_compatible(version) {
            // Deserialize value as Self
            Ok(serde_json::from_value(json)?)
        } else {
            // The config is incompatible with this app version
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                gettextf(
                    "The loaded configuration file version {} is incompatible with this version of Pika Backup",
                    &[&version.to_string()],
                ),
            ))
        }
    }
}

pub trait TrackChanges: Sized {
    fn update_on_change<H>(
        store: &'static LazyLock<ArcSwap<Self>>,
        error_handler: H,
    ) -> std::io::Result<()>
    where
        H: Fn(std::io::Error) + 'static;
}

thread_local! {
static FILE_MONITORS: Cell<Vec<gio::FileMonitor>> = Default::default();
}

impl<C> TrackChanges for C
where
    C: ConfigType + ConfigVersion + serde::de::DeserializeOwned + Default + Clone,
{
    fn update_on_change<H>(
        store: &'static LazyLock<ArcSwap<Self>>,
        error_handler: H,
    ) -> std::io::Result<()>
    where
        H: Fn(std::io::Error) + 'static,
    {
        let path = Self::path();
        let file = gio::File::for_path(&path);
        let monitor = file
            .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .unwrap_or_else(|err| panic!("Failed to initiate file monitor for {path:?} ({err})"));

        monitor.connect_changed(
            move |_monitor: &gio::FileMonitor,
                  file: &gio::File,
                  _other_file: Option<&gio::File>,
                  event: gio::FileMonitorEvent| {
                if event == gio::FileMonitorEvent::ChangesDoneHint {
                    info!("Reloading file after change {:?}", file.path());
                    // TODO send notification?
                    match Self::from_file() {
                        Ok(new) => store.update(|s| *s = new.clone()),
                        Err(err) => {
                            error!("Failed to reload {:?}: {}", file.path(), err);
                            error_handler(err);
                        }
                    }
                }
            },
        );

        debug!("File monitor connected for {:?}", path);

        FILE_MONITORS.with(|file_monitors| {
            let mut new = file_monitors.take();
            new.push(monitor);
            file_monitors.set(new);
        });

        info!("Initial load for {:?}", path);
        let new = Self::from_file()?;
        store.update(|s| *s = new.clone());

        Ok(())
    }
}

pub trait ConfigType {
    fn path() -> std::path::PathBuf;
}

/// This trait needs to be implemented for all config files
///
/// The default implementation considers all versions valid <= current config version
pub trait ConfigVersion {
    /// Whether the version on disk is read-compatible with this version of the app
    ///
    /// Unless the on-disk version is newer than our latest version this is assumed to be true
    fn version_compatible(version: u64) -> bool {
        version <= super::VERSION
    }

    /// Extract the config version from the json value
    fn extract_version(json: &serde_json::Value) -> u64;
}
