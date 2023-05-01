pub use super::log_json::LogExt;
pub use super::BorgRunConfig;
pub use crate::prelude::*;

#[async_trait]
pub(crate) trait CommandExt {
    async fn output_with_communication<T: super::Task>(
        &mut self,
        communication: super::Communication<T>,
    ) -> super::Result<async_std::process::Output>;
}

#[async_trait]
impl CommandExt for async_std::process::Command {
    async fn output_with_communication<T: super::Task>(
        &mut self,
        communication: super::Communication<T>,
    ) -> super::Result<async_std::process::Output> {
        // TODO: Handle borg questions (stdin)
        self.stdin(async_std::process::Stdio::piped());
        self.stdout(async_std::process::Stdio::piped());
        self.stderr(async_std::process::Stdio::piped());

        let mut child = self.spawn()?;

        loop {
            if let super::Instruction::Abort(abort) = communication.instruction.get() {
                return Err(super::Error::Aborted(abort));
            }

            if let Some(_) = child.try_status()? {
                return Ok(child.output().await?);
            }

            async_std::task::sleep(Duration::from_millis(100)).await;
        }
    }
}
