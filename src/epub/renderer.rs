use crate::constants::UI_MARGIN_WIDTH;
use crate::epub::code_highlight::CodeHighlighter;
use crate::types::{Chapter, InlineStyle, LineStyle, RenderedLine};
use lazy_static::lazy_static;
use scraper::{ElementRef, Html, Selector};
use textwrap::wrap;

lazy_static! {
    static ref CODE_HIGHLIGHTER: CodeHighlighter = CodeHighlighter::new();
}

/// Render a chapter's HTML content into styled text lines
///
/// Converts HTML to wrapped text with appropriate styling for headings,
/// code blocks, quotes, etc. Updates the chapter's content_lines and
/// section start_line positions.
///
/// # Arguments
/// * `chapter` - Mutable chapter to render (updates content_lines and section positions)
/// * `max_width` - Optional maximum line width (None = use terminal width)
/// * `terminal_width` - Current terminal width in columns
pub fn render_chapter(chapter: &mut Chapter, max_width: Option<usize>, terminal_width: u16) {
    log::debug!(
        "Rendering chapter '{}': max_width={:?}, terminal_width={}",
        chapter.title,
        max_width,
        terminal_width
    );

    // Determine effective width
    let width = if let Some(max) = max_width {
        max.min(terminal_width as usize)
    } else {
        terminal_width as usize
    };

    let width = width.saturating_sub(UI_MARGIN_WIDTH); // Reserve space for margins/UI
    log::debug!("  Effective rendering width: {} columns", width);

    // Parse HTML content from the chapter's file
    let html = Html::parse_fragment(&chapter.file_path);
    let html_len = chapter.file_path.len();

    // Extract and render content, also track heading positions
    let (rendered_lines, headings) = extract_and_render(&html, width);
    log::debug!(
        "  Rendered {} lines, found {} headings from {} bytes of HTML",
        rendered_lines.len(),
        headings.len(),
        html_len
    );

    // If chapter has no sections from TOC, extract them from HTML headings
    if chapter.sections.is_empty() {
        log::debug!("  No TOC sections, extracting from HTML headings");
        // Build sections from h2/h3 headings found in content
        for heading in &headings {
            // Skip h1 (chapter title) and only include h2/h3 as sections
            if heading.level >= 2 && heading.level <= 3 {
                log::debug!(
                    "    Adding section from heading: '{}' at line {}",
                    heading.text,
                    heading.line_number
                );
                chapter.sections.push(crate::types::Section {
                    title: heading.text.clone(),
                    start_line: heading.line_number,
                    fragment_id: heading.id.clone(),
                });
            }
        }
        log::debug!(
            "  Extracted {} sections from headings",
            chapter.sections.len()
        );
    } else {
        // Match existing TOC sections to rendered headings
        log::debug!(
            "Matching {} TOC sections to {} headings",
            chapter.sections.len(),
            headings.len()
        );

        for section in &mut chapter.sections {
            let mut matched = false;

            // First, try to match by fragment ID (most reliable)
            if let Some(ref section_fragment) = section.fragment_id {
                log::debug!(
                    "Trying to match section '{}' with fragment_id '{}'",
                    section.title,
                    section_fragment
                );

                for heading in &headings {
                    if let Some(ref heading_id) = heading.id
                        && heading_id == section_fragment
                    {
                        log::debug!(
                            "  ✓ Matched by fragment ID to heading '{}' at line {}",
                            heading.text,
                            heading.line_number
                        );
                        section.start_line = heading.line_number;
                        matched = true;
                        break;
                    }
                }

                if !matched {
                    log::debug!("  ✗ No fragment ID match found");
                }
            }

            // If no fragment ID match, fall back to title matching
            if !matched {
                let normalized_section_title = normalize_text(&section.title);
                log::debug!(
                    "Trying to match section '{}' by title (normalized: '{}')",
                    section.title,
                    normalized_section_title
                );

                for heading in &headings {
                    let normalized_heading_text = normalize_text(&heading.text);

                    if normalized_heading_text == normalized_section_title {
                        log::debug!(
                            "  ✓ Matched by title to heading '{}' at line {}",
                            heading.text,
                            heading.line_number
                        );
                        section.start_line = heading.line_number;
                        matched = true;
                        break;
                    }
                }

                if !matched {
                    log::debug!("  ✗ No title match found, section will remain at start_line 0");
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
    id: Option<String>,
}

fn extract_and_render(html: &Html, width: usize) -> (Vec<RenderedLine>, Vec<HeadingInfo>) {
    let mut rendered_lines = Vec::new();
    let mut headings = Vec::new();

    // Find the body or root element
    let body_selector = Selector::parse("body").ok();
    let root = if let Some(ref sel) = body_selector {
        html.select(sel).next()
    } else {
        None
    };

    let start_element = root.unwrap_or_else(|| html.root_element());

    // Process all child nodes
    process_element(
        start_element,
        &mut rendered_lines,
        &mut headings,
        width,
        false,
    );

    (rendered_lines, headings)
}

fn process_element(
    element: ElementRef,
    lines: &mut Vec<RenderedLine>,
    headings: &mut Vec<HeadingInfo>,
    width: usize,
    in_paragraph: bool,
) {
    let tag_name = element.value().name();

    match tag_name {
        // Headings
        "h1" => process_heading(element, lines, headings, width, 1, LineStyle::Heading1),
        "h2" => process_heading(element, lines, headings, width, 2, LineStyle::Heading2),
        "h3" | "h4" | "h5" | "h6" => {
            let level = match tag_name {
                "h3" => 3,
                "h4" => 4,
                "h5" => 5,
                "h6" => 6,
                _ => 3,
            };
            process_heading(element, lines, headings, width, level, LineStyle::Heading3);
        }

        // Code blocks
        "pre" => process_code_block(element, lines),

        // Images
        "img" => process_image(element, lines),

        // Paragraphs
        "p" => process_paragraph(element, lines, width),

        // Blockquotes
        "blockquote" => process_blockquote(element, lines, width),

        // Lists (EPUB3)
        "ul" => process_unordered_list(element, lines, width),
        "ol" => process_ordered_list(element, lines, width),
        "dl" => process_definition_list(element, lines, width),

        // Tables (basic EPUB3 support)
        "table" => process_table(element, lines, width),

        // Horizontal rules
        "hr" => process_horizontal_rule(lines, width),

        // EPUB3 semantic elements
        "aside" | "figure" | "figcaption" => {
            process_semantic_container(element, lines, headings, width)
        }
        "nav" => process_navigation(element, lines, headings, width),

        // Links (extract text only)
        "a" => {
            if !in_paragraph {
                process_link(element, lines, width);
            }
        }

        // Divs and sections - recurse into children
        "div" | "section" | "article" | "body" | "html" | "main" => {
            process_container(element, lines, headings, width);
        }

        // Inline elements that shouldn't create new blocks
        "span" | "em" | "strong" | "i" | "b" | "code" => {
            if !in_paragraph {
                process_inline_as_block(element, lines, width);
            }
        }

        // Other block elements
        _ => {
            process_container(element, lines, headings, width);
        }
    }
}

fn process_heading(
    element: ElementRef,
    lines: &mut Vec<RenderedLine>,
    headings: &mut Vec<HeadingInfo>,
    width: usize,
    level: u8,
    style: LineStyle,
) {
    let (text, inline_styles) = extract_text_with_inline_styles(element);
    let start_line = lines.len();
    let id = element.value().attr("id").map(|s| s.to_string());
    headings.push(HeadingInfo {
        text: text.clone(),
        level,
        line_number: start_line,
        id,
    });
    add_text_lines(lines, &text, width, style, inline_styles);
    add_blank_line(lines);
}

fn process_code_block(element: ElementRef, lines: &mut Vec<RenderedLine>) {
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
                    style: LineStyle::CodeBlock {
                        language: language.clone(),
                    },
                    search_matches: Vec::new(),
                    inline_styles: Vec::new(),
                });
            }
        }
    } else {
        // Treat as preformatted text
        let text = get_text_content(element);
        for line in text.lines() {
            lines.push(RenderedLine {
                text: line.to_string(),
                style: LineStyle::CodeBlock { language: None },
                search_matches: Vec::new(),
                inline_styles: Vec::new(),
            });
        }
    }
    add_blank_line(lines);
}

