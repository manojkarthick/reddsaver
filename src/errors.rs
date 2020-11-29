use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReddSaverError {
    #[error("Missing environment variable")]
    EnvVarNotPresent(#[from] std::env::VarError),
    #[error("Unable to process request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Could not create directory")]
    CouldNotCreateDirectory,
    #[error("Could not save image to filesystem")]
    CouldNotSaveImageError,
    #[error("Could not save image to filesystem")]
    CouldNotCreateImageError,
    #[error("Unable to join tasks")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Could not save string to int")]
    IntegerConversionError(#[from] std::num::ParseIntError)

}