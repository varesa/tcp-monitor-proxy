
#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("IO error {0}")]
    IoError(#[from] std::io::Error),

    #[error("Name resolution error: {0}")]
    NameResolutionError(String),

    #[error("Mirror error: {0}")]
    MirrorError(#[from] MirrorError),
}

#[derive(thiserror::Error, Debug)]
pub enum MirrorError {
    #[error("IO error {0}")]
    IoError(#[from] std::io::Error),

    #[error("Channel send error: {0}")]
    SendError(#[from] futures_channel::mpsc::SendError),

    #[error("Async task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}