use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::{glib, gtk};
use common::borg::{SizeEstimate, SizeEstimateInfo};

use super::list;
use super::model_node::ModelNode;
use super::value_unit::ValueUnit;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "dialog.ui")]
    pub struct SizeEstimateDialog {
        #[template_child]
        changed: TemplateChild<ValueUnit>,
        #[template_child]
        total: TemplateChild<ValueUnit>,

        #[template_child]
        pub(super) list: TemplateChild<list::List>,

        size_estimate: Cell<SizeEstimate>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SizeEstimateDialog {
        const NAME: &'static str = "PkSizeEstimateDialog";
        type Type = super::SizeEstimateDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SizeEstimateDialog {}
    impl WidgetImpl for SizeEstimateDialog {}
    impl AdwDialogImpl for SizeEstimateDialog {}

    impl SizeEstimateDialog {
        pub fn set_size_estimate(&self, size_estimate: SizeEstimate) {
            self.size_estimate.set(size_estimate);

            self.changed.set_value(size_estimate.changed);
            self.total.set_value(size_estimate.total);
        }
    }
}

glib::wrapper! {
    pub struct SizeEstimateDialog(ObjectSubclass<imp::SizeEstimateDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SizeEstimateDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_data(&self, size_estimate_info: SizeEstimateInfo) {
        let imp = self.imp();

        imp.set_size_estimate(size_estimate_info.tree.overall);

        let model = ModelNode::new_model(size_estimate_info.tree);
        imp.list.set_model(model);
    }
}
