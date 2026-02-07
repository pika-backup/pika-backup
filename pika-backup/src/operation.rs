//! Track [common::borg] operation from UI's side

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use common::borg::log_json;
use common::{borg, config};
use enclose::enclose;

use super::App;
use crate::prelude::*;

const TIME_METERED_ABORT: Duration = Duration::from_secs(60);
const TIME_ON_BATTERY_ABORT: Duration = Duration::from_secs(20 * 60);
const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub struct Operation<T: borg::Task> {
    command: borg::Command<T>,
    last_log: RefCell<Option<Rc<borg::log_json::Output>>>,
    inhibit_cookie: Cell<Option<u32>>,
    aborting: Cell<bool>,
    operation_shutdown: Cell<bool>,
}

impl<T: borg::Task> Operation<T> {
    /// Globally register a running borg command
    pub fn register(command: borg::Command<T>) -> Rc<dyn OperationExt> {
        let process = Rc::new(Self {
            command,
            last_log: Default::default(),
            inhibit_cookie: Default::default(),
            aborting: Default::default(),
            operation_shutdown: Default::default(),
        });

        let weak_process = Rc::downgrade(&process);
        glib::MainContext::default().spawn_local(async move {
            while let Some(log_receiver) = weak_process
                .upgrade()
                .map(|x| x.communication().new_receiver())
            {
                tracing::debug!("Connect to new communication messages");
                while let Ok(output) = log_receiver.recv().await {
                    if let Some(process) = weak_process.upgrade() {
                        process.check_output(output);
                    }
                }

                if Some(false) != weak_process.upgrade().map(|x| x.operation_shutdown.get()) {
                    tracing::debug!("Stop listening to communication messages because of shutdown");
                    return;
                }
            }
        });

        glib::source::timeout_add_local(
            POLL_INTERVAL,
            glib::clone!(
                #[weak]
                process,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    glib::MainContext::default().spawn_local(Self::check(process));
                    glib::ControlFlow::Continue
                }
            ),
        );

        // prevent shutdown etc.
        if process.is_application_inhibit() {
            process.application_inhibit();
        }

        BORG_OPERATION.with(enclose!((process) move |operations| {
            operations.update(|op| {
                op.insert(
                    process.command.config.id.clone(),
                    process.clone(),
                );
            });
        }));

        process.ui_status_update();
        process.ui_schedule_update();

