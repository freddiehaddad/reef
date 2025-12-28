//! Application state and core business logic
//!
//! This module contains the main application state (`AppState`) and all
//! the methods for managing UI state, navigation, and user interactions.

use crate::constants::{
    DEFAULT_TERMINAL_HEIGHT, DEFAULT_TERMINAL_WIDTH, WIDTH_PRESET_1, WIDTH_PRESET_2, WIDTH_PRESET_3,
};
use crate::persistence::{PersistenceManager, ReadingProgress};
use crate::toc::TocManager;
use crate::types::{
    Book, Bookmark, Config, FocusTarget, LoadingState, SearchMatch, TocState, UiMode, Viewport,
    ZenModeState,
};
use std::collections::{HashMap, HashSet};

/// Main application state containing all UI and data state
pub struct AppState {
    pub book: Option<Book>,
    pub viewport: Viewport,
    pub current_chapter: usize,
    pub cursor_line: usize,
    pub focus: FocusTarget,
    pub config: Config,
    pub should_quit: bool,

    // Max width can be temporarily overridden by CLI (not persisted)
    pub cli_max_width_override: Option<usize>,

    // UI Mode
    pub ui_mode: UiMode,
    pub previous_focus: Option<FocusTarget>,

    // Panels
    pub toc_panel_visible: bool,
    pub toc_state: TocState,
    pub toc_expanded_chapters: HashSet<String>,
    pub bookmarks_panel_visible: bool,
    pub selected_bookmark_idx: Option<usize>,
    pub titlebar_visible: bool,
    pub statusbar_visible: bool,

    // Zen Mode
    pub zen_mode_active: bool,
    pub pre_zen_state: Option<ZenModeState>,

    // Search
    pub search_query: String,
    pub search_results: Vec<SearchMatch>,
    pub current_search_idx: usize,
    pub input_buffer: String,

    // Bookmarks
    pub bookmarks: Vec<Bookmark>,

    // Persistence
    pub persistence: PersistenceManager,
    pub reading_progress: HashMap<String, ReadingProgress>,
    pub recent_books: Vec<String>,
    pub current_book_path: Option<String>,
    pub book_picker_selected_idx: Option<usize>,

    // Async task state
    pub loading_state: LoadingState,
}

impl AppState {
    /// Create a new application state with default settings
    pub fn new(config: Config, persistence: PersistenceManager) -> Self {
        let reading_progress = persistence.load_reading_progress().unwrap_or_default();
        let recent_books = persistence.load_recent_books().unwrap_or_default();

        AppState {
            book: None,
            viewport: Viewport {
                width: DEFAULT_TERMINAL_WIDTH,
                height: DEFAULT_TERMINAL_HEIGHT,
                scroll_offset: 0,
            },
            current_chapter: 0,
            cursor_line: 0,
            focus: FocusTarget::Content,
            config,
            should_quit: false,
            cli_max_width_override: None,
            ui_mode: UiMode::Normal,
            previous_focus: None,
            toc_panel_visible: false,
            toc_state: TocState::new(),
            toc_expanded_chapters: HashSet::new(),
            bookmarks_panel_visible: false,
            selected_bookmark_idx: None,
            titlebar_visible: true,
            statusbar_visible: true,
            zen_mode_active: false,
            pre_zen_state: None,
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_idx: 0,
            input_buffer: String::new(),
            bookmarks: Vec::new(),
            persistence,
            reading_progress,
            recent_books,
            current_book_path: None,
            book_picker_selected_idx: None,
            loading_state: LoadingState::Idle,
        }
    }

    fn build_toc_tree(&mut self, book: &Book) {
        self.toc_state.items = TocManager::build_tree(book);
    }

