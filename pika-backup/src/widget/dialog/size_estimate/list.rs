use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;

use super::model_node::ModelNode;
use super::value_unit::ValueUnit;
mod imp {

    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(file = "list.ui")]
    pub struct List {
        #[template_child]
        list_view: TemplateChild<gtk::ColumnView>,

        #[template_child]
        value_size_group_changed: TemplateChild<gtk::SizeGroup>,
        #[template_child]
        unit_size_group_changed: TemplateChild<gtk::SizeGroup>,

        #[template_child]
        value_size_group_total: TemplateChild<gtk::SizeGroup>,
        #[template_child]
        unit_size_group_total: TemplateChild<gtk::SizeGroup>,

        base_model: RefCell<gio::ListStore>,
    }

    impl Default for List {
        fn default() -> Self {
            Self {
                list_view: Default::default(),
                value_size_group_changed: Default::default(),
                unit_size_group_changed: Default::default(),
                value_size_group_total: Default::default(),
                unit_size_group_total: Default::default(),
                base_model: RefCell::new(gio::ListStore::new::<ModelNode>()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for List {
        const NAME: &'static str = "PkSizeEstimateList";
        type Type = super::List;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for List {
        fn constructed(&self) {
            self.parent_constructed();

            let factory = gtk::SignalListItemFactory::new();

            factory.connect_setup(|_, list_item| {
                dbg!(list_item.type_().name());
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                let label = gtk::Label::new(None);

                let expander = gtk::TreeExpander::new();
                expander.set_child(Some(&label));

                list_item.set_child(Some(&expander));
            });

            factory.connect_bind(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                let tree_row = list_item.item().and_downcast::<gtk::TreeListRow>().unwrap();

                let node = tree_row.item().and_downcast::<ModelNode>().unwrap();

                let expander = list_item
                    .child()
                    .and_downcast::<gtk::TreeExpander>()
                    .unwrap();

                let label = expander.child().and_downcast::<gtk::Label>().unwrap();

                expander.set_list_row(Some(&tree_row));
                label.set_text(&node.name().display().to_string());
            });

            let folder_name_factory = gtk::SignalListItemFactory::new();

            folder_name_factory.connect_setup(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                // Label for the node name
                let label = gtk::Label::new(None);
                label.set_xalign(0.0);
                label.set_hexpand(true);

                // Tree expander (adds indentation + arrow)
                let expander = gtk::TreeExpander::new();
                expander.set_child(Some(&label));

                // The ListItem holds exactly one child widget
                list_item.set_child(Some(&expander));
            });

            folder_name_factory.connect_bind(|_, list_item| {
                // The ListItem itself (always this type)
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                // Model item is TreeListRow when using TreeListModel
                let tree_row = list_item
                    .item()
                    .and_downcast::<gtk::TreeListRow>()
                    .expect("Expected TreeListRow");

                // Actual user object
                let node = tree_row
                    .item()
                    .and_downcast::<ModelNode>()
                    .expect("Expected NodeObject");

                // Our widgets
                let expander = list_item
                    .child()
                    .and_downcast::<gtk::TreeExpander>()
                    .unwrap();

                let label = expander.child().and_downcast::<gtk::Label>().unwrap();

                // Connect the expander to the tree row
                expander.set_list_row(Some(&tree_row));

                // Set text
                label.set_text(&node.name().display().to_string());
            });

            let changed_size_factory = gtk::SignalListItemFactory::new();

            changed_size_factory.connect_setup(glib::clone!(
                #[strong(rename_to = value_size_group)]
                self.value_size_group_changed,
                #[strong(rename_to = unit_size_group)]
                self.unit_size_group_changed,
                move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();

                    let label = ValueUnit::new(value_size_group.clone(), unit_size_group.clone());
                    list_item.set_child(Some(&label));
                }
            ));

            changed_size_factory.connect_bind(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let tree_row = list_item.item().and_downcast::<gtk::TreeListRow>().unwrap();
                let model_node = tree_row.item().and_downcast::<ModelNode>().unwrap();

                let label = list_item.child().and_downcast::<ValueUnit>().unwrap();
                label.set_value(model_node.size_estimate().changed);
            });

            let total_size_factory = gtk::SignalListItemFactory::new();

            total_size_factory.connect_setup(glib::clone!(
                #[strong(rename_to = value_size_group)]
                self.value_size_group_total,
                #[strong(rename_to = unit_size_group)]
                self.unit_size_group_total,
                move |_, list_item| {
                    let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                    let label = ValueUnit::new(value_size_group.clone(), unit_size_group.clone());
                    list_item.set_child(Some(&label));
                }
            ));

            total_size_factory.connect_bind(|_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let tree_row = list_item.item().and_downcast::<gtk::TreeListRow>().unwrap();
                let model_node = tree_row.item().and_downcast::<ModelNode>().unwrap();

                let label = list_item.child().and_downcast::<ValueUnit>().unwrap();
                label.set_value(model_node.size_estimate().total);
            });

            let name_column = gtk::ColumnViewColumn::builder()
                .title("Folder")
                .factory(&folder_name_factory)
                .expand(true)
                .build();
            let changed_size_column =
                gtk::ColumnViewColumn::new(Some("Changed Files"), Some(changed_size_factory));
            let total_size_column =
                gtk::ColumnViewColumn::new(Some("Total Size"), Some(total_size_factory));

            self.list_view.append_column(&name_column);
            self.list_view.append_column(&changed_size_column);
            self.list_view.append_column(&total_size_column);
        }
    }
    impl WidgetImpl for List {}
    impl BoxImpl for List {}

    impl List {
        fn selection_model(&self) -> gtk::SingleSelection {
            let tree_model =
                gtk::TreeListModel::new(self.base_model.borrow().clone(), false, false, |obj| {
                    let node = obj.downcast_ref::<ModelNode>().unwrap();
                    node.children()
                });

            gtk::SingleSelection::new(Some(tree_model))
        }

        pub(super) fn set_model(&self, model: gio::ListStore) {
            self.base_model.replace(model);
        }

        pub(super) fn load_model(&self) {
            self.list_view.set_model(Some(&self.selection_model()));
        }
    }
}

glib::wrapper! {
    pub struct List(ObjectSubclass<imp::List>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl List {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_model(&self, model: gio::ListStore) {
        self.imp().set_model(model);
        self.imp().load_model();
    }
}
