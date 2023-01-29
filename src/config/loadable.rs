use crate::prelude::*;
use gio::prelude::*;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use std::cell::Cell;

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

pub trait TrackChanges: Sized {
    fn update_on_change(store: &'static Lazy<ArcSwap<Self>>) -> std::io::Result<()>;
}

thread_local! {
static FILE_MONITORS: Cell<Vec<gio::FileMonitor>> = Default::default();
}

impl<C: ConfigType + serde::de::DeserializeOwned + Default + Clone> TrackChanges for C {
    fn update_on_change(store: &'static Lazy<ArcSwap<Self>>) -> std::io::Result<()> {
        let path = Self::path();
        let file = gio::File::for_path(&path);
        let monitor = file
            .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .unwrap_or_else(|err| panic!("Failed to initiate file monitor for {path:?} ({err})"));

        monitor.connect_changed(
            |_monitor: &gio::FileMonitor,
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
