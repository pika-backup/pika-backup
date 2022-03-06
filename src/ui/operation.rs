//! Track [crate::borg] operation from UI's side

use adw::prelude::*;
use async_std::prelude::*;
use ui::prelude::*;

use crate::borg;
use crate::ui;
use glib::{Continue, SignalHandlerId};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

const TIME_METERED_ABORT: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub struct Operation<T: borg::Task> {
    //battery_since: Rc<Cell<Option<Instant>>>,
    command: borg::Command<T>,
    metered_since: Cell<Option<Instant>>,
    metered_signal_handler: Cell<Option<SignalHandlerId>>,
    last_log: RefCell<Option<Rc<borg::log_json::Output>>>,
}

impl<T: borg::Task> Operation<T> {
    pub fn register(command: borg::Command<T>) -> Rc<dyn OperationExt> {
        let process = Rc::new(Self {
            //battery_since: Cell::new(None),
            command,
            metered_since: Default::default(),
            metered_signal_handler: Default::default(),
            last_log: Default::default(),
        });

        process.metered_signal_handler.set(Some(
            gio::NetworkMonitor::default().connect_network_metered_notify(
                glib::clone!(@weak process => move |x| {
                    if x.is_network_metered() {
                        debug!("Connection now metered.");
                        process.metered_since.set(Some(Instant::now()));
                    } else {
                        debug!("Connection no longer metered.");
                        process.metered_since.set(None);
                    }
                }),
            ),
        ));

        let mut log_receiver = process.communication().new_receiver();
        let weak_process = Rc::downgrade(&process);
        glib::MainContext::default().spawn_local(async move {
            while let Some(output) = log_receiver.next().await {
                if let Some(process) = weak_process.upgrade() {
                    process.check_output(output);
                }
            }
        });

        glib::source::timeout_add_local(
            POLL_INTERVAL,
            glib::clone!(@weak process => @default-return Continue(false), move || process.check()),
        );

        BORG_OPERATION.with(enclose!((process) move |operations| {
            operations.update(|op| {
                op.insert(
                    process.command.config.id.clone(),
                    process.clone(),
                );
            });
        }));

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

    fn check_output(&self, output: borg::log_json::Output) {
        // TODO call other stuff
        self.last_log.replace(Some(Rc::new(output)));
        ui::page_backup::refresh_status();
    }

    fn check(&self) -> Continue {
        if self.time_metered_exceeded() {
            info!("Stopping operation on metered connection now.");
            self.communication()
                .instruction
                .store(Arc::new(borg::Instruction::Abort(
                    borg::Abort::MeteredConnection,
                )));
        }

        Continue(true)
    }

    pub fn time_metered_exceeded(&self) -> bool {
        if let Some(instant) = self.metered_since.get() {
            instant.elapsed() > TIME_METERED_ABORT
        } else {
            false
        }
    }
}

impl<T: borg::Task> Drop for Operation<T> {
    fn drop(&mut self) {
        debug!("Dropping operation tracking '{}'.", T::name());

        if let Some(handler) = self.metered_signal_handler.take() {
            gio::NetworkMonitor::default().disconnect(handler);
        }
    }
}

pub trait OperationExt {
    fn name(&self) -> String;
    fn any(&self) -> &dyn Any;
    fn repo_id(&self) -> &borg::RepoId;
    fn set_instruction(&self, instruction: borg::Instruction);
    fn try_as_create(&self) -> Option<&Operation<borg::task::Create>>;
}

impl<T: borg::Task> OperationExt for Operation<T> {
    fn name(&self) -> String {
        T::name()
    }
    fn any(&self) -> &dyn Any {
        self
    }
    fn set_instruction(&self, instruction: borg::Instruction) {
        self.communication().set_instruction(instruction);
    }
    fn repo_id(&self) -> &borg::RepoId {
        self.repo_id()
    }

    fn try_as_create(&self) -> Option<&Operation<borg::task::Create>> {
        self.any().downcast_ref()
    }
}
