use crate::types::{Chapter, LineStyle, RenderedLine};
use crate::epub::code_highlight::CodeHighlighter;
use scraper::{Html, Selector, ElementRef};
use textwrap::wrap;
use lazy_static::lazy_static;

lazy_static! {
    static ref CODE_HIGHLIGHTER: CodeHighlighter = CodeHighlighter::new();
}

pub fn render_chapter(chapter: &mut Chapter, max_width: Option<usize>, terminal_width: u16) {
    // Determine effective width
    let width = if let Some(max) = max_width {
        max.min(terminal_width as usize)
    } else {
        terminal_width as usize
    };

    let width = width.saturating_sub(4); // Reserve space for margins/UI

    // Parse HTML content from the chapter's file
    let html = Html::parse_fragment(&chapter.file_path);
    
    // Extract and render content, also track heading positions
    let (rendered_lines, headings) = extract_and_render(&html, width);
    
    // If chapter has no sections from TOC, extract them from HTML headings
    if chapter.sections.is_empty() {
        // Build sections from h2/h3 headings found in content
        for heading in &headings {
            // Skip h1 (chapter title) and only include h2/h3 as sections
            if heading.level >= 2 && heading.level <= 3 {
                chapter.sections.push(crate::types::Section {
                    title: heading.text.clone(),
                    start_line: heading.line_number,
                });
            }
        }
    } else {
        // Match existing TOC sections to rendered headings
        for section in &mut chapter.sections {
            // Try to find matching heading by title (with basic normalization)
            let normalized_section_title = normalize_text(&section.title);
            
            for heading in &headings {
                let normalized_heading_text = normalize_text(&heading.text);
                
                if normalized_heading_text == normalized_section_title {
                    section.start_line = heading.line_number;
                    break;
                }
            }
        }
    }
    
    chapter.content_lines = rendered_lines;
}

