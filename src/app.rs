use crate::types::{Book, Config, FocusTarget, Viewport};

pub struct AppState {
    pub book: Option<Book>,
    pub viewport: Viewport,
    pub current_chapter: usize,
    pub cursor_line: usize,
    pub focus: FocusTarget,
    pub config: Config,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
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
        }
    }

    pub fn load_book(&mut self, book: Book) {
        self.book = Some(book);
        self.current_chapter = 0;
        self.cursor_line = 0;
        self.viewport.scroll_offset = 0;
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
        // Reserve space for titlebar and statusbar
        self.viewport.height = height.saturating_sub(2);
    }
}
