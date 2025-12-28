mod app;
mod async_tasks;
mod bookmarks;
mod cli;
mod constants;
mod epub;
mod error;
mod persistence;
mod search;
mod toc;
mod types;
mod ui;

use app::AppState;
use async_tasks::{AsyncTaskRunner, TaskMessage};
use clap::Parser;
use cli::Cli;
use constants::{FRAME_DURATION_MS, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, RESIZE_DEBOUNCE_MS};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use error::{AppError, Result};
use persistence::PersistenceManager;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use types::{Config, LoadingState, UiMode};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Validate CLI arguments
    cli.validate().map_err(AppError::Other)?;

    // Initialize logging if requested
    if let Some(log_file) = &cli.log_file {
        init_logging(log_file)?;
        log::info!("=== EPUB Reader starting ===");
        log::info!("Log file: {}", log_file);
        if let Some(file) = &cli.file {
            log::info!("Loading file: {}", file);
        }
        if let Some(max_width) = cli.max_width {
            log::info!("CLI max width override: {}", max_width);
        }
    }

    // Check terminal size
    let (width, height) = crossterm::terminal::size()?;
    if width < MIN_TERMINAL_WIDTH || height < MIN_TERMINAL_HEIGHT {
        log::error!(
            "Terminal too small: {}x{} (minimum: {}x{})",
            width,
            height,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT
        );
        return Err(AppError::TerminalTooSmall);
    }
    log::debug!("Terminal size: {}x{}", width, height);

    // Setup terminal
    setup_terminal()?;
    log::debug!("Terminal setup completed");

    // Setup Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        log::info!("Ctrl-C received, shutting down");
        r.store(false, Ordering::SeqCst);
    })
    .map_err(|e| AppError::Other(format!("Failed to set Ctrl-C handler: {}", e)))?;

    // Run the application
    let result = run_app(cli, running).await;

    // Cleanup terminal
    cleanup_terminal()?;
    log::debug!("Terminal cleanup completed");

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
    use std::fs::OpenOptions;
    use std::io::Write;

    // Open/create log file, truncating if it exists
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_file)
        .map_err(|e| AppError::Other(format!("Failed to open log file: {}", e)))?;

    // Initialize env_logger with file output
    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .filter_module("reef", log::LevelFilter::Debug) // Only log from our crate
        .filter_level(log::LevelFilter::Off) // Disable all other crates
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    Ok(())
}

async fn run_app(cli: Cli, running: Arc<AtomicBool>) -> Result<()> {
    // Create backend and terminal
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create task channel
    let (task_tx, mut task_rx) = mpsc::unbounded_channel();

    // Initialize app state
    let mut app = initialize_app_state(&cli)?;

    // Create task runner
    let task_runner = AsyncTaskRunner::new(task_tx);

    // Create resize debouncer
    let resize_tx = task_runner.spawn_resize_debouncer(RESIZE_DEBOUNCE_MS);

    // Load initial book or show picker
    load_initial_book(&mut app, &cli, &task_runner)?;

    // Run main event loop
    run_event_loop(&mut terminal, &mut app, &mut task_rx, running, &resize_tx).await?;

    // Save state before quitting
    save_app_state(&mut app);

    log::info!("EPUB Reader shutting down");
    Ok(())
}

fn initialize_app_state(cli: &Cli) -> Result<AppState> {
    log::debug!("Initializing application state");

    // Initialize persistence manager
    let persistence = PersistenceManager::new().map_err(|e| {
        log::error!("Failed to initialize persistence: {}", e);
        AppError::Other(format!("Failed to initialize persistence: {}", e))
    })?;

    // Load config
    let config = persistence.load_config().unwrap_or_else(|e| {
        log::warn!("Failed to load config: {}. Using defaults.", e);
        Config::default()
    });
    log::debug!(
        "Config loaded: max_width={:?}, toc_panel_width={}, bookmarks_panel_width={}",
        config.max_width,
        config.toc_panel_width,
        config.bookmarks_panel_width
    );

    let mut app = AppState::new(config, persistence);

    // Set CLI max_width override (not persisted)
    if let Some(max_width) = cli.max_width {
        app.cli_max_width_override = Some(max_width);
        log::debug!("CLI max width override applied: {}", max_width);
    }

    // Get terminal size and update viewport
    let (width, height) = crossterm::terminal::size()?;
    app.update_viewport_size(width, height);
    log::debug!(
        "Viewport initialized: {}x{}",
        app.viewport.width,
        app.viewport.height
    );

    Ok(app)
}

