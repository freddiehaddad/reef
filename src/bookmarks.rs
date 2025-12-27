use crate::types::Bookmark;

const MAX_BOOKMARKS: usize = 1000;
const MAX_LABEL_LENGTH: usize = 100;

pub struct BookmarkManager;

impl BookmarkManager {
    /// Add a new bookmark
    /// Returns an error if the maximum number of bookmarks has been reached
    /// or if the label is empty or exceeds the maximum length
    pub fn add_bookmark(
        bookmarks: &mut Vec<Bookmark>,
        chapter_idx: usize,
        line: usize,
        label: String,
    ) -> Result<(), String> {
        // Validate label
        let trimmed_label = label.trim();
        if trimmed_label.is_empty() {
            return Err("Bookmark label cannot be empty".to_string());
        }
        
        if trimmed_label.len() > MAX_LABEL_LENGTH {
            return Err(format!(
                "Bookmark label too long (max {} characters)",
                MAX_LABEL_LENGTH
            ));
        }
        
        // Check bookmark limit
        if bookmarks.len() >= MAX_BOOKMARKS {
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
        bookmarks.sort_by(|a, b| {
            a.chapter_idx
                .cmp(&b.chapter_idx)
                .then(a.line.cmp(&b.line))
        });
        
        Ok(())
    }
    
    /// Generate auto-suggested label from current line text
    /// Returns first 50 characters of the line, with newlines stripped
    /// Returns None if the line is empty
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
    
    /// Delete bookmark at index
    /// Returns the deleted bookmark if successful
    pub fn delete_bookmark(bookmarks: &mut Vec<Bookmark>, index: usize) -> Option<Bookmark> {
        if index < bookmarks.len() {
            Some(bookmarks.remove(index))
        } else {
            None
        }
    }
    
    /// Find bookmark index for a specific chapter and line
    /// Returns None if no bookmark found at that position
    pub fn find_bookmark_at_position(
        bookmarks: &[Bookmark],
        chapter_idx: usize,
        line: usize,
    ) -> Option<usize> {
        bookmarks
            .iter()
            .position(|b| b.chapter_idx == chapter_idx && b.line == line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_bookmark() {
        let mut bookmarks = Vec::new();
        let result = BookmarkManager::add_bookmark(
            &mut bookmarks,
            0,
            10,
            "Test bookmark".to_string(),
        );
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

    #[test]
    fn test_delete_bookmark() {
        let mut bookmarks = vec![
            Bookmark {
                chapter_idx: 0,
                line: 10,
                label: "First".to_string(),
            },
            Bookmark {
                chapter_idx: 1,
                line: 20,
                label: "Second".to_string(),
            },
        ];

        let deleted = BookmarkManager::delete_bookmark(&mut bookmarks, 0);
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().label, "First");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].label, "Second");
    }

    #[test]
    fn test_find_bookmark_at_position() {
        let bookmarks = vec![
            Bookmark {
                chapter_idx: 0,
                line: 10,
                label: "First".to_string(),
            },
            Bookmark {
                chapter_idx: 1,
                line: 20,
                label: "Second".to_string(),
            },
        ];

        let found = BookmarkManager::find_bookmark_at_position(&bookmarks, 1, 20);
        assert_eq!(found, Some(1));

        let not_found = BookmarkManager::find_bookmark_at_position(&bookmarks, 2, 30);
        assert_eq!(not_found, None);
    }
}