// Simple text normalization - trim whitespace and decode common HTML entities
fn normalize_text(text: &str) -> String {
    text.trim()
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[derive(Debug)]
struct HeadingInfo {
    text: String,
    level: u8,
    line_number: usize,
}

fn extract_and_render(html: &Html, width: usize) -> (Vec<RenderedLine>, Vec<HeadingInfo>) {
    let mut rendered_lines = Vec::new();
    let mut headings = Vec::new();
    
    // Find the body or root element
    let body_selector = Selector::parse("body").ok();
    let root = if let Some(ref sel) = body_selector {
        html.select(sel).next().map(|e| e)
    } else {
        None
    };
    
    let start_element = root.unwrap_or_else(|| html.root_element());
    
    // Process all child nodes
    process_element(start_element, &mut rendered_lines, &mut headings, width, false);
    
    (rendered_lines, headings)
}

fn process_element(element: ElementRef, lines: &mut Vec<RenderedLine>, headings: &mut Vec<HeadingInfo>, width: usize, in_paragraph: bool) {
    let tag_name = element.value().name();
    
    match tag_name {
        // Headings
        "h1" => {
            let text = get_text_content(element);
            let start_line = lines.len();
            headings.push(HeadingInfo {
                text: text.clone(),
                level: 1,
                line_number: start_line,
            });
            add_text_lines(lines, &text, width, LineStyle::Heading1);
            add_blank_line(lines);
        }
        "h2" => {
            let text = get_text_content(element);
            let start_line = lines.len();
            headings.push(HeadingInfo {
                text: text.clone(),
                level: 2,
                line_number: start_line,
            });
            add_text_lines(lines, &text, width, LineStyle::Heading2);
            add_blank_line(lines);
        }
        "h3" | "h4" | "h5" | "h6" => {
            let text = get_text_content(element);
            let start_line = lines.len();
            let level = match tag_name {
                "h3" => 3,
                "h4" => 4,
                "h5" => 5,
                "h6" => 6,
                _ => 3,
            };
            headings.push(HeadingInfo {
                text: text.clone(),
                level,
                line_number: start_line,
            });
            add_text_lines(lines, &text, width, LineStyle::Heading3);
            add_blank_line(lines);
        }
        
        // Code blocks
        "pre" => {
            // Check if it contains a <code> element
            let code_selector = Selector::parse("code").unwrap();
            if let Some(code_elem) = element.select(&code_selector).next() {
                let code_text = get_text_content(code_elem);
                let language = detect_language(code_elem);
                
                // Highlight code
                let highlighted = CODE_HIGHLIGHTER.highlight_code(&code_text, language.as_deref());
                
                // Add highlighted lines
                for (text, _color) in highlighted {
                    for line in text.lines() {
                        lines.push(RenderedLine {
                            text: line.to_string(),
                            style: LineStyle::CodeBlock { language: language.clone() },
                            search_matches: Vec::new(),
                        });
                    }
                }
                add_blank_line(lines);
            } else {
                // Treat as preformatted text
                let text = get_text_content(element);
                for line in text.lines() {
                    lines.push(RenderedLine {
                        text: line.to_string(),
                        style: LineStyle::CodeBlock { language: None },
                        search_matches: Vec::new(),
                    });
                }
                add_blank_line(lines);
            }
        }
        
        // Images
        "img" => {
            let alt_text = element.value().attr("alt").unwrap_or("");
            let placeholder = if alt_text.is_empty() {
                "[Image]".to_string()
            } else {
                let truncated = if alt_text.len() > 50 {
                    format!("{}...", &alt_text[..50])
                } else {
                    alt_text.to_string()
                };
                format!("[Image: {}]", truncated)
            };
            
            lines.push(RenderedLine {
                text: placeholder,
                style: LineStyle::Normal,
                search_matches: Vec::new(),
            });
            add_blank_line(lines);
        }
        
        // Paragraphs
        "p" => {
            let text = extract_paragraph_with_inline_code(element);
            add_text_lines(lines, &text, width, LineStyle::Normal);
            add_blank_line(lines);
        }
        
        // Blockquotes
        "blockquote" => {
            let text = get_text_content(element);
            add_text_lines(lines, &text, width, LineStyle::Quote);
            add_blank_line(lines);
        }
        
        // Links (extract text only)
        "a" => {
            if !in_paragraph {
                let text = get_text_content(element);
                add_text_lines(lines, &text, width, LineStyle::Link);
            }
        }
        
        // Divs and sections - recurse into children
        "div" | "section" | "article" | "body" | "html" => {
            for child in element.children() {
                if let Some(child_element) = ElementRef::wrap(child) {
                    process_element(child_element, lines, headings, width, false);
                }
            }
        }
        
        // Inline elements that shouldn't create new blocks
        "span" | "em" | "strong" | "i" | "b" | "code" => {
            // These are handled within paragraph context
            if !in_paragraph {
                let text = get_text_content(element);
                if !text.trim().is_empty() {
                    add_text_lines(lines, &text, width, LineStyle::Normal);
                }
            }
        }
        
        // Other block elements
        _ => {
            // Recurse into children for unknown elements
            for child in element.children() {
                if let Some(child_element) = ElementRef::wrap(child) {
                    process_element(child_element, lines, headings, width, false);
                }
            }
        }
    }
}

fn get_text_content(element: ElementRef) -> String {
    element.text().collect::<Vec<_>>().join("")
}

fn extract_paragraph_with_inline_code(element: ElementRef) -> String {
    let mut result = String::new();
    
    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            result.push_str(text);
        } else if let Some(child_elem) = ElementRef::wrap(child) {
            let tag = child_elem.value().name();
            if tag == "code" {
                // Mark inline code with backticks for now
                // In future phases, we can apply special styling
                result.push('`');
                result.push_str(&get_text_content(child_elem));
                result.push('`');
            } else {
                result.push_str(&get_text_content(child_elem));
            }
        }
    }
    
    result
}

fn detect_language(code_element: ElementRef) -> Option<String> {
    let classes = code_element.value().attr("class")?;
    
    const KNOWN_LANGUAGES: &[&str] = &[
        "rust", "python", "javascript", "typescript", "java", "c", "cpp", "go",
        "ruby", "php", "swift", "kotlin", "scala", "haskell", "elixir", "erlang",
        "clojure", "bash", "sh", "shell", "sql", "html", "css", "json", "xml",
        "yaml", "markdown", "md", "toml",
    ];
    
    for class in classes.split_whitespace() {
        // Check for "language-X" pattern
        if let Some(lang) = class.strip_prefix("language-") {
            return Some(lang.to_string());
        }
        
        // Check for "highlight-X" pattern
        if let Some(lang) = class.strip_prefix("highlight-") {
            return Some(lang.to_string());
        }
        
        // Check for "sourceCode X" pattern
        if let Some(lang) = class.strip_prefix("sourceCode") {
            return Some(lang.trim().to_string());
        }
        
        // Check if it's a known language name directly
        if KNOWN_LANGUAGES.contains(&class) {
            return Some(class.to_string());
        }
    }
    
    None
}

fn add_text_lines(lines: &mut Vec<RenderedLine>, text: &str, width: usize, style: LineStyle) {
    if text.trim().is_empty() {
        return;
    }
    
    let wrapped = wrap(text, width);
    for wrapped_line in wrapped {
        lines.push(RenderedLine {
            text: wrapped_line.to_string(),
            style: style.clone(),
            search_matches: Vec::new(),
        });
    }
}

fn add_blank_line(lines: &mut Vec<RenderedLine>) {
    lines.push(RenderedLine {
        text: String::new(),
        style: LineStyle::Normal,
        search_matches: Vec::new(),
    });
}
