pub mod about;
mod archive_prefix;
mod backup_info;
mod check;
mod check_result;
mod delete_archive;
mod device_missing;
mod encryption_password;
mod exclude;
mod preferences;
mod prune;
pub mod setup;
mod size_estimate;
mod storage;
mod unmount_archives;

pub use archive_prefix::ArchivePrefixDialog;
pub use backup_info::BackupInfoDialog;
pub use check::CheckDialog;
pub use check_result::CheckResultDialog;
pub use delete_archive::DeleteArchiveDialog;
pub use device_missing::DeviceMissingDialog;
pub use encryption_password::EncryptionPasswordDialog;
pub use exclude::ExcludeDialog;
use glib::prelude::*;
pub use preferences::PreferencesDialog;
pub use prune::PruneDialog;
pub use size_estimate::SizeEstimateDialog;
pub use storage::StorageDialog;
pub use unmount_archives::UnmountArchives;

pub fn init() {
    ArchivePrefixDialog::static_type();
    CheckDialog::static_type();
    CheckResultDialog::static_type();
    DeleteArchiveDialog::static_type();
    DeviceMissingDialog::static_type();
    EncryptionPasswordDialog::static_type();
    ExcludeDialog::static_type();
    PreferencesDialog::static_type();
    PruneDialog::static_type();
    UnmountArchives::static_type();
    StorageDialog::static_type();
}
