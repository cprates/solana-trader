use thiserror::Error;

pub mod utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to read solana config file: ({0})")]
    ConfigReadError(std::io::Error),
    #[error("invalid config: ({0})")]
    InvalidConfig(String),
    #[error("serialization error: ({0})")]
    SerializationError(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
