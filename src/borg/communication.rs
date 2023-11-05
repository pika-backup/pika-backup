use crate::borg::prelude::*;

use super::error;
use super::Result;
use arc_swap::ArcSwap;
use async_std::channel::{self, unbounded};
use std::sync::Arc;

use super::log_json;
use super::status::Run as Status;
use super::task::Task;

#[derive(Debug, Clone)]
pub enum Update {
    Msg(log_json::Output),
    Status(Status),
}

#[derive(Default, Debug, Clone)]
pub struct Communication<T: Task> {
    pub general_info: Arc<ArcSwap<super::status::GeneralStatus>>,
    pub specific_info: Arc<ArcSwap<T::Info>>,
    pub status: Arc<ArcSwap<Status>>,
    pub(in crate::borg) instruction: Arc<ArcSwap<Instruction>>,
    sender: Arc<ArcSwap<Vec<channel::Sender<Update>>>>,
}

impl<T: Task> Communication<T> {
    pub fn new_receiver(&self) -> channel::Receiver<Update> {
        let (sender, receiver) = unbounded::<Update>();
        self.sender
            .rcu(move |v| [v.to_vec(), vec![sender.clone()]].concat());
        receiver
    }

    pub(in crate::borg) fn new_sender(&self) -> Sender<T> {
        Sender(self.clone())
    }

    pub fn drop_sender(&self) {
        self.sender.store(Default::default());
    }

    pub(in crate::borg) fn set_status(&self, status: Status) {
        if !matches!(**self.status.load(), Status::Stopping) {
            self.status.store(Arc::new(status));
            let senders = self.sender.get().into_iter();
            async_std::task::spawn(async move {
                for sender in senders {
                    if let Err(err) = sender.send(Update::Status(status)).await {
                        error!("Failed to send status update: {}", err);
                    }
                }
            });
        }
    }

    pub fn status(&self) -> Status {
        *(*self.status.load()).clone()
    }

    pub fn set_instruction(&self, instruction: Instruction) {
        self.instruction.store(Arc::new(instruction));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Yes,
    No,
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::Yes => write!(f, "YES"),
            Response::No => write!(f, "NO"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nothing,
    Abort(error::Abort),
    Response(Response),
}

impl Default for Instruction {
    fn default() -> Self {
        Self::Nothing
    }
}

#[derive(Clone)]
pub(super) struct Sender<T: Task>(Communication<T>);

impl<T: Task> Sender<T> {
    pub async fn send(&self, msg: log_json::Output) -> Result<()> {
        for sender in self.0.sender.load().iter() {
            sender.send(Update::Msg(msg.clone())).await?;
        }
        Ok(())
    }
}

impl<T: Task> Drop for Sender<T> {
    fn drop(&mut self) {
        self.0.sender.store(Default::default());
    }
}
