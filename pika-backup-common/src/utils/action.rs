use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;

use glib::Variant;
use glib::variant::FromVariant;

pub trait Action<T: FromVariant + Debug + 'static> {
    const PREFIX: &'static str = "app";
    const NAME: &'static str;
    const PARAMETER_TYPE: Option<&glib::VariantTy> = None;

    fn action() -> gio::SimpleAction {
        std::thread_local! {
            static ACTION: RefCell<HashMap<String, gio::SimpleAction>> = Default::default();
        };

        ACTION.with(move |x| {
            x.borrow_mut().entry(Self::NAME.to_string()).or_insert_with(move || {
                let action = gio::SimpleAction::new(Self::NAME, Self::PARAMETER_TYPE);
                if TypeId::of::<T>() == TypeId::of::<()>() {
                    action.connect_activate(|_, parameter| {
                        tracing::debug!("Activating action {}(())", Self::name());
                        if let Some(parameter) = parameter {
                            tracing::error!("Action {} doesn't expect any parameters. Got: {}", Self::name(), parameter.print(true))
                        }
                        Self::activate(T::from_variant(&Variant::from(())).expect("This transformation must succeed."));
                    });
                } else {
                    action.connect_activate(|_, parameter| {
                        if let Some(parameter) = parameter {
                            if let Some(parameter) = T::from_variant(parameter) {
                                tracing::debug!("Activating action {}({:#?})", Self::name(), parameter);
                                Self::activate(parameter);
                            } else {
                                tracing::error!(
                                    "Can't activate action '{}' with wrong parameter type. Expected type is {:?}, but got {}",
                                    Self::name(),
                                    Self::PARAMETER_TYPE,
                                    parameter.type_()
                                );
                            }
                        } else {
                            tracing::error!(
                                "Can't activate action '{}' without parameter",
                                Self::name(),
                            );
                        }
                    });
                }

                action
            })
            .clone()
        })
    }

    fn activate(parameter: T);

    fn name() -> String {
        format!("{}.{}", Self::PREFIX, Self::NAME)
    }
}