    /// Toggle the table of contents panel visibility
    pub fn toggle_toc(&mut self) {
        self.toc_panel_visible = !self.toc_panel_visible;
        log::debug!(
            "TOC panel toggled: {}",
            if self.toc_panel_visible {
                "visible"
            } else {
                "hidden"
            }
        );

        // If opening TOC panel, sync selection to current position
        if self.toc_panel_visible {
            self.sync_toc_to_cursor();
        }

        // If closing TOC panel while it has focus, move focus back to Content
        if !self.toc_panel_visible && self.focus == FocusTarget::Toc {
            self.focus = FocusTarget::Content;
        }

        // Re-render chapters to account for changed available width
        self.rerender_chapters();
    }

    /// Toggle the bookmarks panel visibility
    pub fn toggle_bookmarks(&mut self) {
        self.bookmarks_panel_visible = !self.bookmarks_panel_visible;

        // If closing bookmarks panel while it has focus, move focus back to Content
        if !self.bookmarks_panel_visible && self.focus == FocusTarget::Bookmarks {
            self.focus = FocusTarget::Content;
        }

        // Re-render chapters to account for changed available width
        self.rerender_chapters();
    }

    fn update_viewport_from_terminal(&mut self) {
        let (width, height) = crossterm::terminal::size()
            .unwrap_or((DEFAULT_TERMINAL_WIDTH, DEFAULT_TERMINAL_HEIGHT));
        self.update_viewport_size(width, height);
    }

    pub fn toggle_titlebar(&mut self) {
        self.titlebar_visible = !self.titlebar_visible;
        self.update_viewport_from_terminal();
    }

    pub fn toggle_statusbar(&mut self) {
        self.statusbar_visible = !self.statusbar_visible;
        self.update_viewport_from_terminal();
    }

    /// Toggle zen mode (hide all UI elements for distraction-free reading)
    pub fn toggle_zen_mode(&mut self) {
        if self.zen_mode_active {
            // Exiting zen mode - restore previous state
            if let Some(state) = &self.pre_zen_state {
                self.toc_panel_visible = state.toc_visible;
                self.bookmarks_panel_visible = state.bookmarks_visible;
                self.statusbar_visible = state.statusbar_visible;
                self.titlebar_visible = state.titlebar_visible;
            }
            self.zen_mode_active = false;
            self.pre_zen_state = None;
        } else {
            // Entering zen mode - save current state
            self.pre_zen_state = Some(ZenModeState {
                toc_visible: self.toc_panel_visible,
                bookmarks_visible: self.bookmarks_panel_visible,
                statusbar_visible: self.statusbar_visible,
                titlebar_visible: self.titlebar_visible,
            });

            // Hide everything
            self.toc_panel_visible = false;
            self.bookmarks_panel_visible = false;
            self.statusbar_visible = false;
            self.titlebar_visible = false;
            self.zen_mode_active = true;
        }

        // Update viewport height
        self.update_viewport_from_terminal();
    }

    /// Synchronize TOC selection to match current cursor position
    pub fn sync_toc_to_cursor(&mut self) {
        let book = match &self.book {
            Some(b) => b,
            None => return,
        };

        // Find the appropriate TOC item for current position
        let item_path =
            match TocManager::find_item_for_cursor(book, self.current_chapter, self.cursor_line) {
                Some(path) => path,
                None => return,
            };

        // If selecting a section (path has 2 elements), expand the parent chapter first
        if item_path.len() > 1 {
            TocManager::expand_parent(
                &mut self.toc_state,
                &mut self.toc_expanded_chapters,
                &item_path,
            );
        }

        // Select the item
        TocManager::select_item(&mut self.toc_state, item_path);
    }

    pub fn cycle_focus(&mut self) {
        // Cycle through open panels only
        self.focus = match self.focus {
            FocusTarget::Content => {
                if self.toc_panel_visible {
                    FocusTarget::Toc
                } else if self.bookmarks_panel_visible {
                    FocusTarget::Bookmarks
                } else {
                    FocusTarget::Content
                }
            }
            FocusTarget::Toc => {
                if self.bookmarks_panel_visible {
                    FocusTarget::Bookmarks
                } else {
                    FocusTarget::Content
                }
            }
            FocusTarget::Bookmarks => FocusTarget::Content,
        };
    }

