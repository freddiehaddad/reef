use crate::constants::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH};
use thiserror::Error;

/// Application-level errors
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

    #[error("Terminal too small (minimum {MIN_TERMINAL_WIDTH}x{MIN_TERMINAL_HEIGHT})")]
    TerminalTooSmall,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
