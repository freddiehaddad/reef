use crate::types::Bookmark;

const MAX_BOOKMARKS: usize = 1000;
const MAX_LABEL_LENGTH: usize = 100;

/// Utilities for managing user bookmarks
pub struct BookmarkManager;

impl BookmarkManager {
    /// Add a new bookmark to the collection
    ///
    /// Bookmarks are automatically sorted by position (chapter, then line).
    ///
    /// # Arguments
    /// * `bookmarks` - Mutable bookmark collection to add to
    /// * `chapter_idx` - Chapter index where bookmark should be placed
    /// * `line` - Line number within chapter
    /// * `label` - User-provided label (will be trimmed)
    ///
    /// # Returns
    /// * `Ok(())` - Bookmark added successfully
    /// * `Err(String)` - Empty label, too long, or max bookmarks reached
    pub fn add_bookmark(
        bookmarks: &mut Vec<Bookmark>,
        chapter_idx: usize,
        line: usize,
        label: String,
    ) -> Result<(), String> {
        log::debug!(
            "Adding bookmark: chapter={}, line={}, label='{}'",
            chapter_idx,
            line,
            label
        );

        // Validate label
        let trimmed_label = label.trim();
        if trimmed_label.is_empty() {
            log::warn!("Bookmark creation failed: empty label");
            return Err("Bookmark label cannot be empty".to_string());
        }

        if trimmed_label.len() > MAX_LABEL_LENGTH {
            log::warn!(
                "Bookmark creation failed: label too long ({} > {})",
                trimmed_label.len(),
                MAX_LABEL_LENGTH
            );
            return Err(format!(
                "Bookmark label too long (max {} characters)",
                MAX_LABEL_LENGTH
            ));
        }

        // Check bookmark limit
        if bookmarks.len() >= MAX_BOOKMARKS {
            log::warn!(
                "Bookmark creation failed: max bookmarks ({}) reached",
                MAX_BOOKMARKS
            );
            return Err(format!("Maximum bookmarks ({}) reached", MAX_BOOKMARKS));
        }

        // Create and add bookmark
        let bookmark = Bookmark {
            chapter_idx,
            line,
            label: trimmed_label.to_string(),
        };

        bookmarks.push(bookmark);

        // Sort bookmarks by position (chapter, then line)
        bookmarks.sort_by(|a, b| a.chapter_idx.cmp(&b.chapter_idx).then(a.line.cmp(&b.line)));

        log::info!(
            "Bookmark added successfully: '{}' at chapter {}, line {} ({} total bookmarks)",
            trimmed_label,
            chapter_idx,
            line,
            bookmarks.len()
        );
        Ok(())
    }

    /// Generate an auto-suggested label from current line or chapter
    ///
    /// Uses the first 50 characters of the line text. If the line is empty,
    /// falls back to the chapter title.
    ///
    /// # Returns
    /// * `Some(String)` - Suggested label text
    /// * `None` - Both line and chapter title are empty
    pub fn generate_label_suggestion(line_text: &str, chapter_title: &str) -> Option<String> {
        let trimmed = line_text.trim();

        if trimmed.is_empty() {
            // If line is empty, try chapter title
            let chapter_trimmed = chapter_title.trim();
            if chapter_trimmed.is_empty() {
                None
            } else {
                Some(Self::truncate_label(chapter_trimmed, 50))
            }
        } else {
            Some(Self::truncate_label(trimmed, 50))
        }
    }

    /// Truncate label to maximum length with "..." suffix if needed
    fn truncate_label(text: &str, max_len: usize) -> String {
        // Remove newlines first
        let single_line: String = text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();

        let trimmed = single_line.trim();

        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            format!("{}...", &trimmed[..max_len.saturating_sub(3)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_bookmark() {
        let mut bookmarks = Vec::new();
        let result =
            BookmarkManager::add_bookmark(&mut bookmarks, 0, 10, "Test bookmark".to_string());
        assert!(result.is_ok());
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].label, "Test bookmark");
    }

    #[test]
    fn test_empty_label_rejected() {
        let mut bookmarks = Vec::new();
        let result = BookmarkManager::add_bookmark(&mut bookmarks, 0, 10, "   ".to_string());
        assert!(result.is_err());
        assert_eq!(bookmarks.len(), 0);
    }

    #[test]
    fn test_bookmark_sorting() {
        let mut bookmarks = Vec::new();
        BookmarkManager::add_bookmark(&mut bookmarks, 1, 20, "Second".to_string()).unwrap();
        BookmarkManager::add_bookmark(&mut bookmarks, 0, 10, "First".to_string()).unwrap();
        BookmarkManager::add_bookmark(&mut bookmarks, 1, 5, "Third".to_string()).unwrap();

        assert_eq!(bookmarks[0].label, "First");
        assert_eq!(bookmarks[1].label, "Third");
        assert_eq!(bookmarks[2].label, "Second");
    }

    #[test]
    fn test_label_truncation() {
        let long_text = "a".repeat(100);
        let result = BookmarkManager::truncate_label(&long_text, 50);
        assert_eq!(result.len(), 50);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_generate_label_from_text() {
        let suggestion =
            BookmarkManager::generate_label_suggestion("This is a test line", "Chapter 1");
        assert_eq!(suggestion, Some("This is a test line".to_string()));
    }

    #[test]
    fn test_generate_label_from_chapter_title() {
        let suggestion = BookmarkManager::generate_label_suggestion("", "Chapter 1");
        assert_eq!(suggestion, Some("Chapter 1".to_string()));
    }

    #[test]
    fn test_generate_label_empty() {
        let suggestion = BookmarkManager::generate_label_suggestion("", "");
        assert_eq!(suggestion, None);
    }
}
