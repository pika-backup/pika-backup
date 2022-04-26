pub mod cache;
mod display;
mod events;
mod init;

pub use display::update_info;
pub use init::init;

use adw::prelude::*;

use crate::ui;
use ui::prelude::*;

fn is_visible() -> bool {
    main_ui().detail_stack().visible_child()
        == Some(main_ui().page_archives().upcast::<gtk::Widget>())
}

fn find_first_populated_dir(dir: &std::path::Path) -> std::path::PathBuf {
    if let Ok(mut dir_iter) = dir.read_dir() {
        if let Some(Ok(new_dir)) = dir_iter.next() {
            if new_dir.path().is_dir() && dir_iter.next().is_none() {
                return find_first_populated_dir(&new_dir.path());
            }
        }
    }

    dir.to_path_buf()
}
