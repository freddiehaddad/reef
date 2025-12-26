use crate::types::{Chapter, LineStyle, RenderedLine};
use scraper::{Html, Selector};
use textwrap::wrap;

pub fn render_chapter(chapter: &mut Chapter, max_width: Option<usize>, terminal_width: u16) {
    // Determine effective width
    let width = if let Some(max) = max_width {
        max.min(terminal_width as usize)
    } else {
        terminal_width as usize
    };

    let width = width.saturating_sub(4); // Reserve space for margins/UI

    // Parse HTML content from the chapter's file
    // For Phase 1, we'll do basic text extraction without syntax highlighting
    let mut rendered_lines = Vec::new();

    // Simple text extraction for now
    let fragment = Html::parse_fragment(&chapter.file_path);
    
    // For Phase 1, just create simple text lines
    // We'll enhance this in Phase 3 with proper HTML parsing
    let text = extract_text_simple(&fragment);
    
    // Wrap text to width
    for line in text.lines() {
        if line.trim().is_empty() {
            rendered_lines.push(RenderedLine {
                text: String::new(),
                style: LineStyle::Normal,
                search_matches: Vec::new(),
            });
        } else {
            let wrapped = wrap(line, width);
            for wrapped_line in wrapped {
                rendered_lines.push(RenderedLine {
                    text: wrapped_line.to_string(),
                    style: LineStyle::Normal,
                    search_matches: Vec::new(),
                });
            }
        }
    }

    chapter.content_lines = rendered_lines;
}

fn extract_text_simple(html: &Html) -> String {
    // Basic text extraction - will be enhanced in Phase 3
    let body_selector = Selector::parse("body").unwrap();
    let p_selector = Selector::parse("p").unwrap();
    
    let mut text = String::new();
    
    // Try to get body content
    if let Some(body) = html.select(&body_selector).next() {
        for p in body.select(&p_selector) {
            text.push_str(&p.text().collect::<String>());
            text.push('\n');
            text.push('\n');
        }
    }
    
    // Fallback: just get all text
    if text.is_empty() {
        text = html.root_element().text().collect::<String>();
    }
    
    text
}
