mod app;
mod bookmarks;
mod cli;
mod epub;
mod error;
mod persistence;
mod search;
mod types;
mod ui;

use app::AppState;
use cli::Cli;
use clap::Parser;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use error::{AppError, Result};
use persistence::PersistenceManager;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use types::{Config, UiMode};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Validate CLI arguments
    cli.validate().map_err(|e| AppError::Other(e))?;

    // Initialize logging if requested
    if let Some(log_file) = &cli.log_file {
        init_logging(log_file)?;
        tracing::info!("EPUB Reader starting");
    }

    // Check terminal size
    let (width, height) = crossterm::terminal::size()?;
    if width < 80 || height < 24 {
        return Err(AppError::TerminalTooSmall);
    }

    // Setup terminal
    setup_terminal()?;

    // Setup Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| AppError::Other(format!("Failed to set Ctrl-C handler: {}", e)))?;

    // Run the application
    let result = run_app(cli, running);

    // Cleanup terminal
    cleanup_terminal()?;

    result
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;

    // Set panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = cleanup_terminal();
        original_hook(panic_info);
    }));

    Ok(())
}

fn cleanup_terminal() -> Result<()> {
    execute!(io::stdout(), Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn init_logging(log_file: &str) -> Result<()> {
    let path = std::path::Path::new(log_file);
    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    let filename = path.file_name().unwrap();

    let file_appender = RollingFileAppender::new(Rotation::NEVER, parent, filename);

    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("epub_reader=debug".parse().unwrap()),
        )
        .init();

    Ok(())
}

fn run_app(cli: Cli, running: Arc<AtomicBool>) -> Result<()> {
    // Create backend
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Initialize persistence manager
    let persistence = PersistenceManager::new()
        .map_err(|e| AppError::Other(format!("Failed to initialize persistence: {}", e)))?;
    
    // Load config
    let mut config = persistence.load_config()
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}. Using defaults.", e);
            Config::default()
        });
    
    // Override with CLI arguments
    if let Some(max_width) = cli.max_width {
        config.max_width = Some(max_width);
    }

    let mut app = AppState::new(config, persistence);
    
    // Get terminal size and update viewport
    let (width, height) = crossterm::terminal::size()?;
    app.update_viewport_size(width, height);

    // Load EPUB if file argument provided
    if let Some(file_path) = &cli.file {
        tracing::info!("Loading EPUB: {}", file_path);
        
        let mut book = epub::parse_epub(file_path)?;
        
        tracing::info!(
            "Book loaded: {} ({} chapters)",
            book.metadata.title,
            book.chapters.len()
        );

        // Render all chapters
        for chapter in &mut book.chapters {
            epub::render_chapter(chapter, app.config.max_width, app.viewport.width);
        }

        // Load book with path (handles persistence)
        app.load_book_with_path(file_path.clone())
            .map_err(|e| AppError::Other(format!("Failed to load book: {}", e)))?;
        
        // Re-render with actual book content (in case of resize)
        if let Some(book) = &mut app.book {
            for chapter in &mut book.chapters {
                epub::render_chapter(chapter, app.config.max_width, app.viewport.width);
            }
        }
    } else {
        // No file provided - check if we have recent books
        if app.recent_books.is_empty() {
            return Err(AppError::Other("No recent books. Usage: epub-reader <file.epub>".to_string()));
        }
        
        // Show book picker
        app.ui_mode = UiMode::BookPicker;
        app.book_picker_selected_idx = Some(0);
    }

    // Main event loop
    while running.load(Ordering::SeqCst) && !app.should_quit {
        // Render
        terminal.draw(|f| {
            ui::layout::render(f, &mut app);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    ui::handle_key_event(&mut app, key)?;
                }
                Event::Resize(width, height) => {
                    app.update_viewport_size(width, height);
                    
                    // Re-render all chapters with new width
                    if let Some(book) = &mut app.book {
                        for chapter in &mut book.chapters {
                            epub::render_chapter(chapter, app.config.max_width, app.viewport.width);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Save state before quitting
    if let Err(e) = app.save_state() {
        tracing::error!("Failed to save state: {}", e);
    }

    tracing::info!("EPUB Reader shutting down");
    Ok(())
}
