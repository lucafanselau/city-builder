use crate::graphics::context::BufferHandle;

use anyhow;
use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Buffer allocation failed due to memory constraints")]
    OutOfMemory,
    #[error("Unknown error occurred")]
    Other(#[from] anyhow::Error),
}

pub trait RenderContext {
    fn create_buffer(&self) -> Result<BufferHandle, BufferError>;
}
