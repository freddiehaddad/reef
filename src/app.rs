use crate::types::{Book, Bookmark, Config, FocusTarget, SearchMatch, TocState, UiMode, Viewport, ZenModeState};
use crate::persistence::{PersistenceManager, ReadingProgress};
use std::collections::HashMap;

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
}

impl AppState {
    pub fn new(config: Config, persistence: PersistenceManager) -> Self {
        let reading_progress = persistence.load_reading_progress().unwrap_or_default();
        let recent_books = persistence.load_recent_books().unwrap_or_default();
        
        AppState {
            book: None,
            viewport: Viewport {
                width: 80,
                height: 24,
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
        }
    }

    pub fn load_book(&mut self, book: Book) {
        // Build TOC tree before storing the book
        self.build_toc_tree(&book);
        
        self.book = Some(book);
        self.current_chapter = 0;
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;
        
        // Sync TOC to initial position
        self.sync_toc_to_cursor();
    }

    fn build_toc_tree(&mut self, book: &Book) {
        use tui_tree_widget::TreeItem;
        
        let mut items = Vec::new();
        
        for (chapter_idx, chapter) in book.chapters.iter().enumerate() {
            let chapter_id = format!("chapter_{}", chapter_idx);
            
            if chapter.sections.is_empty() {
                // Chapter with no sections
                items.push(TreeItem::new_leaf(chapter_id, chapter.title.clone()));
            } else {
                // Chapter with sections
                let mut section_items = Vec::new();
                for (section_idx, section) in chapter.sections.iter().enumerate() {
                    let section_id = format!("chapter_{}_section_{}", chapter_idx, section_idx);
                    section_items.push(TreeItem::new_leaf(section_id, section.title.clone()));
                }
                items.push(TreeItem::new(chapter_id, chapter.title.clone(), section_items).expect("Failed to create tree item"));
            }
        }
        
        self.toc_state.items = items;
        // Don't select first here - let sync_toc_to_cursor handle it
    }

    pub fn toggle_toc(&mut self) {
        self.toc_panel_visible = !self.toc_panel_visible;
        
        // If opening TOC panel, sync selection to current position
        if self.toc_panel_visible {
            self.sync_toc_to_cursor();
        }
        
        // If closing TOC panel while it has focus, move focus back to Content
        if !self.toc_panel_visible && self.focus == FocusTarget::TOC {
            self.focus = FocusTarget::Content;
        }
    }

    pub fn toggle_bookmarks(&mut self) {
        self.bookmarks_panel_visible = !self.bookmarks_panel_visible;
        
        // If closing bookmarks panel while it has focus, move focus back to Content
        if !self.bookmarks_panel_visible && self.focus == FocusTarget::Bookmarks {
            self.focus = FocusTarget::Content;
        }
    }

    pub fn toggle_titlebar(&mut self) {
        self.titlebar_visible = !self.titlebar_visible;
        
        // Update viewport height
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        self.update_viewport_size(width, height);
    }

    pub fn toggle_statusbar(&mut self) {
        self.statusbar_visible = !self.statusbar_visible;
        
        // Update viewport height
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        self.update_viewport_size(width, height);
    }

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
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        self.update_viewport_size(width, height);
    }

    /// Synchronize TOC selection to match current cursor position
    pub fn sync_toc_to_cursor(&mut self) {
        if self.book.is_none() {
            return;
        }

        // Determine which TOC item should be selected based on cursor position
        let target_id = if let Some(chapter) = self.get_current_chapter() {
            if chapter.sections.is_empty() {
                // No sections, select the chapter
                format!("chapter_{}", self.current_chapter)
            } else {
                // Find which section contains the cursor
                let mut current_section_idx = None;
                for (idx, section) in chapter.sections.iter().enumerate() {
                    let next_start = chapter.sections.get(idx + 1)
                        .map(|s| s.start_line)
                        .unwrap_or(usize::MAX);
                    if section.start_line <= self.cursor_line && self.cursor_line < next_start {
                        current_section_idx = Some(idx);
                        break;
                    }
                }

                if let Some(sec_idx) = current_section_idx {
                    // Cursor is in a section
                    format!("chapter_{}_section_{}", self.current_chapter, sec_idx)
                } else {
                    // Cursor is before first section, select the chapter
                    format!("chapter_{}", self.current_chapter)
                }
            }
        } else {
            return;
        };

        // Update tree state selection to the target ID
        self.toc_state.tree_state.select(vec![target_id]);
    }

