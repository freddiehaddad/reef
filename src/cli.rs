use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "epub-reader")]
#[command(version = "0.0.1")]
#[command(about = "A cross-platform TUI EPUB reader for developers", long_about = None)]
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
    pub fn validate(&self) -> Result<(), String> {
        if let Some(width) = self.max_width {
            if width < 40 {
                return Err("Max width too small (minimum 40)".to_string());
            }
            if width > 200 {
                return Err("Max width too large (maximum 200)".to_string());
            }
        }
        Ok(())
    }
}
