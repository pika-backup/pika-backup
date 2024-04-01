use glib::subclass::prelude::*;

#[macro_export]
macro_rules! obj {
    (name => $name:literal, type => $type:ident, properties => [$(name => $p_name:ident, type => $p_type:path, $(setter => $p_setter:ident)?),*]$(,)?) => {
        glib::wrapper! {
            pub struct $type(ObjectSubclass<imp::$type>);
        }

        impl $type {
            pub fn new() -> Self {
                glib::Object::new()
            }

            $(
                pub fn $p_name(&self) -> $p_type {
                    self.imp().$p_name.borrow().clone()
                }

                $(
                    pub fn $p_setter(&self, $p_name: $p_type) {
                        self.imp().$p_name.replace($p_name);
                    }
                )?
            )*
        }

        mod imp {
            use super::*;

            #[derive(Default)]
            pub struct $type {
                $(
                    pub $p_name: std::cell::RefCell<$p_type>,
                )*
            }

            impl ObjectImpl for $type {}

            #[glib::object_subclass]
            impl ObjectSubclass for $type {
                const NAME: &'static str = $name;
                type Type = super::$type;
                type ParentType = glib::Object;
            }
        }
    };
}

obj!(
    name => "PikaAddConfigTask",
    type => AddConfigTask,
    properties => [
        name => repo,
        type => Option<crate::config::Repository>,
        setter => set_repo
    ],
);
