use crate::constants::{
    MAX_BOOKMARKS_PANEL_WIDTH, MAX_TOC_PANEL_WIDTH, MIN_BOOKMARKS_PANEL_WIDTH, MIN_TOC_PANEL_WIDTH,
};
use crate::types::{Bookmark, Config};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Reading position and state for a specific book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingProgress {
    pub chapter_idx: usize,
    pub line: usize,
    pub scroll_offset: usize,
    pub last_read: DateTime<Utc>,
    pub toc_expansion_state: Vec<String>,
}

/// Manages persistent storage of reading progress, bookmarks, and configuration
pub struct PersistenceManager {
    config_dir: PathBuf,
}

impl PersistenceManager {
    /// Create a new persistence manager
    /// Initializes the config directory if it doesn't exist
    pub fn new() -> Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "reef").context("Failed to determine config directory")?;

        let config_dir = project_dirs.config_dir().to_path_buf();

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        Ok(PersistenceManager { config_dir })
    }

    // Config methods
    /// Load user configuration from disk
    /// Creates default config if file doesn't exist
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
            log::warn!("Failed to parse config file: {}. Using defaults.", e);
            Config::default()
        });

        // Validate and clamp panel widths
        let mut validated_config = config;
        validated_config.toc_panel_width = validated_config
            .toc_panel_width
            .clamp(MIN_TOC_PANEL_WIDTH, MAX_TOC_PANEL_WIDTH);
        validated_config.bookmarks_panel_width = validated_config
            .bookmarks_panel_width
            .clamp(MIN_BOOKMARKS_PANEL_WIDTH, MAX_BOOKMARKS_PANEL_WIDTH);

        Ok(validated_config)
    }

    /// Save user configuration to disk
    pub fn save_config(&self, config: &Config) -> Result<()> {
        let config_path = self.config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    // Reading progress methods
    /// Load reading progress for all books
    /// Returns empty map if file doesn't exist or can't be parsed
    pub fn load_reading_progress(&self) -> Result<HashMap<String, ReadingProgress>> {
        let progress_path = self.config_dir.join("reading_progress.json");

        if !progress_path.exists() {
            return Ok(HashMap::new());
        }

        let content =
            fs::read_to_string(&progress_path).context("Failed to read reading progress file")?;

        let progress: HashMap<String, ReadingProgress> = serde_json::from_str(&content)
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to parse reading progress file: {}. Starting fresh.",
                    e
                );
                HashMap::new()
            });

        Ok(progress)
    }

    /// Save reading progress for all books
    pub fn save_reading_progress(&self, progress: &HashMap<String, ReadingProgress>) -> Result<()> {
        let progress_path = self.config_dir.join("reading_progress.json");
        let content = serde_json::to_string_pretty(progress)
            .context("Failed to serialize reading progress")?;

        fs::write(&progress_path, content).context("Failed to write reading progress file")?;

        Ok(())
    }

    // Recent books methods
    /// Load list of recently opened books
    /// Filters out books that no longer exist on disk
    pub fn load_recent_books(&self) -> Result<Vec<String>> {
        let recent_path = self.config_dir.join("recent_books.json");

        if !recent_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(&recent_path).context("Failed to read recent books file")?;

        let books: Vec<String> = serde_json::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse recent books file: {}. Starting fresh.", e);
            Vec::new()
        });

        // Filter out books that no longer exist
        let existing_books: Vec<String> = books
            .into_iter()
            .filter(|path| std::path::Path::new(path).exists())
            .collect();

        Ok(existing_books)
    }

    /// Save list of recently opened books
    pub fn save_recent_books(&self, books: &[String]) -> Result<()> {
        let recent_path = self.config_dir.join("recent_books.json");
        let content =
            serde_json::to_string_pretty(books).context("Failed to serialize recent books")?;

        fs::write(&recent_path, content).context("Failed to write recent books file")?;

        Ok(())
    }

    // Bookmark methods
    /// Load bookmarks for a specific book
    /// Returns empty list if no bookmarks exist
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
            log::warn!("Failed to parse bookmarks file: {}. Starting fresh.", e);
            BookmarksFile {
                bookmarks: Vec::new(),
            }
        });

        Ok(file.bookmarks)
    }

    /// Save bookmarks for a specific book
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

