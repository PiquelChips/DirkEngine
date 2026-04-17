use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error while loading assets: {0}")]
    IoError(#[from] std::io::Error),
    #[error("error during .dirkasset JSON serialisation: {0}")]
    SerialisationError(#[from] serde_json::Error),
}
