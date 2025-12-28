//! Command-line interface parsing and validation
//!
//! This module handles CLI argument parsing using clap and validates
//! user inputs for correctness.

use crate::constants::{MAX_MAX_WIDTH, MIN_MAX_WIDTH};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "reef")]
#[command(version = "0.0.1")]
#[command(about = "Dive into your books from the comfort of your terminal", long_about = None)]
pub struct Cli {
    /// Path to EPUB file to open
    pub file: Option<String>,

    /// Maximum text width in columns (40-200)
    #[arg(short = 'm', long, value_name = "COLS")]
    pub max_width: Option<usize>,

    /// Enable logging to specified file
    #[arg(short = 'l', long, value_name = "PATH")]
    pub log_file: Option<String>,
}

impl Cli {
    /// Validate CLI arguments
    /// Returns error if max_width is out of bounds (40-200)
    pub fn validate(&self) -> Result<(), String> {
        if let Some(width) = self.max_width {
            if width < MIN_MAX_WIDTH {
                return Err(format!("Max width too small (minimum {})", MIN_MAX_WIDTH));
            }
            if width > MAX_MAX_WIDTH {
                return Err(format!("Max width too large (maximum {})", MAX_MAX_WIDTH));
            }
        }
        Ok(())
    }
}