    pub fn cycle_focus(&mut self) {
        // Cycle through open panels only
        self.focus = match self.focus {
            FocusTarget::Content => {
                if self.toc_panel_visible {
                    FocusTarget::TOC
                } else if self.bookmarks_panel_visible {
                    FocusTarget::Bookmarks
                } else {
                    FocusTarget::Content
                }
            }
            FocusTarget::TOC => {
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
            self.focus = FocusTarget::TOC;
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
        self.toc_state.tree_state.toggle_selected();
    }

    pub fn toc_close(&mut self) {
        if let Some(selected) = self.toc_state.tree_state.selected().first() {
            let selected_vec = vec![selected.clone()];
            self.toc_state.tree_state.close(&selected_vec);
        }
    }

    pub fn toc_select(&mut self) {
        // Get selected item ID
        if let Some(selected_id) = self.toc_state.tree_state.selected().first() {
            // Parse the ID to determine chapter and section
            if selected_id.starts_with("chapter_") {
                let parts: Vec<&str> = selected_id.split('_').collect();
                if parts.len() == 2 {
                    // Just a chapter ID: "chapter_0"
                    if let Ok(chapter_idx) = parts[1].parse::<usize>() {
                        if chapter_idx < self.total_chapters() {
                            self.current_chapter = chapter_idx;
                            self.cursor_line = 0;
                            self.viewport.scroll_offset = 0;
                        }
                    }
                } else if parts.len() == 4 && parts[2] == "section" {
                    // Section ID: "chapter_0_section_1"
                    if let (Ok(chapter_idx), Ok(section_idx)) = (
                        parts[1].parse::<usize>(),
                        parts[3].parse::<usize>(),
                    ) {
                        if chapter_idx < self.total_chapters() {
                            self.current_chapter = chapter_idx;
                            
                            // Get section start_line before borrowing self mutably
                            let section_start_line = self.book.as_ref()
                                .and_then(|b| b.chapters.get(chapter_idx))
                                .and_then(|ch| ch.sections.get(section_idx))
                                .map(|s| s.start_line);
                                
                            if let Some(start_line) = section_start_line {
                                self.cursor_line = start_line;
                                self.viewport.scroll_offset = start_line;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_current_chapter(&self) -> Option<&crate::types::Chapter> {
        self.book.as_ref()
            .and_then(|b| b.chapters.get(self.current_chapter))
    }

    pub fn get_current_chapter_mut(&mut self) -> Option<&mut crate::types::Chapter> {
        self.book.as_mut()
            .and_then(|b| b.chapters.get_mut(self.current_chapter))
    }

    pub fn total_chapters(&self) -> usize {
        self.book.as_ref().map(|b| b.chapters.len()).unwrap_or(0)
    }

    pub fn current_chapter_lines(&self) -> usize {
        self.get_current_chapter()
            .map(|ch| ch.content_lines.len())
            .unwrap_or(0)
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let max_lines = self.current_chapter_lines();
        if max_lines == 0 {
            return;
        }

        let max_scroll = max_lines.saturating_sub(self.viewport.height as usize);
        self.viewport.scroll_offset = (self.viewport.scroll_offset + lines).min(max_scroll);
        
        // Cursor follows viewport
        self.clamp_cursor_to_viewport();
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.viewport.scroll_offset = self.viewport.scroll_offset.saturating_sub(lines);
        
        // Cursor follows viewport
        self.clamp_cursor_to_viewport();
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

    pub fn next_chapter(&mut self) {
        let total = self.total_chapters();
        if total == 0 {
            return;
        }
        
        self.current_chapter = (self.current_chapter + 1) % total;
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;
        
        // Sync TOC to new position
        self.sync_toc_to_cursor();
    }

    pub fn previous_chapter(&mut self) {
        let total = self.total_chapters();
        if total == 0 {
            return;
        }
        
        if self.current_chapter == 0 {
            self.current_chapter = total - 1;
        } else {
            self.current_chapter -= 1;
        }
        
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;
        
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
            let current_section_idx = chapter.sections.iter().position(|s| {
                s.start_line > self.cursor_line
            }).unwrap_or(chapter.sections.len());

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
                    let next_start = chapter.sections.get(idx + 1)
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
        let reserved_height = 
            (if self.titlebar_visible { 1 } else { 0 }) +
            (if self.statusbar_visible { 1 } else { 0 });
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
            None => Some(80),
            Some(80) => Some(100),
            Some(100) => Some(120),
            Some(120) => None,
            Some(_) => None, // Reset unknown values to None
        };
        
        // Get effective width before borrowing book mutably
        let effective_width = self.effective_max_width();
        let viewport_width = self.viewport.width;
        
        // Re-render all chapters with new width if we have a book
        if let Some(book) = &mut self.book {
            for chapter in &mut book.chapters {
                crate::epub::render_chapter(chapter, effective_width, viewport_width);
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
        if self.bookmarks.is_empty() {
            return;
        }
        
        let current_idx = self.selected_bookmark_idx.unwrap_or(0);
        let next_idx = (current_idx + 1) % self.bookmarks.len();
        self.selected_bookmark_idx = Some(next_idx);
    }

    pub fn bookmark_previous(&mut self) {
        if self.bookmarks.is_empty() {
            return;
        }
        
        let current_idx = self.selected_bookmark_idx.unwrap_or(0);
        let next_idx = if current_idx == 0 {
            self.bookmarks.len() - 1
        } else {
            current_idx - 1
        };
        self.selected_bookmark_idx = Some(next_idx);
    }

    pub fn jump_to_selected_bookmark(&mut self) {
        if let Some(idx) = self.selected_bookmark_idx {
            if let Some(bookmark) = self.bookmarks.get(idx) {
                self.current_chapter = bookmark.chapter_idx;
                self.cursor_line = bookmark.line;
                
                // Center the line in viewport
                let half_viewport = self.viewport.height as usize / 2;
                self.viewport.scroll_offset = bookmark.line.saturating_sub(half_viewport);
                
                // Sync TOC
                self.sync_toc_to_cursor();
            }
        }
    }

    pub fn delete_selected_bookmark(&mut self) {
        if let Some(idx) = self.selected_bookmark_idx {
            if idx < self.bookmarks.len() {
                self.bookmarks.remove(idx);
                
                // Update selection
                if self.bookmarks.is_empty() {
                    self.selected_bookmark_idx = None;
                } else if idx >= self.bookmarks.len() {
                    // Was at last bookmark, move to previous
                    self.selected_bookmark_idx = Some(self.bookmarks.len() - 1);
                }
                // Otherwise, keep same index (moves to next bookmark)
            }
        }
    }

    // Search methods
    pub fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        self.current_search_idx = (self.current_search_idx + 1) % self.search_results.len();
        self.jump_to_current_search_result();
    }

    pub fn previous_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        
        if self.current_search_idx == 0 {
            self.current_search_idx = self.search_results.len() - 1;
        } else {
            self.current_search_idx -= 1;
        }
        self.jump_to_current_search_result();
    }

    fn jump_to_current_search_result(&mut self) {
        if let Some(result) = self.search_results.get(self.current_search_idx) {
            self.current_chapter = result.chapter_idx;
            self.cursor_line = result.line;
            
            // Center the line in viewport
            let half_viewport = self.viewport.height as usize / 2;
            self.viewport.scroll_offset = result.line.saturating_sub(half_viewport);
            
            // Sync TOC
            self.sync_toc_to_cursor();
        }
    }

    // Persistence methods
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
            self.persistence.save_bookmarks(book_path, &self.bookmarks)?;
        }
        
        // Save reading progress
        self.persistence.save_reading_progress(&self.reading_progress)?;
        
        // Save recent books
        self.persistence.save_recent_books(&self.recent_books)?;
        
        // Save config
        self.persistence.save_config(&self.config)?;
        
        Ok(())
    }
    
    pub fn load_book_with_path(&mut self, book_path: String) -> anyhow::Result<()> {
        use crate::persistence::canonicalize_path;
        
        // Clear search state when switching books
        self.search_query.clear();
        self.search_results.clear();
        self.current_search_idx = 0;
        
        // Canonicalize the path
        let canonical_path = canonicalize_path(&book_path)?;
        
        // Add to recent books (or move to top if already present)
        if let Some(pos) = self.recent_books.iter().position(|p| p == &canonical_path) {
            self.recent_books.remove(pos);
        }
        self.recent_books.insert(0, canonical_path.clone());
        
        // Load the EPUB
        let book = crate::epub::parse_epub(&book_path)?;
        
        // Load bookmarks for this book
        let bookmarks = self.persistence.load_bookmarks(&canonical_path).unwrap_or_default();
        self.bookmarks = bookmarks;
        
        // Load and clone reading progress to avoid borrow issues
        let progress = self.reading_progress.get(&canonical_path).cloned();
        
        // Store current book path
        self.current_book_path = Some(canonical_path);
        
        // Build TOC tree before storing the book
        self.build_toc_tree(&book);
        
        // Restore position if we have progress
        if let Some(progress) = progress {
            self.current_chapter = progress.chapter_idx.min(book.chapters.len().saturating_sub(1));
            self.cursor_line = progress.line;
            self.viewport.scroll_offset = progress.scroll_offset;
            
            // Restore TOC expansion state
            self.restore_toc_expansion_state(&progress.toc_expansion_state);
        } else {
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
        // Get list of expanded node identifiers
        // For now, we'll return an empty list since TreeState doesn't expose
        // the internal state easily. This can be enhanced later.
        Vec::new()
    }
    
    fn restore_toc_expansion_state(&mut self, state: &[String]) {
        // Expand nodes that were previously expanded
        for id in state {
            self.toc_state.tree_state.open(vec![id.clone()]);
        }
    }
}
