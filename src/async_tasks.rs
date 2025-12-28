//! Async task management for background operations
//!
//! This module handles all background tasks including:
//! - EPUB loading and parsing
//! - Chapter rendering
//! - Resize debouncing

use crate::epub::{parse_epub, render_chapter};
use crate::types::Book;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

/// Messages sent from background tasks to the main thread
#[derive(Debug)]
pub enum TaskMessage {
    /// EPUB loading started
    BookLoadingStarted { file_path: String },

    /// EPUB loaded successfully with first chapter rendered
    BookLoaded { book: Book, file_path: String },

    /// EPUB loading failed
    BookLoadError { error: String },

    /// A chapter has been rendered
    ChapterRendered {
        chapter_idx: usize,
        rendered_chapter: crate::types::Chapter,
    },

    /// All chapters have been rendered
    AllChaptersRendered,

    /// Resize event after debounce timeout
    ResizeComplete { width: u16, height: u16 },
}

/// Handle for cancelling a background task
pub struct TaskHandle {
    _cancel_tx: watch::Sender<bool>,
}

/// Manages spawning and communication with background tasks
pub struct AsyncTaskRunner {
    tx: mpsc::UnboundedSender<TaskMessage>,
}

impl AsyncTaskRunner {
    /// Create a new task runner
    pub fn new(tx: mpsc::UnboundedSender<TaskMessage>) -> Self {
        Self { tx }
    }

    /// Spawn a task to load and parse an EPUB file
    ///
    /// This will:
    /// 1. Parse the EPUB file
    /// 2. Render the first chapter immediately
    /// 3. Send the book with first chapter rendered
    /// 4. Render remaining chapters in background
    pub fn spawn_load_epub(
        &self,
        file_path: String,
        effective_width: Option<usize>,
        viewport_width: u16,
    ) -> (TaskHandle, JoinHandle<()>) {
        let tx = self.tx.clone();
        let (cancel_tx, cancel_rx) = watch::channel(false);

        let handle = tokio::spawn(async move {
            load_epub_task(file_path, effective_width, viewport_width, tx, cancel_rx).await
        });

        (
            TaskHandle {
                _cancel_tx: cancel_tx,
            },
            handle,
        )
    }

    /// Spawn a resize debouncer
    ///
    /// Collects resize events and sends a single resize message after debounce timeout
    pub fn spawn_resize_debouncer(&self, debounce_ms: u64) -> mpsc::UnboundedSender<(u16, u16)> {
        let tx = self.tx.clone();
        let (resize_tx, resize_rx) = mpsc::unbounded_channel::<(u16, u16)>();

        tokio::spawn(async move { resize_debounce_task(resize_rx, tx, debounce_ms).await });

        resize_tx
    }
}

/// Background task for loading and rendering an EPUB
async fn load_epub_task(
    file_path: String,
    effective_width: Option<usize>,
    viewport_width: u16,
    tx: mpsc::UnboundedSender<TaskMessage>,
    cancel_rx: watch::Receiver<bool>,
) {
    // Send loading started message
    let _ = tx.send(TaskMessage::BookLoadingStarted {
        file_path: file_path.clone(),
    });

    // Parse EPUB in blocking task (file I/O is blocking)
    let path = PathBuf::from(file_path.clone());
    let parse_result = tokio::task::spawn_blocking(move || parse_epub(&path)).await;

    // Check cancellation
    if *cancel_rx.borrow() {
        return;
    }

    let mut book = match parse_result {
        Ok(Ok(book)) => book,
        Ok(Err(e)) => {
            let _ = tx.send(TaskMessage::BookLoadError {
                error: e.to_string(),
            });
            return;
        }
        Err(e) => {
            let _ = tx.send(TaskMessage::BookLoadError {
                error: format!("Task join error: {}", e),
            });
            return;
        }
    };

    // Render first chapter immediately
    if let Some(first_chapter) = book.chapters.first_mut() {
        render_chapter(first_chapter, effective_width, viewport_width);
    }

    // Send book with first chapter rendered
    let _ = tx.send(TaskMessage::BookLoaded {
        book: book.clone(),
        file_path,
    });

    // Render remaining chapters in background
    for (idx, chapter) in book.chapters.iter_mut().enumerate().skip(1) {
        // Check cancellation
        if *cancel_rx.borrow() {
            return;
        }

        // Render chapter
        render_chapter(chapter, effective_width, viewport_width);

        // Send progress
        let _ = tx.send(TaskMessage::ChapterRendered {
            chapter_idx: idx,
            rendered_chapter: chapter.clone(),
        });

        // Yield to prevent blocking tokio runtime
        tokio::task::yield_now().await;
    }

    // Check final cancellation
    if *cancel_rx.borrow() {
        return;
    }

    // All chapters rendered
    let _ = tx.send(TaskMessage::AllChaptersRendered);
}

/// Background task for debouncing resize events
async fn resize_debounce_task(
    mut resize_rx: mpsc::UnboundedReceiver<(u16, u16)>,
    tx: mpsc::UnboundedSender<TaskMessage>,
    debounce_ms: u64,
) {
    let mut last_size: Option<(u16, u16)> = None;

    loop {
        match tokio::time::timeout(Duration::from_millis(debounce_ms), resize_rx.recv()).await {
            Ok(Some(size)) => {
                // Got new resize event
                last_size = Some(size);
            }
            Ok(None) => {
                // Channel closed
                break;
            }
            Err(_) => {
                // Timeout - no more resize events for debounce period
                if let Some((width, height)) = last_size.take() {
                    // Send the debounced resize
                    let _ = tx.send(TaskMessage::ResizeComplete { width, height });
                }
            }
        }
    }
}