    pub fn focus_toc(&mut self) {
        if self.toc_panel_visible {
            self.focus = FocusTarget::Toc;
        }
    }

    pub fn focus_content(&mut self) {
        self.focus = FocusTarget::Content;
    }

    pub fn focus_bookmarks(&mut self) {
        if self.bookmarks_panel_visible {
            self.focus = FocusTarget::Bookmarks;
        }
    }

    pub fn toc_next(&mut self) {
        self.toc_state.tree_state.key_down();
    }

    pub fn toc_previous(&mut self) {
        self.toc_state.tree_state.key_up();
    }

    pub fn toc_open(&mut self) {
        // Get selected item before toggling
        if let Some(selected_id) = self.toc_state.tree_state.selected().first() {
            // Check if this is a chapter (not a section) and has sections (is expandable)
            if selected_id.starts_with("chapter_") && !selected_id.contains("_section_") {
                // Check if chapter has sections by extracting chapter index
                if let Some(chapter_idx) = selected_id
                    .strip_prefix("chapter_")
                    .and_then(|s| s.parse::<usize>().ok())
                    && let Some(chapter) =
                        self.book.as_ref().and_then(|b| b.chapters.get(chapter_idx))
                    && !chapter.sections.is_empty()
                {
                    // Toggle expansion state in our tracking
                    // If currently expanded, it will collapse; if collapsed, it will expand
                    if self.toc_expanded_chapters.contains(selected_id) {
                        self.toc_expanded_chapters.remove(selected_id);
                    } else {
                        self.toc_expanded_chapters.insert(selected_id.clone());
                    }
                }
            }
        }

        self.toc_state.tree_state.toggle_selected();
    }

    /// Close/collapse the currently selected TOC item
    pub fn toc_close(&mut self) {
        if let Some(selected) = self.toc_state.tree_state.selected().first() {
            let selected_id = selected.clone();
            let selected_vec = vec![selected_id.clone()];
            self.toc_state.tree_state.close(&selected_vec);

            // Track collapse in our state
            self.toc_expanded_chapters.remove(&selected_id);
        }
    }

    /// Jump to the position of the currently selected TOC item
    pub fn toc_select(&mut self) {
        // Get selected item ID - use LAST element of path for leaf nodes (sections)
        let selected_id = match self.toc_state.tree_state.selected().last() {
            Some(id) => id.clone(),
            None => return,
        };

        log::debug!("TOC select: selected_id = {}", selected_id);

        // Parse the ID to determine chapter and optional section
        let (chapter_idx, section_idx) = match TocManager::parse_item_id(&selected_id) {
            Some(parsed) => parsed,
            None => {
                log::debug!("TOC select: failed to parse item ID");
                return;
            }
        };

        log::debug!(
            "TOC select: chapter_idx = {}, section_idx = {:?}",
            chapter_idx,
            section_idx
        );

        // Validate chapter index
        if chapter_idx >= self.total_chapters() {
            log::debug!(
                "TOC select: invalid chapter index {} >= {}",
                chapter_idx,
                self.total_chapters()
            );
            return;
        }

        self.current_chapter = chapter_idx;

        if let Some(sec_idx) = section_idx {
            // Jump to section start
            let section_start_line = self
                .book
                .as_ref()
                .and_then(|b| b.chapters.get(chapter_idx))
                .and_then(|ch| {
                    log::debug!("TOC select: chapter has {} sections", ch.sections.len());
                    ch.sections.get(sec_idx)
                })
                .map(|s| {
                    log::debug!(
                        "TOC select: section '{}' has start_line = {}, fragment_id = {:?}",
                        s.title,
                        s.start_line,
                        s.fragment_id
                    );
                    s.start_line
                });

            if let Some(start_line) = section_start_line {
                log::debug!("TOC select: jumping to section at line {}", start_line);
                self.cursor_line = start_line;
                self.viewport.scroll_offset = start_line;
            } else {
                log::debug!("TOC select: section not found (sec_idx = {})", sec_idx);
            }
        } else {
            // Jump to chapter start
            log::debug!("TOC select: jumping to chapter start");
            self.cursor_line = 0;
            self.viewport.scroll_offset = 0;
        }
    }

