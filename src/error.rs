use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unknown command: {0}")]
    UnknownCommand(String),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
} 