fn load_initial_book(app: &mut AppState, cli: &Cli, task_runner: &AsyncTaskRunner) -> Result<()> {
    if let Some(file_path) = &cli.file {
        log::info!("Starting initial book load: {}", file_path);

        // Start async loading
        let effective_width = app.effective_max_width();
        let viewport_width = app.viewport.width;

        let (_handle, _join_handle) =
            task_runner.spawn_load_epub(file_path.clone(), effective_width, viewport_width);

        app.loading_state = LoadingState::LoadingBook {
            file_path: file_path.clone(),
        };
    } else {
        // No file provided - check if we have recent books
        log::debug!("No file provided, checking recent books");
        if app.recent_books.is_empty() {
            log::error!("No recent books available");
            return Err(AppError::Other(
                "No recent books. Usage: reef <file.epub>".to_string(),
            ));
        }

        log::debug!(
            "Showing book picker with {} recent books",
            app.recent_books.len()
        );
        // Show book picker
        app.ui_mode = UiMode::BookPicker;
        app.book_picker_selected_idx = Some(0);
    }

    Ok(())
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    task_rx: &mut mpsc::UnboundedReceiver<TaskMessage>,
    running: Arc<AtomicBool>,
    resize_tx: &mpsc::UnboundedSender<(u16, u16)>,
) -> Result<()> {
    let frame_duration = Duration::from_millis(FRAME_DURATION_MS);

    while running.load(Ordering::SeqCst) && !app.should_quit {
        let frame_start = Instant::now();

        // Process all pending task messages (non-blocking)
        while let Ok(msg) = task_rx.try_recv() {
            handle_task_message(app, msg);
        }

        // Render UI
        terminal.draw(|f| {
            ui::layout::render(f, app);
        })?;

        // Poll for input events (non-blocking)
        if event::poll(Duration::from_millis(0))? {
            let ev = event::read()?;
            handle_event(app, ev, resize_tx)?;
        }

        // Sleep to maintain frame rate
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        }
    }

    Ok(())
}

fn handle_task_message(app: &mut AppState, msg: TaskMessage) {
    match msg {
        TaskMessage::BookLoadingStarted { file_path } => {
            log::info!("Book loading started: {}", file_path);
            app.loading_state = LoadingState::LoadingBook { file_path };
        }

        TaskMessage::BookLoaded { book, file_path } => {
            log::info!(
                "Book loaded: {} ({} chapters)",
                book.metadata.title,
                book.chapters.len()
            );

            let total_chapters = book.chapters.len();

            // Load book through normal path (handles persistence)
            if let Err(e) = app.load_book_with_path(file_path.clone()) {
                log::error!("Failed to load book path: {}", e);
                app.ui_mode = UiMode::ErrorPopup(format!("Failed to load book: {}", e));
                app.loading_state = LoadingState::Idle;
                return;
            }

            // Set the loaded book (with first chapter already rendered)
            app.book = Some(book);

            // Update loading state
            app.loading_state = LoadingState::RenderingChapters {
                rendered: 1,
                total: total_chapters,
            };
        }

        TaskMessage::BookLoadError { error } => {
            log::error!("Book load error: {}", error);
            app.ui_mode = UiMode::ErrorPopup(format!("Failed to load book: {}", error));
            app.loading_state = LoadingState::Idle;
        }

        TaskMessage::ChapterRendered {
            chapter_idx,
            rendered_chapter,
        } => {
            // Update the rendered chapter in the book
            if let Some(book) = &mut app.book {
                if let Some(chapter) = book.chapters.get_mut(chapter_idx) {
                    *chapter = rendered_chapter;
                }

                // Update loading state
                if let LoadingState::RenderingChapters { rendered, total } = &mut app.loading_state
                {
                    *rendered = chapter_idx + 1;
                    log::debug!("Rendered chapter {}/{}", rendered, total);
                }
            }
        }

        TaskMessage::AllChaptersRendered => {
            log::info!("All chapters rendered");
            app.loading_state = LoadingState::Idle;
        }

        TaskMessage::ResizeComplete { width, height } => {
            log::info!("Resize complete: {}x{}", width, height);
            handle_resize_complete(app, width, height);
        }
    }
}

fn handle_event(
    app: &mut AppState,
    ev: Event,
    resize_tx: &mpsc::UnboundedSender<(u16, u16)>,
) -> Result<()> {
    match ev {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            ui::handle_key_event(app, key)?;
        }
        Event::Resize(width, height) => {
            // Update viewport immediately for UI
            app.update_viewport_size(width, height);

            // Send to debouncer for re-rendering
            let _ = resize_tx.send((width, height));
        }
        _ => {}
    }
    Ok(())
}

fn handle_resize_complete(app: &mut AppState, width: u16, _height: u16) {
    log::info!("Handling resize complete: {}x{}", width, _height);

    let effective_width = app.effective_max_width();
    let viewport_width = width;
    let has_search_results = !app.search_results.is_empty();
    let search_query = app.search_query.clone();

    if let Some(book) = &mut app.book {
        log::debug!(
            "Re-rendering {} chapters for new width",
            book.chapters.len()
        );

        // Re-render all chapters
        for chapter in &mut book.chapters {
            epub::render_chapter(chapter, effective_width, viewport_width);
        }

        // Re-apply search highlights if there are active results
        if has_search_results {
            log::debug!("Re-applying search highlights after resize");

            // Re-run search to recalculate match positions in new line structure
            match search::SearchEngine::search(book, &search_query) {
                Ok(new_results) => {
                    app.search_results = new_results;
                    search::SearchEngine::apply_highlights(book, &app.search_results);
                }
                Err(e) => {
                    log::warn!("Failed to re-apply search after resize: {}", e);
                }
            }
        }
    }

    log::debug!("Resize handling complete");
}

fn save_app_state(app: &mut AppState) {
    if let Err(e) = app.save_state() {
        log::error!("Failed to save state: {}", e);
    }
}
