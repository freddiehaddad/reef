# Reef

A terminal-based EPUB reader for reading ebooks from the command line.

## Overview

Reef is a fast, lightweight EPUB reader built with Rust that runs entirely in your terminal. Read your favorite ebooks without leaving the comfort of your command line interface.

## Features

- **EPUB Support** - Read EPUB books directly in your terminal
- **Syntax Highlighting** - Code blocks are highlighted for better readability
- **Table of Contents** - Navigate chapters with an interactive TOC panel
- **Bookmarks** - Create and manage bookmarks with custom labels
- **Search** - Full-text search across the entire book with result highlighting
- **Reading Progress** - Automatically saves your reading position
- **Recent Books** - Quick access to recently opened books
- **Zen Mode** - Distraction-free reading experience
- **Customizable Layout** - Adjustable text width
- **Responsive** - Automatically adapts to terminal resize
- **Persistent State** - Remembers your settings and progress between sessions

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Open an EPUB file
reef book.epub

# Open with custom text width
reef book.epub --max-width 80

# Enable logging for debugging
reef book.epub --log-file reef.log

# Show help
reef --help
```

## Keyboard Shortcuts

- `q` - Quit
- `j/k` or `↓/↑` - Scroll down/up
- `Space/b` - Page down/up
- `t` - Toggle table of contents
- `Enter` - Navigate to selected chapter
- `/` - Search
- `n/N` - Next/previous search result
- `Ctrl-M` - Add bookmark
- `b` - Toggle bookmarks panel
- `z` - Toggle zen mode
- `w` - Cycle text width presets
- `?` - Help

## Requirements

- Terminal with UTF-8 support
- Minimum terminal size: 80x24

## License

MIT License - see [LICENSE](LICENSE.txt) for details.
