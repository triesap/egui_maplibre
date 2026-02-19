use crate::errors::RenderError;
use crate::types::Camera;

use super::image_worker::ImageWorkerState;
use super::protocol::{WorkerCmd, WorkerFrameResult};

pub struct GlowWorkerState {
    inner: ImageWorkerState,
}

impl GlowWorkerState {
    pub fn spawn(initial_camera: Camera) -> Self {
        Self {
            inner: ImageWorkerState::spawn(initial_camera),
        }
    }

    pub fn send(&self, cmd: WorkerCmd) -> Result<(), RenderError> {
        self.inner.send(cmd)
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<WorkerFrameResult> {
        self.inner.try_recv()
    }

    #[must_use]
    pub fn drain_latest(&self) -> Option<WorkerFrameResult> {
        self.inner.drain_latest()
    }
}

#[cfg(test)]
mod tests {
    use super::GlowWorkerState;
    use crate::types::Camera;
    use crate::worker::protocol::WorkerCmd;

    #[test]
    fn glow_worker_accepts_shutdown() {
        let worker = GlowWorkerState::spawn(Camera::default());
        assert!(worker.send(WorkerCmd::Shutdown).is_ok());
    }
}
