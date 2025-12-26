

#[derive(Debug, Clone)]
pub struct Book {
    pub metadata: BookMetadata,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub sections: Vec<Section>,
    pub content_lines: Vec<RenderedLine>,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub start_line: usize,
}

#[derive(Debug, Clone)]
pub struct RenderedLine {
    pub text: String,
    pub style: LineStyle,
    pub search_matches: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LineStyle {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    CodeBlock { language: Option<String> },
    InlineCode,
    Quote,
    Link,
}

#[derive(Debug, Clone)]
pub struct BookMetadata {
    pub title: String,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub publication_date: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Content,
    TOC,
    Bookmarks,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub max_width: Option<usize>,
    pub toc_panel_width: u16,
    pub bookmarks_panel_width: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: None,
            toc_panel_width: 30,
            bookmarks_panel_width: 35,
        }
    }
}
