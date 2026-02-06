use std::cell::{Cell, OnceCell, RefCell};
use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::borg::{SizeEstimate, SizeEstimateLeaf};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Node {
        pub(super) size_estimate: Cell<SizeEstimate>,
        pub(super) children: OnceCell<gio::ListStore>,
        pub(super) path: RefCell<PathBuf>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Node {
        const NAME: &'static str = "PkTreeNode";
        type Type = super::ModelNode;
    }

    impl ObjectImpl for Node {}
}

glib::wrapper! {
    pub struct ModelNode(ObjectSubclass<imp::Node>);
}

impl ModelNode {
    pub fn new(path: PathBuf, leaf: SizeEstimateLeaf) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.size_estimate.replace(leaf.overall);
        imp.path.replace(path);

        imp.children.set(Self::new_model(leaf)).unwrap();

        obj
    }

    pub fn new_model(leaf: SizeEstimateLeaf) -> gio::ListStore {
        let children = gio::ListStore::new::<Self>();
        for (path, child) in leaf.children {
            children.append(&Self::new(path, child));
        }
        children
    }

    pub fn name(&self) -> PathBuf {
        self.imp().path.borrow().clone()
    }

    pub fn size_estimate(&self) -> SizeEstimate {
        self.imp().size_estimate.get()
    }

    pub fn children(&self) -> Option<gio::ListModel> {
        let children = self.imp().children.get().unwrap();

        if children.n_items() > 0 {
            Some(children.clone().upcast::<gio::ListModel>())
        } else {
            None
        }
    }
}
