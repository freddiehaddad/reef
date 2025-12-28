//! Application-wide constants

/// Minimum terminal width required to run the application
pub const MIN_TERMINAL_WIDTH: u16 = 80;

/// Minimum terminal height required to run the application
pub const MIN_TERMINAL_HEIGHT: u16 = 24;

/// Default terminal width if size detection fails
pub const DEFAULT_TERMINAL_WIDTH: u16 = 80;

/// Default terminal height if size detection fails
pub const DEFAULT_TERMINAL_HEIGHT: u16 = 24;

/// Maximum length for search input buffer (characters)
pub const MAX_SEARCH_INPUT_LENGTH: usize = 500;

/// Maximum length for bookmark label input (characters)
pub const MAX_BOOKMARK_INPUT_LENGTH: usize = 100;

/// Minimum allowed value for max_width CLI parameter
pub const MIN_MAX_WIDTH: usize = 40;

/// Maximum allowed value for max_width CLI parameter
pub const MAX_MAX_WIDTH: usize = 200;

/// Reserved columns for margins and UI elements
pub const UI_MARGIN_WIDTH: usize = 4;

/// Width preset for first cycle position
pub const WIDTH_PRESET_1: usize = 80;

/// Width preset for second cycle position
pub const WIDTH_PRESET_2: usize = 100;

/// Width preset for third cycle position
pub const WIDTH_PRESET_3: usize = 120;

/// Minimum width for TOC panel
pub const MIN_TOC_PANEL_WIDTH: u16 = 15;

/// Maximum width for TOC panel
pub const MAX_TOC_PANEL_WIDTH: u16 = 60;

/// Minimum width for bookmarks panel
pub const MIN_BOOKMARKS_PANEL_WIDTH: u16 = 20;

/// Maximum width for bookmarks panel
pub const MAX_BOOKMARKS_PANEL_WIDTH: u16 = 80;

/// Frame duration in milliseconds for the UI render loop (targeting 60 FPS)
pub const FRAME_DURATION_MS: u64 = 16;

/// Debounce timeout for terminal resize events in milliseconds
pub const RESIZE_DEBOUNCE_MS: u64 = 200;
