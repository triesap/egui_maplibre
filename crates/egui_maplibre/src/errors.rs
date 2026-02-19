use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[cfg(feature = "image")]
    #[error("map rendering error: {0}")]
    Native(#[from] maplibre_native::RenderingError),

    #[error("invalid style source")]
    InvalidStyle,

    #[error("worker thread ended unexpectedly")]
    WorkerGone,

    #[error("unsupported operation: {0}")]
    UnsupportedOperation(&'static str),

    #[error("renderer not initialized")]
    RendererNotInitialized,

    #[error("worker channel closed")]
    WorkerChannelClosed,
}
