use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("db: {0}")]
    Db(String),
    #[error("git: {0}")]
    Git(String),
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e.to_string())
    }
}
