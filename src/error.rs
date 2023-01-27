use std::io;

use tokio::task::JoinError;




pub type Result<V> = std::result::Result<V, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Task Error: {0:?}")]
    TaskError(#[from] JoinError),

    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),
}