fn process_image(element: ElementRef, lines: &mut Vec<RenderedLine>) {
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
        inline_styles: Vec::new(),
    });
    add_blank_line(lines);
}

fn process_paragraph(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let (text, inline_styles) = extract_text_with_inline_styles(element);
    add_text_lines(lines, &text, width, LineStyle::Normal, inline_styles);
    add_blank_line(lines);
}

fn process_blockquote(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let (text, inline_styles) = extract_text_with_inline_styles(element);
    add_text_lines(lines, &text, width, LineStyle::Quote, inline_styles);
    add_blank_line(lines);
}

fn process_link(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let (text, inline_styles) = extract_text_with_inline_styles(element);
    add_text_lines(lines, &text, width, LineStyle::Link, inline_styles);
}

fn process_container(
    element: ElementRef,
    lines: &mut Vec<RenderedLine>,
    headings: &mut Vec<HeadingInfo>,
    width: usize,
) {
    for child in element.children() {
        if let Some(child_element) = ElementRef::wrap(child) {
            process_element(child_element, lines, headings, width, false);
        }
    }
}

fn process_inline_as_block(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let (text, inline_styles) = extract_text_with_inline_styles(element);
    if !text.trim().is_empty() {
        add_text_lines(lines, &text, width, LineStyle::Normal, inline_styles);
    }
}