        process
    }

    pub fn last_log(&self) -> Option<Rc<borg::log_json::Output>> {
        self.last_log.try_borrow().ok().and_then(|x| x.clone())
    }

    pub fn repo_id(&self) -> &borg::RepoId {
        &self.command.config.repo_id
    }

    pub fn communication(&self) -> &borg::Communication<T> {
        &self.command.communication
    }

    fn check_output(&self, update: borg::Update) {
        match update {
            borg::Update::Msg(output) => {
                let output = Rc::new(output);

                if !output.to_string().is_empty() {
                    self.last_log.replace(Some(output.clone()));
                }

                if let log_json::Output::Progress(log_json::Progress::Question(question)) = &*output
                {
                    // A question was asked
                    self.handle_borg_question(question);
                }
            }
            _ => {
                self.ui_schedule_update();
            }
        }

        self.ui_status_update();
    }

    async fn check(self_: Rc<Self>) {
        if self_.command.from_schedule.is_some()
            && self_.is_time_metered_exceeded()
            && self_.command.config.repo.is_internet().await
        {
            tracing::info!("Stopping scheduled operation on metered connection now.");
            self_
                .communication()
                .set_instruction(borg::Instruction::Abort(borg::Abort::MeteredConnection));
        } else if self_.command.from_schedule.is_some() && self_.is_time_on_battery_exceeded() {
            tracing::info!("Stopping scheduled operation on battery now.");
            self_
                .communication()
                .set_instruction(borg::Instruction::Abort(borg::Abort::OnBattery));
        }
    }

    pub fn is_time_metered_exceeded(&self) -> bool {
        match status_tracking().metered_since.get() {
            Some(instant) => instant.elapsed() > TIME_METERED_ABORT,
            _ => false,
        }
    }

    pub fn is_time_on_battery_exceeded(&self) -> bool {
        if self.command.config.schedule.settings.run_on_battery {
            // Running on battery was explicitly enabled
            false
        } else {
            match status_tracking().on_battery_since.get() {
                Some(instant) => instant.elapsed() > TIME_ON_BATTERY_ABORT,
                _ => false,
            }
        }
    }

    pub fn is_application_inhibit(&self) -> bool {
        // Do not inhibit for hourly backups
        !(self.command.from_schedule.is_some()
            && matches!(
                self.command.config.schedule.frequency,
                config::Frequency::Hourly
            ))
    }

    /// Prevent shutdown as long as operation is in progress
    fn application_inhibit(&self) {
        let cookie = adw_app().inhibit(
            Some(&main_ui().window()),
            gtk::ApplicationInhibitFlags::LOGOUT | gtk::ApplicationInhibitFlags::SUSPEND,
            Some(&T::name()),
        );

        if cookie > 0 {
            self.inhibit_cookie.set(Some(cookie));
        } else {
            tracing::warn!("Failed to set application inhibit.");
        }
    }

    fn ui_status_update(&self) {
        tracing::debug!("UI status update");

        if ACTIVE_BACKUP_ID.get() == self.command.config_id() {
            main_ui().page_detail().backup_page().refresh_status();
            main_ui().page_detail().archives_page().refresh_status();
        }

        main_ui().page_overview().refresh_status();
        main_ui().page_detail().backup_page().refresh_disk_status();
        glib::MainContext::default().spawn(crate::shell::background_activity_update());
    }

    fn ui_schedule_update(&self) {
        tracing::debug!("UI schedule update");

        if ACTIVE_BACKUP_ID.get() == self.command.config_id() {
            main_ui().page_detail().schedule_page().refresh_status();
        }

        main_ui().page_overview().refresh_status();
    }

    /// Handle a borg question (such as repository was relocated)
    fn handle_borg_question(&self, question: &log_json::QuestionPrompt) {
        let communication = self.communication().clone();

        if !self.command.is_scheduled() {
            // Abort backup if question is asked during schedule
            communication.set_instruction(borg::Instruction::Abort(
                borg::Abort::QuestionDuringSchedule(question.clone()),
            ));
        } else {
            // Show dialog if question is asked during schedule
            glib::MainContext::default().spawn_local(glib::clone!(
                #[strong]
                question,
                async move {
                    let response =
                        crate::utils::show_borg_question(&App::default().main_window(), &question)
                            .await;
                    communication.set_instruction(borg::Instruction::Response(response));
                }
            ));
        }
    }
}

impl<T: borg::Task> Drop for Operation<T> {
    fn drop(&mut self) {
        tracing::debug!("Dropping operation tracking '{}'.", T::name());

        self.operation_shutdown.replace(true);
        self.communication().drop_senders();

        if BORG_OPERATION.try_with(|_| {}).is_err() {
            tracing::debug!("Not doing any external operations.");
        } else {
            self.ui_status_update();
            self.ui_schedule_update();

            if let Some(cookie) = self.inhibit_cookie.take() {
                adw_app().uninhibit(cookie);
            }
        }
    }
}

pub trait OperationExt {
    fn name(&self) -> String;
    fn any(&self) -> &dyn Any;
    fn repo_id(&self) -> &borg::RepoId;
    fn set_instruction(&self, instruction: borg::Instruction);
    fn aborting(&self) -> bool;
    fn status(&self) -> borg::RunStatus;
    fn try_as_create(&self) -> Option<&Operation<borg::task::Create>>;
    fn last_log(&self) -> Option<Rc<borg::log_json::Output>>;
    fn task_kind(&self) -> borg::task::Kind;
}

impl<T: borg::Task> OperationExt for Operation<T> {
    fn name(&self) -> String {
        T::name()
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn set_instruction(&self, instruction: borg::Instruction) {
        if matches!(instruction, borg::Instruction::Abort(_)) {
            self.aborting.set(true);
        }

        self.communication().set_instruction(instruction);
    }

    fn aborting(&self) -> bool {
        self.aborting.get()
    }

    fn status(&self) -> borg::RunStatus {
        self.communication().status()
    }

    fn repo_id(&self) -> &borg::RepoId {
        self.repo_id()
    }

    fn try_as_create(&self) -> Option<&Operation<borg::task::Create>> {
        self.any().downcast_ref()
    }

    fn last_log(&self) -> Option<Rc<borg::log_json::Output>> {
        self.last_log()
    }

    fn task_kind(&self) -> borg::task::Kind {
        T::KIND
    }
}
