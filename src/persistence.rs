use crate::types::{Bookmark, Config};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingProgress {
    pub chapter_idx: usize,
    pub line: usize,
    pub scroll_offset: usize,
    pub last_read: DateTime<Utc>,
    pub toc_expansion_state: Vec<String>,
}

pub struct PersistenceManager {
    config_dir: PathBuf,
}

impl PersistenceManager {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("", "", "epub-reader")
            .context("Failed to determine config directory")?;

        let config_dir = project_dirs.config_dir().to_path_buf();

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        Ok(PersistenceManager { config_dir })
    }

    // Config methods
    pub fn load_config(&self) -> Result<Config> {
        let config_path = self.config_dir.join("config.json");

        if !config_path.exists() {
            // Create default config
            let config = Config::default();
            self.save_config(&config)?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;

        let config: Config = serde_json::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config file: {}. Using defaults.", e);
            Config::default()
        });

        // Validate and clamp panel widths
        let mut validated_config = config;
        validated_config.toc_panel_width = validated_config.toc_panel_width.clamp(15, 60);
        validated_config.bookmarks_panel_width =
            validated_config.bookmarks_panel_width.clamp(20, 80);

        Ok(validated_config)
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        let config_path = self.config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    // Reading progress methods
    pub fn load_reading_progress(&self) -> Result<HashMap<String, ReadingProgress>> {
        let progress_path = self.config_dir.join("reading_progress.json");

        if !progress_path.exists() {
            return Ok(HashMap::new());
        }

        let content =
            fs::read_to_string(&progress_path).context("Failed to read reading progress file")?;

        let progress: HashMap<String, ReadingProgress> = serde_json::from_str(&content)
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to parse reading progress file: {}. Starting fresh.",
                    e
                );
                HashMap::new()
            });

        Ok(progress)
    }

    pub fn save_reading_progress(&self, progress: &HashMap<String, ReadingProgress>) -> Result<()> {
        let progress_path = self.config_dir.join("reading_progress.json");
        let content = serde_json::to_string_pretty(progress)
            .context("Failed to serialize reading progress")?;

        fs::write(&progress_path, content).context("Failed to write reading progress file")?;

        Ok(())
    }

    // Recent books methods
    pub fn load_recent_books(&self) -> Result<Vec<String>> {
        let recent_path = self.config_dir.join("recent_books.json");

        if !recent_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&recent_path).context("Failed to read recent books file")?;

        let books: Vec<String> = serde_json::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse recent books file: {}. Starting fresh.", e);
            Vec::new()
        });

        // Filter out books that no longer exist
        let existing_books: Vec<String> = books
            .into_iter()
            .filter(|path| std::path::Path::new(path).exists())
            .collect();

        Ok(existing_books)
    }

    pub fn save_recent_books(&self, books: &[String]) -> Result<()> {
        let recent_path = self.config_dir.join("recent_books.json");
        let content =
            serde_json::to_string_pretty(books).context("Failed to serialize recent books")?;

        fs::write(&recent_path, content).context("Failed to write recent books file")?;

        Ok(())
    }

    // Bookmark methods
    pub fn load_bookmarks(&self, book_path: &str) -> Result<Vec<Bookmark>> {
        let hash = compute_path_hash(book_path);
        let bookmarks_path = self.config_dir.join(format!("bookmarks_{}.json", hash));

        if !bookmarks_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&bookmarks_path).context("Failed to read bookmarks file")?;

        #[derive(Deserialize)]
        struct BookmarksFile {
            bookmarks: Vec<Bookmark>,
        }

        let file: BookmarksFile = serde_json::from_str(&content).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse bookmarks file: {}. Starting fresh.", e);
            BookmarksFile {
                bookmarks: Vec::new(),
            }
        });

        Ok(file.bookmarks)
    }

    pub fn save_bookmarks(&self, book_path: &str, bookmarks: &[Bookmark]) -> Result<()> {
        let hash = compute_path_hash(book_path);
        let bookmarks_path = self.config_dir.join(format!("bookmarks_{}.json", hash));

        #[derive(Serialize)]
        struct BookmarksFile<'a> {
            bookmarks: &'a [Bookmark],
        }

        let file = BookmarksFile { bookmarks };
        let content =
            serde_json::to_string_pretty(&file).context("Failed to serialize bookmarks")?;

        fs::write(&bookmarks_path, content).context("Failed to write bookmarks file")?;

        Ok(())
    }
}

// Compute SHA-256 hash of file path (first 16 hex chars)
fn compute_path_hash(path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{:016x}", hash)
}

// Helper function to canonicalize path
pub fn canonicalize_path(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);
    let canonical =
        fs::canonicalize(&path_buf).context(format!("Failed to canonicalize path: {}", path))?;

    canonical
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8"))
        .map(|s| s.to_string())
}