fn get_text_content(element: ElementRef) -> String {
    element.text().collect::<Vec<_>>().join("")
}

/// Extract text content with inline styling information
/// Returns: (text, Vec<(start, end, InlineStyle)>)
fn extract_text_with_inline_styles(
    element: ElementRef,
) -> (String, Vec<(usize, usize, InlineStyle)>) {
    let mut result = String::new();
    let mut inline_styles = Vec::new();

    fn process_children(
        element: ElementRef,
        result: &mut String,
        inline_styles: &mut Vec<(usize, usize, InlineStyle)>,
        current_styles: &[InlineStyle],
    ) {
        for child in element.children() {
            if let Some(text) = child.value().as_text() {
                let start = result.len();
                result.push_str(text);
                let end = result.len();

                // Add all current styles for this text range
                for style in current_styles {
                    if end > start {
                        inline_styles.push((start, end, style.clone()));
                    }
                }
            } else if let Some(child_elem) = ElementRef::wrap(child) {
                let tag = child_elem.value().name();

                // Determine which styles to add for this tag
                let mut new_styles = current_styles.to_vec();
                match tag {
                    "strong" | "b" => new_styles.push(InlineStyle::Bold),
                    "em" | "i" => new_styles.push(InlineStyle::Italic),
                    "code" => new_styles.push(InlineStyle::Code),
                    "u" => new_styles.push(InlineStyle::Underline),
                    "s" | "del" | "strike" => new_styles.push(InlineStyle::Strikethrough),
                    "mark" => new_styles.push(InlineStyle::Highlight),
                    _ => {}
                }

                // Process children with accumulated styles
                process_children(child_elem, result, inline_styles, &new_styles);
            }
        }
    }

    process_children(element, &mut result, &mut inline_styles, &[]);

    (result, inline_styles)
}

