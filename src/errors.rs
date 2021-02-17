use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum ReddSaverError {
    #[error("Missing environment variable")]
    EnvVarNotPresent(#[from] std::env::VarError),
    #[error("Unable to process request")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Could not create directory")]
    CouldNotCreateDirectory,
    #[error("Could not save image `{0}` to filesystem")]
    CouldNotSaveImageError(String),
    #[error("Could not create image `{0}` from `{1}`")]
    CouldNotCreateImageError(String, String),
    #[error("Unable to join tasks")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("Could not save string to int")]
    ParsingIntError(#[from] std::num::ParseIntError),
    #[error("Could not save usize to int")]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error("Data directory not found, please check if it exists")]
    DataDirNotFound,
    #[error("Could not create or save image")]
    IoError(#[from] std::io::Error),
    #[error("Unable to parse URL")]
    UrlError(#[from] url::ParseError),
}
