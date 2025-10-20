use adw::subclass::prelude::*;
use common::config;

use super::actions;
use crate::widget::SpinnerPage;

mod imp {
    use super::*;
    use crate::widget::PkSpinnerPageImpl;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "add_existing.ui")]
    pub struct SetupAddExistingPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for SetupAddExistingPage {
        const NAME: &'static str = "PkSetupAddExistingPage";
        type Type = super::SetupAddExistingPage;
        type ParentType = SpinnerPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupAddExistingPage {}
    impl WidgetImpl for SetupAddExistingPage {}
    impl NavigationPageImpl for SetupAddExistingPage {}
    impl PkSpinnerPageImpl for SetupAddExistingPage {}

    impl SetupAddExistingPage {
        pub(super) async fn check_repo(
            &self,
            repo: common::config::Repository,
            password: Option<config::Password>,
        ) -> std::result::Result<config::Backup, actions::ConnectRepoError> {
            actions::try_peek(repo.clone(), password.clone())
                .await
                .map(|info| config::Backup::new(repo, info, password.is_some()))
        }
    }
}

glib::wrapper! {
    pub struct SetupAddExistingPage(ObjectSubclass<imp::SetupAddExistingPage>)
    @extends SpinnerPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupAddExistingPage {
    pub async fn check_repo(
        &self,
        repo: config::Repository,
        password: Option<config::Password>,
    ) -> std::result::Result<config::Backup, actions::ConnectRepoError> {
        self.imp().check_repo(repo, password).await
    }
}
