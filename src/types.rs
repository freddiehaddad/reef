//! Core type definitions for the EPUB reader
//!
//! This module contains all the primary data structures used throughout
//! the application, including:
//! - Book, Chapter, and Section structures for EPUB content
//! - UI state types (UiMode, FocusTarget, LoadingState)
//! - Configuration and viewport types
//! - Search and bookmark types

/// Represents a parsed EPUB book with metadata and chapters
#[derive(Debug, Clone)]
pub struct Book {
    pub metadata: BookMetadata,
    pub chapters: Vec<Chapter>,
}

/// Represents a single chapter in an EPUB book
#[derive(Debug, Clone)]
pub struct Chapter {
    /// The chapter title extracted from TOC or heading
    pub title: String,
    /// Sub-sections within this chapter (h2/h3 headings)
    pub sections: Vec<Section>,
    /// Rendered lines of text ready for display
    pub content_lines: Vec<RenderedLine>,
    /// Original HTML file path or content (used for re-rendering)
    pub file_path: String,
}

/// Represents a section within a chapter (e.g., h2/h3 headings)
#[derive(Debug, Clone)]
pub struct Section {
    /// Section title from heading text
    pub title: String,
    /// Line number where this section starts in rendered content
    pub start_line: usize,
    /// Fragment identifier from EPUB TOC (e.g., "lexical-analysis" from "ch003.xhtml#lexical-analysis")
    pub fragment_id: Option<String>,
}

/// Manages table of contents tree state for the UI
pub struct TocState {
    pub tree_state: tui_tree_widget::TreeState<String>,
    pub items: Vec<tui_tree_widget::TreeItem<'static, String>>,
}

impl TocState {
    pub fn new() -> Self {
        TocState {
            tree_state: tui_tree_widget::TreeState::default(),
            items: Vec::new(),
        }
    }
}

impl Default for TocState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TocState {
    fn clone(&self) -> Self {
        TocState {
            tree_state: tui_tree_widget::TreeState::default(),
            items: self.items.clone(),
        }
    }
}

impl std::fmt::Debug for TocState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TocState")
            .field("items", &self.items)
            .finish()
    }
}

/// A single rendered line of text with styling and search match metadata
#[derive(Debug, Clone)]
pub struct RenderedLine {
    /// The actual text content
    pub text: String,
    /// Visual style to apply when rendering
    pub style: LineStyle,
    /// Character ranges (start, end) that match the current search query
    pub search_matches: Vec<(usize, usize)>,
    /// Inline text styles (start, end, style_type) for bold, italic, code, etc.
    pub inline_styles: Vec<(usize, usize, InlineStyle)>,
    /// Syntax highlighting color spans (start, end, color) for code blocks
    /// Each span defines a range of characters and their foreground color
    pub syntax_colors: Vec<(usize, usize, ratatui::style::Color)>,
}

/// Visual style options for rendering text lines
#[derive(Debug, Clone, PartialEq)]
pub enum LineStyle {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    CodeBlock { language: Option<String> },
    Quote,
    Link,
}

/// Inline text styling options (bold, italic, code, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum InlineStyle {
    Bold,
    Italic,
    Code,
    Underline,
    Strikethrough,
    Highlight,
}

/// EPUB metadata extracted from the book
#[derive(Debug, Clone)]
pub struct BookMetadata {
    pub title: String,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub publication_date: Option<String>,
    pub language: Option<String>,
}

/// Viewport configuration for rendering content
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Terminal width in columns
    pub width: u16,
    /// Terminal height in rows (excluding title/status bars)
    pub height: u16,
    /// Current vertical scroll position in lines
    pub scroll_offset: usize,
}

/// Indicates which UI panel currently has keyboard focus
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusTarget {
    Content,
    Toc,
    Bookmarks,
}

/// User-configurable settings persisted across sessions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Maximum line width for text wrapping (None = use full terminal width)
    pub max_width: Option<usize>,
    /// Width of the table of contents panel in columns
    pub toc_panel_width: u16,
    /// Width of the bookmarks panel in columns
    pub bookmarks_panel_width: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: None,
            toc_panel_width: 34,
            bookmarks_panel_width: 34,
        }
    }
}

/// Location of a search match within the book
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Index of the chapter containing this match
    pub chapter_idx: usize,
    /// Line number within the chapter
    pub line: usize,
    /// Character column where the match starts
    pub column: usize,
    /// Length of the matched text in characters
    pub match_length: usize,
}

/// User-created bookmark for quick navigation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    /// Chapter index where bookmark is located
    pub chapter_idx: usize,
    /// Line number within the chapter
    pub line: usize,
    /// User-provided label for this bookmark
    pub label: String,
}

/// Current UI mode determining which controls are active
#[derive(Debug, Clone, PartialEq)]
pub enum UiMode {
    /// Normal reading mode
    Normal,
    /// Search input dialog is open
    SearchPopup,
    /// Bookmark creation prompt is open
    BookmarkPrompt,
    /// Book selection dialog is open
    BookPicker,
    /// Help screen is displayed
    Help,
    /// Metadata information popup is displayed
    MetadataPopup,
    /// Error message popup with error text
    ErrorPopup(String),
}

/// Saved UI state for restoring after exiting zen mode
#[derive(Debug, Clone)]
pub struct ZenModeState {
    pub toc_visible: bool,
    pub bookmarks_visible: bool,
    pub statusbar_visible: bool,
    pub titlebar_visible: bool,
}

/// Loading state for background operations
#[derive(Debug, Clone)]
pub enum LoadingState {
    /// No background operation running
    Idle,
    /// Loading and parsing an EPUB file
    LoadingBook { file_path: String },
    /// Rendering chapters in background
    RenderingChapters { rendered: usize, total: usize },
}