    /// Get the currently displayed chapter
    pub fn get_current_chapter(&self) -> Option<&crate::types::Chapter> {
        self.book
            .as_ref()
            .and_then(|b| b.chapters.get(self.current_chapter))
    }

    /// Get total number of chapters in the current book
    pub fn total_chapters(&self) -> usize {
        self.book.as_ref().map(|b| b.chapters.len()).unwrap_or(0)
    }

    /// Get the number of lines in the current chapter
    pub fn current_chapter_lines(&self) -> usize {
        self.get_current_chapter()
            .map(|ch| ch.content_lines.len())
            .unwrap_or(0)
    }

    /// Scroll down by the specified number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        let max_lines = self.current_chapter_lines();
        if max_lines == 0 {
            return;
        }

        let max_scroll = max_lines.saturating_sub(self.viewport.height as usize);
        self.viewport.scroll_offset = (self.viewport.scroll_offset + lines).min(max_scroll);

        // Cursor follows viewport
        self.clamp_cursor_to_viewport();

        // Sync TOC if visible (to update highlighting as we scroll through sections)
        if self.toc_panel_visible {
            self.sync_toc_to_cursor();
        }
    }

    /// Scroll up by the specified number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.viewport.scroll_offset = self.viewport.scroll_offset.saturating_sub(lines);

        // Cursor follows viewport
        self.clamp_cursor_to_viewport();

        // Sync TOC if visible (to update highlighting as we scroll through sections)
        if self.toc_panel_visible {
            self.sync_toc_to_cursor();
        }
    }

    pub fn move_cursor_to_top(&mut self) {
        self.cursor_line = self.viewport.scroll_offset;
    }

    pub fn move_cursor_to_middle(&mut self) {
        let middle = self.viewport.scroll_offset + (self.viewport.height as usize / 2);
        let max_line = self.current_chapter_lines().saturating_sub(1);
        self.cursor_line = middle.min(max_line);
    }

    pub fn move_cursor_to_bottom(&mut self) {
        let bottom = self.viewport.scroll_offset + self.viewport.height as usize - 1;
        let max_line = self.current_chapter_lines().saturating_sub(1);
        self.cursor_line = bottom.min(max_line);
    }

    pub fn move_cursor_to_chapter_start(&mut self) {
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;
    }

    pub fn move_cursor_to_chapter_end(&mut self) {
        let max_line = self.current_chapter_lines().saturating_sub(1);
        self.cursor_line = max_line;

        // Scroll to show the end
        let viewport_height = self.viewport.height as usize;
        if max_line >= viewport_height {
            self.viewport.scroll_offset = max_line.saturating_sub(viewport_height - 1);
        }
    }

    /// Navigate to the next chapter (wraps to first chapter)
    pub fn next_chapter(&mut self) {
        let total = self.total_chapters();
        if total == 0 {
            return;
        }

        let old_chapter = self.current_chapter;
        self.current_chapter = (self.current_chapter + 1) % total;
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;

        log::debug!(
            "Chapter navigation: {} -> {} (next)",
            old_chapter,
            self.current_chapter
        );

        // Sync TOC to new position
        self.sync_toc_to_cursor();
    }

    /// Navigate to the previous chapter (wraps to last chapter)
    pub fn previous_chapter(&mut self) {
        let total = self.total_chapters();
        if total == 0 {
            return;
        }

        let old_chapter = self.current_chapter;
        if self.current_chapter == 0 {
            self.current_chapter = total - 1;
        } else {
            self.current_chapter -= 1;
        }

        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;

        log::debug!(
            "Chapter navigation: {} -> {} (previous)",
            old_chapter,
            self.current_chapter
        );

        // Sync TOC to new position
        self.sync_toc_to_cursor();
    }

    pub fn next_section(&mut self) {
        if let Some(chapter) = self.get_current_chapter() {
            if chapter.sections.is_empty() {
                // No sections, jump to next chapter
                self.next_chapter();
                return;
            }

            // Find current section
            let current_section_idx = chapter
                .sections
                .iter()
                .position(|s| s.start_line > self.cursor_line)
                .unwrap_or(chapter.sections.len());

            if current_section_idx < chapter.sections.len() {
                // Jump to next section in current chapter
                let target_line = chapter.sections[current_section_idx].start_line;
                self.cursor_line = target_line;
                self.viewport.scroll_offset = target_line;
            } else {
                // At last section, jump to next chapter
                self.next_chapter();
                return; // next_chapter already syncs TOC
            }
        }

        // Sync TOC to new position
        self.sync_toc_to_cursor();
    }

    pub fn previous_section(&mut self) {
        if let Some(chapter) = self.get_current_chapter() {
            if chapter.sections.is_empty() {
                // No sections, jump to previous chapter
                self.previous_chapter();
                return;
            }

            // Find current section - we're in the section if start_line <= cursor_line < next_start_line
            let mut current_section_idx = None;
            for (idx, section) in chapter.sections.iter().enumerate() {
                if section.start_line <= self.cursor_line {
                    let next_start = chapter
                        .sections
                        .get(idx + 1)
                        .map(|s| s.start_line)
                        .unwrap_or(usize::MAX);
                    if self.cursor_line < next_start {
                        current_section_idx = Some(idx);
                        break;
                    }
                }
            }

            match current_section_idx {
                Some(0) => {
                    // At first section, jump to previous chapter
                    self.previous_chapter();
                    return; // previous_chapter already syncs TOC
                }
                Some(idx) => {
                    // Jump to previous section
                    let target_line = chapter.sections[idx - 1].start_line;
                    self.cursor_line = target_line;
                    self.viewport.scroll_offset = target_line;
                }
                None => {
                    // Before first section or no current section, jump to previous chapter
                    self.previous_chapter();
                    return; // previous_chapter already syncs TOC
                }
            }
        }

        // Sync TOC to new position
        self.sync_toc_to_cursor();
    }

    pub fn page_down(&mut self) {
        let page_size = self.viewport.height as usize;
        self.scroll_down(page_size);
    }

    pub fn page_up(&mut self) {
        let page_size = self.viewport.height as usize;
        self.scroll_up(page_size);
    }

    pub fn half_page_down(&mut self) {
        let half_page = (self.viewport.height as usize) / 2;
        self.scroll_down(half_page);
    }

    pub fn half_page_up(&mut self) {
        let half_page = (self.viewport.height as usize) / 2;
        self.scroll_up(half_page);
    }

    fn clamp_cursor_to_viewport(&mut self) {
        let max_line = self.current_chapter_lines().saturating_sub(1);
        let viewport_start = self.viewport.scroll_offset;
        let viewport_end = self.viewport.scroll_offset + self.viewport.height as usize - 1;

        // Keep cursor within current viewport
        if self.cursor_line < viewport_start {
            self.cursor_line = viewport_start;
        } else if self.cursor_line > viewport_end {
            self.cursor_line = viewport_end.min(max_line);
        }

        // Ensure cursor is within valid range
        self.cursor_line = self.cursor_line.min(max_line);
    }

    pub fn update_viewport_size(&mut self, width: u16, height: u16) {
        self.viewport.width = width;
        // Reserve space for titlebar and statusbar if visible
        let reserved_height = (if self.titlebar_visible { 1 } else { 0 })
            + (if self.statusbar_visible { 1 } else { 0 });
        self.viewport.height = height.saturating_sub(reserved_height);
    }

    /// Get the effective max width (CLI override takes precedence over config)
    pub fn effective_max_width(&self) -> Option<usize> {
        self.cli_max_width_override.or(self.config.max_width)
    }

    /// Cycle through max width presets: None -> 80 -> 100 -> 120 -> None
    pub fn cycle_max_width(&mut self) {
        let current = self.config.max_width;
        self.config.max_width = match current {
            None => Some(WIDTH_PRESET_1),
            Some(w) if w == WIDTH_PRESET_1 => Some(WIDTH_PRESET_2),
            Some(w) if w == WIDTH_PRESET_2 => Some(WIDTH_PRESET_3),
            Some(w) if w == WIDTH_PRESET_3 => None,
            Some(_) => None, // Reset unknown values to None
        };

        // Re-render chapters with new width
        self.rerender_chapters();
    }

    /// Re-render all chapters with current effective width
    /// Call this when max-width changes or panel visibility changes
    fn rerender_chapters(&mut self) {
        // Get effective width before borrowing book mutably
        let effective_width = self.effective_max_width();

        // Calculate available content width accounting for panels and margins
        let mut available_width = self.viewport.width;

        // Subtract TOC panel width and margin if visible
        if self.toc_panel_visible {
            available_width = available_width.saturating_sub(self.config.toc_panel_width + 1);
        }

        // Subtract bookmarks panel width and margin if visible
        if self.bookmarks_panel_visible {
            available_width = available_width.saturating_sub(self.config.bookmarks_panel_width + 1);
        }

        // Re-render all chapters with available width if we have a book
        if let Some(book) = &mut self.book {
            for chapter in &mut book.chapters {
                crate::epub::render_chapter(chapter, effective_width, available_width);
            }

            // Re-apply search highlights if there are active results
            if !self.search_results.is_empty() {
                // Re-run search to recalculate match positions in new line structure
                let search_query = self.search_query.clone();
                if let Ok(new_results) = crate::search::SearchEngine::search(book, &search_query) {
                    self.search_results = new_results;
                    crate::search::SearchEngine::apply_highlights(book, &self.search_results);
                }
            }
        }
    }

    // Bookmark methods
    pub fn bookmark_next(&mut self) {
        self.selected_bookmark_idx =
            crate::bookmarks::BookmarkManager::next(&self.bookmarks, self.selected_bookmark_idx);
    }

    pub fn bookmark_previous(&mut self) {
        self.selected_bookmark_idx = crate::bookmarks::BookmarkManager::previous(
            &self.bookmarks,
            self.selected_bookmark_idx,
        );
    }

    pub fn jump_to_selected_bookmark(&mut self) {
        if let Some((chapter_idx, line, scroll_offset)) =
            crate::bookmarks::BookmarkManager::get_jump_position(
                &self.bookmarks,
                self.selected_bookmark_idx,
                &self.viewport,
            )
        {
            self.current_chapter = chapter_idx;
            self.cursor_line = line;
            self.viewport.scroll_offset = scroll_offset;

            // Sync TOC
            self.sync_toc_to_cursor();
        }
    }

    pub fn delete_selected_bookmark(&mut self) {
        self.selected_bookmark_idx = crate::bookmarks::BookmarkManager::delete(
            &mut self.bookmarks,
            self.selected_bookmark_idx,
        );
    }

    // Search methods
    pub fn next_search_result(&mut self) {
        if let Some((new_idx, chapter_idx, line, scroll_offset)) =
            crate::search::SearchEngine::next_result(
                &self.search_results,
                self.current_search_idx,
                &self.viewport,
            )
        {
            self.current_search_idx = new_idx;
            self.current_chapter = chapter_idx;
            self.cursor_line = line;
            self.viewport.scroll_offset = scroll_offset;

            // Sync TOC
            self.sync_toc_to_cursor();
        }
    }

    pub fn previous_search_result(&mut self) {
        if let Some((new_idx, chapter_idx, line, scroll_offset)) =
            crate::search::SearchEngine::previous_result(
                &self.search_results,
                self.current_search_idx,
                &self.viewport,
            )
        {
            self.current_search_idx = new_idx;
            self.current_chapter = chapter_idx;
            self.cursor_line = line;
            self.viewport.scroll_offset = scroll_offset;

            // Sync TOC
            self.sync_toc_to_cursor();
        }
    }

    // Persistence methods
    /// Save current reading state, bookmarks, and configuration to disk
    pub fn save_state(&mut self) -> anyhow::Result<()> {
        // Save current book progress if we have one
        if let Some(book_path) = &self.current_book_path {
            let progress = ReadingProgress {
                chapter_idx: self.current_chapter,
                line: self.cursor_line,
                scroll_offset: self.viewport.scroll_offset,
                last_read: chrono::Utc::now(),
                toc_expansion_state: self.get_toc_expansion_state(),
            };

            self.reading_progress.insert(book_path.clone(), progress);

            // Save bookmarks for current book
            self.persistence
                .save_bookmarks(book_path, &self.bookmarks)?;
        }

        // Save reading progress
        self.persistence
            .save_reading_progress(&self.reading_progress)?;

        // Save recent books
        self.persistence.save_recent_books(&self.recent_books)?;

        // Save config
        self.persistence.save_config(&self.config)?;

        Ok(())
    }

    /// Load a book from file path and restore reading progress
    pub fn load_book_with_path(&mut self, book_path: String) -> anyhow::Result<()> {
        use crate::persistence::canonicalize_path;

        log::info!("Loading book: {}", book_path);

        // Clear search state when switching books
        self.search_query.clear();
        self.search_results.clear();
        self.current_search_idx = 0;

        // Canonicalize the path
        let canonical_path = canonicalize_path(&book_path)?;
        log::debug!("Canonical path: {}", canonical_path);

        // Add to recent books (or move to top if already present)
        if let Some(pos) = self.recent_books.iter().position(|p| p == &canonical_path) {
            log::debug!(
                "Book already in recent list at position {}, moving to top",
                pos
            );
            self.recent_books.remove(pos);
        }
        self.recent_books.insert(0, canonical_path.clone());

        // Load the EPUB
        let book = crate::epub::parse_epub(&book_path)?;
        log::info!("EPUB parsed: {} chapters", book.chapters.len());

        // Load bookmarks for this book
        let bookmarks = self
            .persistence
            .load_bookmarks(&canonical_path)
            .unwrap_or_default();
        log::debug!("Loaded {} bookmarks for this book", bookmarks.len());
        self.bookmarks = bookmarks;

        // Load and clone reading progress to avoid borrow issues
        let progress = self.reading_progress.get(&canonical_path).cloned();

        // Store current book path
        self.current_book_path = Some(canonical_path);

        // Build TOC tree before storing the book
        self.build_toc_tree(&book);
        log::debug!("TOC tree built: {} items", self.toc_state.items.len());

        // Restore position if we have progress
        if let Some(progress) = progress {
            log::info!(
                "Restoring reading progress: chapter {}, line {}",
                progress.chapter_idx,
                progress.line
            );
            self.current_chapter = progress
                .chapter_idx
                .min(book.chapters.len().saturating_sub(1));
            self.cursor_line = progress.line;
            self.viewport.scroll_offset = progress.scroll_offset;

            // Restore TOC expansion state
            self.restore_toc_expansion_state(&progress.toc_expansion_state);
        } else {
            log::debug!("No reading progress found, starting at beginning");
            self.current_chapter = 0;
            self.cursor_line = 0;
            self.viewport.scroll_offset = 0;
        }

        self.book = Some(book);

        // Sync TOC to restored position
        self.sync_toc_to_cursor();

        Ok(())
    }

    fn get_toc_expansion_state(&self) -> Vec<String> {
        // Return list of expanded chapter IDs from our tracking
        self.toc_expanded_chapters.iter().cloned().collect()
    }

    fn restore_toc_expansion_state(&mut self, state: &[String]) {
        // Clear current tracking
        self.toc_expanded_chapters.clear();

        // Expand nodes that were previously expanded and track them
        for id in state {
            self.toc_state.tree_state.open(vec![id.clone()]);
            self.toc_expanded_chapters.insert(id.clone());
        }
    }
}
