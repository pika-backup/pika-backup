use super::Result;
use arc_swap::ArcSwap;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::prelude::*;
use std::sync::Arc;

use super::log_json;
use super::status::Status;

#[derive(Default, Debug, Clone)]
pub struct Communication {
    pub status: Arc<ArcSwap<Status>>,
    pub instruction: Arc<ArcSwap<Instruction>>,
    pub sender: Arc<ArcSwap<Vec<UnboundedSender<log_json::Output>>>>,
}

impl Communication {
    pub fn new_receiver(&self) -> UnboundedReceiver<log_json::Output> {
        let (sender, receiver) = unbounded::<log_json::Output>();
        self.sender
            .rcu(move |v| [v.to_vec(), vec![sender.clone()]].concat());
        receiver
    }

    pub(super) fn new_sender(&self) -> Sender {
        Sender(self.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nothing,
    Abort,
}

impl Default for Instruction {
    fn default() -> Self {
        Self::Nothing
    }
}

pub(super) struct Sender(Communication);

impl Sender {
    pub async fn send(&self, msg: log_json::Output) -> Result<()> {
        for mut sender in self.0.sender.load().iter() {
            sender.send(msg.clone()).await?;
        }
        Ok(())
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        self.0.sender.store(Default::default());
    }
}