fn detect_language(code_element: ElementRef) -> Option<String> {
    let classes = code_element.value().attr("class")?;

    const KNOWN_LANGUAGES: &[&str] = &[
        "rust",
        "python",
        "javascript",
        "typescript",
        "java",
        "c",
        "cpp",
        "go",
        "ruby",
        "php",
        "swift",
        "kotlin",
        "scala",
        "haskell",
        "elixir",
        "erlang",
        "clojure",
        "bash",
        "sh",
        "shell",
        "sql",
        "html",
        "css",
        "json",
        "xml",
        "yaml",
        "markdown",
        "md",
        "toml",
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

fn add_text_lines(
    lines: &mut Vec<RenderedLine>,
    text: &str,
    width: usize,
    style: LineStyle,
    inline_styles: Vec<(usize, usize, InlineStyle)>,
) {
    if text.trim().is_empty() {
        return;
    }

    let wrapped = wrap(text, width);
    let mut char_offset = 0;

    for wrapped_line in wrapped {
        let line_text = wrapped_line.to_string();
        let line_len = line_text.len();
        let line_end = char_offset + line_len;

        // Find inline styles that overlap with this wrapped line
        let mut line_inline_styles = Vec::new();
        for (start, end, style_type) in &inline_styles {
            // Check if this style range overlaps with current line
            if *end > char_offset && *start < line_end {
                // Adjust positions relative to this line
                let new_start = (*start).max(char_offset) - char_offset;
                let new_end = (*end).min(line_end) - char_offset;
                if new_end > new_start {
                    line_inline_styles.push((new_start, new_end, style_type.clone()));
                }
            }
        }

        lines.push(RenderedLine {
            text: line_text,
            style: style.clone(),
            search_matches: Vec::new(),
            inline_styles: line_inline_styles,
        });

        // Account for space or newline that was removed by wrapping
        char_offset = line_end;
        // textwrap removes spaces at wrap points, so we need to account for that
        if char_offset < text.len() && text.chars().nth(char_offset) == Some(' ') {
            char_offset += 1;
        }
    }
}

fn add_blank_line(lines: &mut Vec<RenderedLine>) {
    lines.push(RenderedLine {
        text: String::new(),
        style: LineStyle::Normal,
        search_matches: Vec::new(),
        inline_styles: Vec::new(),
    });
}

// EPUB3 feature handlers

fn process_unordered_list(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let li_selector = Selector::parse("li").unwrap();
    for li in element.select(&li_selector) {
        let (text, inline_styles) = extract_text_with_inline_styles(li);
        let bullet_text = format!("• {}", text);
        // Adjust inline style positions for the bullet prefix (2 chars)
        let adjusted_styles: Vec<_> = inline_styles
            .into_iter()
            .map(|(start, end, style)| (start + 2, end + 2, style))
            .collect();
        add_text_lines(
            lines,
            &bullet_text,
            width.saturating_sub(2),
            LineStyle::Normal,
            adjusted_styles,
        );
    }
    add_blank_line(lines);
}

fn process_ordered_list(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let li_selector = Selector::parse("li").unwrap();
    for (index, li) in element.select(&li_selector).enumerate() {
        let (text, inline_styles) = extract_text_with_inline_styles(li);
        let numbered_text = format!("{}. {}", index + 1, text);
        // Adjust inline style positions for the number prefix
        let prefix_len = format!("{}. ", index + 1).len();
        let adjusted_styles: Vec<_> = inline_styles
            .into_iter()
            .map(|(start, end, style)| (start + prefix_len, end + prefix_len, style))
            .collect();
        add_text_lines(
            lines,
            &numbered_text,
            width.saturating_sub(3),
            LineStyle::Normal,
            adjusted_styles,
        );
    }
    add_blank_line(lines);
}

fn process_definition_list(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    let dt_selector = Selector::parse("dt").unwrap();
    let dd_selector = Selector::parse("dd").unwrap();

    // Process definition terms
    for dt in element.select(&dt_selector) {
        let (text, inline_styles) = extract_text_with_inline_styles(dt);
        add_text_lines(lines, &text, width, LineStyle::Heading3, inline_styles);
    }

    // Process definition descriptions
    for dd in element.select(&dd_selector) {
        let (text, inline_styles) = extract_text_with_inline_styles(dd);
        let indented_text = format!("  {}", text);
        // Adjust inline style positions for the indent (2 chars)
        let adjusted_styles: Vec<_> = inline_styles
            .into_iter()
            .map(|(start, end, style)| (start + 2, end + 2, style))
            .collect();
        add_text_lines(
            lines,
            &indented_text,
            width.saturating_sub(2),
            LineStyle::Normal,
            adjusted_styles,
        );
    }

    add_blank_line(lines);
}

fn process_table(element: ElementRef, lines: &mut Vec<RenderedLine>, width: usize) {
    // Simple table rendering - just extract text row by row
    lines.push(RenderedLine {
        text: "[Table]".to_string(),
        style: LineStyle::Normal,
        search_matches: Vec::new(),
        inline_styles: Vec::new(),
    });

    let tr_selector = Selector::parse("tr").unwrap();
    let td_selector = Selector::parse("td, th").unwrap();

    for tr in element.select(&tr_selector) {
        let mut row_text = String::new();
        let mut row_inline_styles = Vec::new();
        let mut current_pos = 0;

        for (index, td) in tr.select(&td_selector).enumerate() {
            if index > 0 {
                row_text.push_str(" | ");
                current_pos += 3;
            }
            let (text, inline_styles) = extract_text_with_inline_styles(td);
            // Adjust inline style positions for the current position in row
            for (start, end, style) in inline_styles {
                row_inline_styles.push((start + current_pos, end + current_pos, style));
            }
            row_text.push_str(&text);
            current_pos += text.len();
        }
        if !row_text.trim().is_empty() {
            add_text_lines(
                lines,
                &row_text,
                width,
                LineStyle::Normal,
                row_inline_styles,
            );
        }
    }

    add_blank_line(lines);
}

fn process_horizontal_rule(lines: &mut Vec<RenderedLine>, width: usize) {
    let rule = "─".repeat(width.min(80));
    lines.push(RenderedLine {
        text: rule,
        style: LineStyle::Normal,
        search_matches: Vec::new(),
        inline_styles: Vec::new(),
    });
    add_blank_line(lines);
}

fn process_semantic_container(
    element: ElementRef,
    lines: &mut Vec<RenderedLine>,
    _headings: &mut Vec<HeadingInfo>,
    _width: usize,
) {
    // Add a visual separator for semantic containers
    let tag_name = element.value().name();
    if tag_name == "aside" {
        lines.push(RenderedLine {
            text: "┌─ Aside ─".to_string(),
            style: LineStyle::Quote,
            search_matches: Vec::new(),
            inline_styles: Vec::new(),
        });
    } else if tag_name == "figure" {
        lines.push(RenderedLine {
            text: "[Figure]".to_string(),
            style: LineStyle::Normal,
            search_matches: Vec::new(),
            inline_styles: Vec::new(),
        });
    }

    // Process children
    for child in element.children() {
        if let Some(child_element) = ElementRef::wrap(child) {
            process_element(child_element, lines, _headings, _width, false);
        }
    }

    if tag_name == "aside" {
        lines.push(RenderedLine {
            text: "└─────────".to_string(),
            style: LineStyle::Quote,
            search_matches: Vec::new(),
            inline_styles: Vec::new(),
        });
        add_blank_line(lines);
    }
}

fn process_navigation(
    element: ElementRef,
    lines: &mut Vec<RenderedLine>,
    _headings: &mut Vec<HeadingInfo>,
    _width: usize,
) {
    // Navigation elements are typically TOC - we can skip or render minimally
    lines.push(RenderedLine {
        text: "─── Navigation ───".to_string(),
        style: LineStyle::Heading3,
        search_matches: Vec::new(),
        inline_styles: Vec::new(),
    });

    // Process links in navigation
    let a_selector = Selector::parse("a").unwrap();
    for link in element.select(&a_selector) {
        let text = get_text_content(link);
        if !text.trim().is_empty() {
            let nav_item = format!("→ {}", text);
            lines.push(RenderedLine {
                text: nav_item,
                style: LineStyle::Link,
                search_matches: Vec::new(),
                inline_styles: Vec::new(),
            });
        }
    }

    add_blank_line(lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chapter(html_content: &str) -> Chapter {
        Chapter {
            title: "Test Chapter".to_string(),
            sections: Vec::new(),
            content_lines: Vec::new(),
            file_path: html_content.to_string(),
        }
    }

    #[test]
    fn test_render_simple_paragraph() {
        let html = "<p>This is a simple paragraph.</p>";
        let mut chapter = create_test_chapter(html);

        render_chapter(&mut chapter, Some(80), 100);

        assert!(!chapter.content_lines.is_empty());
        assert!(
            chapter.content_lines[0]
                .text
                .contains("This is a simple paragraph")
        );
    }

    #[test]
    fn test_render_heading() {
        let html = "<h1>Main Heading</h1><p>Content here.</p>";
        let mut chapter = create_test_chapter(html);

        render_chapter(&mut chapter, Some(80), 100);

        assert!(!chapter.content_lines.is_empty());
        // Find the heading line
        let heading_line = chapter
            .content_lines
            .iter()
            .find(|line| line.style == LineStyle::Heading1);
        assert!(heading_line.is_some());
    }

    #[test]
    fn test_extract_sections_from_headings() {
        let html = r#"
            <h1>Chapter Title</h1>
            <h2 id="section-1">Section 1</h2>
            <p>Content</p>
            <h2 id="section-2">Section 2</h2>
            <p>More content</p>
        "#;
        let mut chapter = create_test_chapter(html);

        render_chapter(&mut chapter, Some(80), 100);

        // Should extract h2 headings as sections
        assert!(chapter.sections.len() >= 2);
        assert!(
            chapter
                .sections
                .iter()
                .any(|s| s.title.contains("Section 1"))
        );
        assert!(
            chapter
                .sections
                .iter()
                .any(|s| s.title.contains("Section 2"))
        );
    }

    #[test]
    fn test_word_wrapping() {
        let long_text = "word ".repeat(50); // 250 characters
        let html = format!("<p>{}</p>", long_text);
        let mut chapter = create_test_chapter(&html);

        render_chapter(&mut chapter, Some(40), 100);

        // Should wrap into multiple lines
        assert!(chapter.content_lines.len() > 1);
        // Each line should be shorter than max width
        for line in &chapter.content_lines {
            assert!(line.text.len() <= 40 + 10); // +10 for some margin
        }
    }

    #[test]
    fn test_max_width_limiting() {
        let html = "<p>Short text</p>";
        let mut chapter = create_test_chapter(html);

        // Set max_width smaller than terminal width
        render_chapter(&mut chapter, Some(50), 200);

        // Should use max_width, not terminal width
        assert!(!chapter.content_lines.is_empty());
    }

    #[test]
    fn test_empty_html() {
        let html = "";
        let mut chapter = create_test_chapter(html);

        render_chapter(&mut chapter, Some(80), 100);

        // Should handle empty content gracefully
        // May have 0 or 1 empty line
        assert!(chapter.content_lines.len() <= 1);
    }
}
