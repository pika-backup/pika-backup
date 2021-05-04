use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

pub static BACKUP_CONFIG: Lazy<ArcSwap<crate::config::Backups>> = Lazy::new(Default::default);
pub static BACKUP_HISTORY: Lazy<ArcSwap<crate::config::Histories>> = Lazy::new(Default::default);
