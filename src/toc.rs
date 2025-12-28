//! Table of Contents management and synchronization

use crate::types::{Book, Chapter, TocState};
use std::collections::HashSet;
use tui_tree_widget::TreeItem;

/// Helper for managing TOC tree state and synchronization
pub struct TocManager;

impl TocManager {
    /// Build the TOC tree from book chapters
    pub fn build_tree(book: &Book) -> Vec<TreeItem<'static, String>> {
        let mut items = Vec::new();

        for (chapter_idx, chapter) in book.chapters.iter().enumerate() {
            let chapter_id = Self::make_chapter_id(chapter_idx);

            if chapter.sections.is_empty() {
                // Chapter with no sections
                items.push(TreeItem::new_leaf(chapter_id, chapter.title.clone()));
            } else {
                // Chapter with sections
                let section_items = Self::build_section_items(chapter_idx, chapter);
                items.push(
                    TreeItem::new(chapter_id, chapter.title.clone(), section_items)
                        .expect("Failed to create tree item"),
                );
            }
        }

        items
    }

    /// Build section items for a chapter
    fn build_section_items(
        chapter_idx: usize,
        chapter: &Chapter,
    ) -> Vec<TreeItem<'static, String>> {
        chapter
            .sections
            .iter()
            .enumerate()
            .map(|(section_idx, section)| {
                let section_id = Self::make_section_id(chapter_idx, section_idx);
                TreeItem::new_leaf(section_id, section.title.clone())
            })
            .collect()
    }

    /// Determine which TOC item should be selected based on cursor position
    pub fn find_item_for_cursor(
        book: &Book,
        current_chapter: usize,
        cursor_line: usize,
    ) -> Option<Vec<String>> {
        let chapter = book.chapters.get(current_chapter)?;

        if chapter.sections.is_empty() {
            // No sections, select the chapter
            Some(vec![Self::make_chapter_id(current_chapter)])
        } else {
            // Find which section contains the cursor
            let section_idx = Self::find_section_at_line(chapter, cursor_line);

            if let Some(sec_idx) = section_idx {
                // Cursor is in a section - return path with both parent and child
                let chapter_id = Self::make_chapter_id(current_chapter);
                let section_id = Self::make_section_id(current_chapter, sec_idx);
                Some(vec![chapter_id, section_id])
            } else {
                // Cursor is before first section, select the chapter
                Some(vec![Self::make_chapter_id(current_chapter)])
            }
        }
    }

    /// Find the section index that contains the given line
    fn find_section_at_line(chapter: &Chapter, cursor_line: usize) -> Option<usize> {
        for (idx, section) in chapter.sections.iter().enumerate() {
            let next_start = chapter
                .sections
                .get(idx + 1)
                .map(|s| s.start_line)
                .unwrap_or(usize::MAX);

            if section.start_line <= cursor_line && cursor_line < next_start {
                return Some(idx);
            }
        }
        None
    }

    /// Expand a parent chapter in the tree state
    pub fn expand_parent(
        toc_state: &mut TocState,
        expanded_chapters: &mut HashSet<String>,
        item_path: &[String],
    ) {
        if let Some(chapter_id) = item_path.first() {
            if !expanded_chapters.contains(chapter_id) {
                toc_state.tree_state.open(vec![chapter_id.clone()]);
                expanded_chapters.insert(chapter_id.clone());
            }
        }
    }

    /// Select an item in the TOC tree
    pub fn select_item(toc_state: &mut TocState, item_path: Vec<String>) {
        toc_state.tree_state.select(item_path);
    }

    /// Make a chapter ID string
    #[inline]
    fn make_chapter_id(chapter_idx: usize) -> String {
        format!("chapter_{}", chapter_idx)
    }

    /// Make a section ID string
    #[inline]
    fn make_section_id(chapter_idx: usize, section_idx: usize) -> String {
        format!("chapter_{}_section_{}", chapter_idx, section_idx)
    }

    /// Parse a TOC item ID to extract chapter and optional section indices
    pub fn parse_item_id(item_id: &str) -> Option<(usize, Option<usize>)> {
        if !item_id.starts_with("chapter_") {
            return None;
        }

        let parts: Vec<&str> = item_id.split('_').collect();

        if parts.len() == 2 {
            // Just a chapter ID: "chapter_0"
            parts[1].parse::<usize>().ok().map(|ch| (ch, None))
        } else if parts.len() == 4 && parts[2] == "section" {
            // Section ID: "chapter_0_section_1"
            let chapter_idx = parts[1].parse::<usize>().ok()?;
            let section_idx = parts[3].parse::<usize>().ok()?;
            Some((chapter_idx, Some(section_idx)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BookMetadata, Section};

    fn create_test_book() -> Book {
        Book {
            metadata: BookMetadata {
                title: "Test Book".to_string(),
                author: None,
                publisher: None,
                publication_date: None,
                language: None,
            },
            chapters: vec![
                Chapter {
                    title: "Chapter 1".to_string(),
                    sections: vec![
                        Section {
                            title: "Section 1.1".to_string(),
                            start_line: 10,
                        },
                        Section {
                            title: "Section 1.2".to_string(),
                            start_line: 50,
                        },
                    ],
                    content_lines: vec![],
                    file_path: String::new(),
                },
                Chapter {
                    title: "Chapter 2".to_string(),
                    sections: vec![],
                    content_lines: vec![],
                    file_path: String::new(),
                },
            ],
        }
    }

    #[test]
    fn test_build_tree() {
        let book = create_test_book();
        let items = TocManager::build_tree(&book);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_find_item_for_cursor_in_section() {
        let book = create_test_book();
        let path = TocManager::find_item_for_cursor(&book, 0, 30).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], "chapter_0");
        assert_eq!(path[1], "chapter_0_section_0");
    }

    #[test]
    fn test_find_item_for_cursor_before_sections() {
        let book = create_test_book();
        let path = TocManager::find_item_for_cursor(&book, 0, 5).unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "chapter_0");
    }

    #[test]
    fn test_find_item_for_cursor_no_sections() {
        let book = create_test_book();
        let path = TocManager::find_item_for_cursor(&book, 1, 10).unwrap();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "chapter_1");
    }

    #[test]
    fn test_parse_chapter_id() {
        let result = TocManager::parse_item_id("chapter_5");
        assert_eq!(result, Some((5, None)));
    }

    #[test]
    fn test_parse_section_id() {
        let result = TocManager::parse_item_id("chapter_3_section_2");
        assert_eq!(result, Some((3, Some(2))));
    }

    #[test]
    fn test_parse_invalid_id() {
        let result = TocManager::parse_item_id("invalid_id");
        assert_eq!(result, None);
    }
}
