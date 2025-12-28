//! Full-text search functionality for EPUB content
//!
//! This module provides regex-based search across all chapters and lines,
//! with highlighting support and result navigation.

use crate::types::{Book, SearchMatch, Viewport};
use regex::Regex;
use std::time::{Duration, Instant};

const MAX_SEARCH_RESULTS: usize = 1000;
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Search engine for full-text regex search across EPUB content
pub struct SearchEngine;

impl SearchEngine {
    /// Perform full-book search with regex pattern
    ///
    /// Searches through all chapters and lines, collecting up to
    /// MAX_SEARCH_RESULTS matches or timing out after SEARCH_TIMEOUT.
    ///
    /// # Arguments
    /// * `book` - The book to search through
    /// * `query` - Regex pattern (supports standard Rust regex syntax)
    ///
    /// # Returns
    /// * `Ok(Vec<SearchMatch>)` - List of matches found
    /// * `Err(String)` - Invalid regex or search timeout
    pub fn search(book: &Book, query: &str) -> Result<Vec<SearchMatch>, String> {
        log::info!("Starting search: query='{}'", query);

        // Validate and compile regex
        let regex = Regex::new(query).map_err(|e| {
            log::warn!("Invalid regex pattern '{}': {}", query, e);
            format!("Invalid regex pattern: {}", e)
        })?;

        let mut results = Vec::new();
        let start_time = Instant::now();

        // Search through all chapters
        for (chapter_idx, chapter) in book.chapters.iter().enumerate() {
            // Check timeout
            if start_time.elapsed() > SEARCH_TIMEOUT {
                log::warn!(
                    "Search timed out after {:?} ({} results found)",
                    start_time.elapsed(),
                    results.len()
                );
                return Err("Search cancelled (timeout)".to_string());
            }

            // Search through all lines in the chapter
            for (line_idx, rendered_line) in chapter.content_lines.iter().enumerate() {
                // Find all matches in this line
                for mat in regex.find_iter(&rendered_line.text) {
                    results.push(SearchMatch {
                        chapter_idx,
                        line: line_idx,
                        column: mat.start(),
                        match_length: mat.end() - mat.start(),
                    });

                    // Stop if we've hit the limit
                    if results.len() >= MAX_SEARCH_RESULTS {
                        log::warn!(
                            "Search hit maximum result limit ({} results)",
                            MAX_SEARCH_RESULTS
                        );
                        return Ok(results);
                    }
                }
            }
        }

        log::info!(
            "Search completed: found {} matches in {:?}",
            results.len(),
            start_time.elapsed()
        );
        Ok(results)
    }

    /// Navigate to the next search result
    ///
    /// # Returns
    /// * `Some((new_idx, chapter_idx, line, scroll_offset))` - New position to jump to
    /// * `None` - No search results available
    pub fn next_result(
        results: &[SearchMatch],
        current_idx: usize,
        viewport: &Viewport,
    ) -> Option<(usize, usize, usize, usize)> {
        if results.is_empty() {
            return None;
        }

        let new_idx = (current_idx + 1) % results.len();
        SearchEngine::get_jump_position(results, new_idx, viewport)
    }

    /// Navigate to the previous search result
    ///
    /// # Returns
    /// * `Some((new_idx, chapter_idx, line, scroll_offset))` - New position to jump to
    /// * `None` - No search results available
    pub fn previous_result(
        results: &[SearchMatch],
        current_idx: usize,
        viewport: &Viewport,
    ) -> Option<(usize, usize, usize, usize)> {
        if results.is_empty() {
            return None;
        }

        let new_idx = if current_idx == 0 {
            results.len() - 1
        } else {
            current_idx - 1
        };
        SearchEngine::get_jump_position(results, new_idx, viewport)
    }

    /// Get the position to jump to for a search result
    fn get_jump_position(
        results: &[SearchMatch],
        idx: usize,
        viewport: &Viewport,
    ) -> Option<(usize, usize, usize, usize)> {
        results.get(idx).map(|result| {
            let half_viewport = viewport.height as usize / 2;
            let scroll_offset = result.line.saturating_sub(half_viewport);
            (idx, result.chapter_idx, result.line, scroll_offset)
        })
    }

    /// Apply search match highlighting to rendered lines
    /// Updates the search_matches field in RenderedLine structs
    pub fn apply_highlights(book: &mut Book, results: &[SearchMatch]) {
        log::debug!("Applying search highlights: {} matches", results.len());

        // First, clear all existing highlights
        for chapter in &mut book.chapters {
            for line in &mut chapter.content_lines {
                line.search_matches.clear();
            }
        }

        // Apply new highlights
        for result in results {
            if let Some(chapter) = book.chapters.get_mut(result.chapter_idx)
                && let Some(line) = chapter.content_lines.get_mut(result.line)
            {
                line.search_matches
                    .push((result.column, result.column + result.match_length));
            }
        }

        log::debug!("Search highlights applied successfully");
    }

    /// Clear all search highlights from the book
    pub fn clear_highlights(book: &mut Book) {
        log::debug!("Clearing all search highlights");

        for chapter in &mut book.chapters {
            for line in &mut chapter.content_lines {
                line.search_matches.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BookMetadata, Chapter, LineStyle, RenderedLine};

    fn create_test_book() -> Book {
        Book {
            metadata: BookMetadata {
                title: "Test Book".to_string(),
                author: Some("Test Author".to_string()),
                publisher: None,
                publication_date: None,
                language: None,
            },
            chapters: vec![Chapter {
                title: "Chapter 1".to_string(),
                sections: vec![],
                content_lines: vec![
                    RenderedLine {
                        text: "This is a test line".to_string(),
                        style: LineStyle::Normal,
                        search_matches: vec![],
                        inline_styles: vec![],
                        syntax_colors: vec![],
                    },
                    RenderedLine {
                        text: "Another test line here".to_string(),
                        style: LineStyle::Normal,
                        search_matches: vec![],
                        inline_styles: vec![],
                        syntax_colors: vec![],
                    },
                ],
                file_path: "ch1.xhtml".to_string(),
            }],
        }
    }

    #[test]
    fn test_simple_search() {
        let book = create_test_book();
        let results = SearchEngine::search(&book, "test").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].match_length, 4);
        assert_eq!(results[1].match_length, 4);
    }

    #[test]
    fn test_case_insensitive_search() {
        let book = create_test_book();
        let results = SearchEngine::search(&book, "(?i)TEST").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_invalid_regex() {
        let book = create_test_book();
        let result = SearchEngine::search(&book, "[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_matches() {
        let book = create_test_book();
        let results = SearchEngine::search(&book, "nonexistent").unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_navigation() {
        let book = create_test_book();
        let results = SearchEngine::search(&book, "test").unwrap();
        let viewport = crate::types::Viewport {
            width: 80,
            height: 24,
            scroll_offset: 0,
        };

        // Test next
        let (new_idx, _, _, _) = SearchEngine::next_result(&results, 0, &viewport).unwrap();
        assert_eq!(new_idx, 1);

        // Test wrapping
        let (new_idx, _, _, _) = SearchEngine::next_result(&results, 1, &viewport).unwrap();
        assert_eq!(new_idx, 0);

        // Test previous
        let (new_idx, _, _, _) = SearchEngine::previous_result(&results, 1, &viewport).unwrap();
        assert_eq!(new_idx, 0);

        // Test wrapping backward
        let (new_idx, _, _, _) = SearchEngine::previous_result(&results, 0, &viewport).unwrap();
        assert_eq!(new_idx, 1);
    }
}
