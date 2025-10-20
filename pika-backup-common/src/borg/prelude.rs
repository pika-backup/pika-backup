pub use async_trait::async_trait;

pub use super::BorgRunConfig;
pub use super::log_json::LogExt;
pub use crate::prelude::*;

#[async_trait]
pub(crate) trait CommandCommunicationExt {
    /// Abortable [Command::output](async_process::Command::output)
    ///
    /// This will listen on the communication channel for abort messages and
    /// abort the task early when an abort is desired.
    async fn output_with_communication<T: super::Task>(
        &mut self,
        communication: super::Communication<T>,
    ) -> super::Result<async_process::Output>;
}

#[async_trait]
impl CommandCommunicationExt for async_process::Command {
    async fn output_with_communication<T: super::Task>(
        &mut self,
        communication: super::Communication<T>,
    ) -> super::Result<async_process::Output> {
        // TODO: Handle borg questions (stdin)
        self.stdin(async_process::Stdio::piped());
        self.stdout(async_process::Stdio::piped());
        self.stderr(async_process::Stdio::piped());

        let mut child = self.spawn()?;

        loop {
            if let super::Instruction::Abort(abort) = communication.instruction.get() {
                return Err(super::Error::Aborted(abort));
            }

            if child.try_status()?.is_some() {
                return Ok(child.output().await?);
            }

            smol::Timer::after(Duration::from_millis(100)).await;
        }
    }
}
