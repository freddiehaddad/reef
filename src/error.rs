use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("EPUB file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid or corrupted EPUB: {0}")]
    InvalidEpub(String),

    #[error("Failed to extract chapter: {0}")]
    ChapterExtractionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Terminal too small (minimum 80x24)")]
    TerminalTooSmall,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