// Compute hash of file path for creating unique bookmark files
fn compute_path_hash(path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{:016x}", hash)
}

/// Convert a file path to its canonical absolute form
/// This ensures consistent path representation across sessions
pub fn canonicalize_path(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);
    let canonical =
        fs::canonicalize(&path_buf).context(format!("Failed to canonicalize path: {}", path))?;

    canonical
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid UTF-8"))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (PersistenceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PersistenceManager {
            config_dir: temp_dir.path().to_path_buf(),
        };
        (manager, temp_dir)
    }

    #[test]
    fn test_save_and_load_config() {
        let (manager, _temp) = create_test_manager();

        let config = Config {
            max_width: Some(100),
            toc_panel_width: 35,
            bookmarks_panel_width: 40,
        };

        manager.save_config(&config).unwrap();
        let loaded = manager.load_config().unwrap();

        assert_eq!(loaded.max_width, Some(100));
        assert_eq!(loaded.toc_panel_width, 35);
        assert_eq!(loaded.bookmarks_panel_width, 40);
    }

    #[test]
    fn test_save_and_load_reading_progress() {
        let (manager, _temp) = create_test_manager();

        let mut progress = HashMap::new();
        progress.insert(
            "/path/to/book.epub".to_string(),
            ReadingProgress {
                chapter_idx: 5,
                line: 42,
                scroll_offset: 30,
                last_read: chrono::Utc::now(),
                toc_expansion_state: vec!["chapter_0".to_string()],
            },
        );

        manager.save_reading_progress(&progress).unwrap();
        let loaded = manager.load_reading_progress().unwrap();

        assert_eq!(loaded.len(), 1);
        let book_progress = loaded.get("/path/to/book.epub").unwrap();
        assert_eq!(book_progress.chapter_idx, 5);
        assert_eq!(book_progress.line, 42);
        assert_eq!(book_progress.scroll_offset, 30);
    }

    #[test]
    fn test_save_and_load_bookmarks() {
        let (manager, _temp) = create_test_manager();

        let bookmarks = vec![
            Bookmark {
                chapter_idx: 0,
                line: 10,
                label: "Important point".to_string(),
            },
            Bookmark {
                chapter_idx: 2,
                line: 50,
                label: "Remember this".to_string(),
            },
        ];

        manager
            .save_bookmarks("/path/to/book.epub", &bookmarks)
            .unwrap();
        let loaded = manager.load_bookmarks("/path/to/book.epub").unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].label, "Important point");
        assert_eq!(loaded[1].label, "Remember this");
    }

    #[test]
    fn test_load_nonexistent_bookmarks() {
        let (manager, _temp) = create_test_manager();
        let loaded = manager.load_bookmarks("/nonexistent/book.epub").unwrap();
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    fn test_recent_books_filtering() {
        let (manager, _temp) = create_test_manager();

        // Create a temporary file to represent a real book
        let temp_book = _temp.path().join("real_book.epub");
        fs::write(&temp_book, b"fake epub").unwrap();

        let books = vec![
            temp_book.to_string_lossy().to_string(),
            "/nonexistent/book.epub".to_string(), // This should be filtered out
        ];

        manager.save_recent_books(&books).unwrap();
        let loaded = manager.load_recent_books().unwrap();

        // Only the real file should be loaded
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].contains("real_book.epub"));
    }

    #[test]
    fn test_config_validation() {
        let (manager, _temp) = create_test_manager();

        // Save config with out-of-bounds panel widths
        let config = Config {
            max_width: None,
            toc_panel_width: 5,         // Too small
            bookmarks_panel_width: 100, // Too large
        };

        manager.save_config(&config).unwrap();
        let loaded = manager.load_config().unwrap();

        // Should be clamped to valid ranges
        assert!(loaded.toc_panel_width >= 15 && loaded.toc_panel_width <= 60);
        assert!(loaded.bookmarks_panel_width >= 20 && loaded.bookmarks_panel_width <= 80);
    }

    #[test]
    fn test_path_hash_consistency() {
        let path = "/some/path/to/book.epub";
        let hash1 = compute_path_hash(path);
        let hash2 = compute_path_hash(path);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 16);
    }
}
