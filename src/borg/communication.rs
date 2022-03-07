use super::error;
use super::Result;
use arc_swap::ArcSwap;
use async_std::channel::{self, unbounded};
use std::sync::Arc;

use super::log_json;
use super::status::Run as Status;
use super::task::Task;

#[derive(Default, Debug, Clone)]
pub struct Communication<T: Task> {
    pub general_info: Arc<ArcSwap<super::status::GeneralStatus>>,
    pub specific_info: Arc<ArcSwap<T::Info>>,
    pub status: Arc<ArcSwap<Status>>,
    pub(in crate::borg) instruction: Arc<ArcSwap<Instruction>>,
    sender: Arc<ArcSwap<Vec<channel::Sender<log_json::Output>>>>,
}

impl<T: Task> Communication<T> {
    pub fn new_receiver(&self) -> channel::Receiver<log_json::Output> {
        let (sender, receiver) = unbounded::<log_json::Output>();
        self.sender
            .rcu(move |v| [v.to_vec(), vec![sender.clone()]].concat());
        receiver
    }

    pub(in crate::borg) fn new_sender(&self) -> Sender<T> {
        Sender(self.clone())
    }

    pub(in crate::borg) fn set_status(&self, status: Status) {
        self.status.store(Arc::new(status));
    }

    pub fn status(&self) -> Status {
        *(*self.status.load()).clone()
    }

    pub fn set_instruction(&self, instruction: Instruction) {
        self.instruction.store(Arc::new(instruction));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nothing,
    Abort(error::Abort),
}

impl Default for Instruction {
    fn default() -> Self {
        Self::Nothing
    }
}

pub(super) struct Sender<T: Task>(Communication<T>);

impl<T: Task> Sender<T> {
    pub async fn send(&self, msg: log_json::Output) -> Result<()> {
        for sender in self.0.sender.load().iter() {
            sender.send(msg.clone()).await?;
        }
        Ok(())
    }
}

impl<T: Task> Drop for Sender<T> {
    fn drop(&mut self) {
        self.0.sender.store(Default::default());
    }
}
