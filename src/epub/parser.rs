use crate::error::{AppError, Result};
use crate::types::{Book, BookMetadata, Chapter, Section};
use epub::doc::EpubDoc;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
struct TocEntry {
    title: Option<String>,
    sections: Vec<SectionInfo>,
}

#[derive(Debug, Clone)]
struct SectionInfo {
    title: String,
    fragment_id: Option<String>,
}

/// Parse an EPUB file and extract book structure and content
///
/// # Arguments
/// * `path` - Path to the .epub file
///
/// # Returns
/// * `Ok(Book)` - Successfully parsed book with metadata and chapters
/// * `Err(AppError)` - File not found, invalid EPUB, or extraction error
///
/// # Example
/// ```no_run
/// # use reef::epub::parse_epub;
/// let book = parse_epub("mybook.epub")?;
/// # Ok::<(), reef::error::AppError>(())
/// ```
pub fn parse_epub<P: AsRef<Path>>(path: P) -> Result<Book> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    log::info!("Parsing EPUB file: {}", path_str);

    // Check if file exists
    if !path.as_ref().exists() {
        log::error!("EPUB file not found: {}", path_str);
        return Err(AppError::FileNotFound(path_str));
    }

    // Open EPUB
    log::debug!("Opening EPUB document");
    let mut doc = EpubDoc::new(&path).map_err(|e| {
        log::error!("Failed to open EPUB: {}", e);
        AppError::InvalidEpub(format!("{}", e))
    })?;

    // Parse metadata
    let metadata = parse_metadata(&doc);
    log::debug!(
        "Parsed metadata: title='{}', author={:?}",
        metadata.title,
        metadata.author
    );

    // Parse TOC to get chapter and section titles
    let toc = parse_toc(&doc);
    log::debug!("Parsed TOC: {} entries found", toc.len());

    // Build a mapping from spine ID to file path
    let mut id_to_path = HashMap::new();
    for path in toc.keys() {
        // Extract filename from path (e.g., "EPUB\text/ch003.xhtml" -> "ch003.xhtml")
        if let Some(filename) = path.rsplit(&['/', '\\'][..]).next() {
            // Convert filename to potential spine ID (e.g., "ch003.xhtml" -> "ch003_xhtml")
            let potential_id = filename.replace('.', "_");
            id_to_path.insert(potential_id, path.clone());
        }
    }

    // Parse chapters
    let mut chapters = Vec::new();

    // Get the spine (reading order)
    let spine_len = doc.spine.len();
    log::debug!("Processing {} spine entries (chapters)", spine_len);

    for spine_index in 0..spine_len {
        doc.set_current_chapter(spine_index);

        // Get chapter title from TOC, fallback to generic title
        let spine_id = doc.get_current_id().unwrap_or_else(|| {
            log::warn!(
                "No spine ID for chapter {}, using empty string",
                spine_index + 1
            );
            String::new()
        });

        // Map spine ID to file path
        let file_path = id_to_path
            .get(&spine_id)
            .map(|s| s.as_str())
            .unwrap_or(&spine_id);

        let title = toc
            .get(file_path)
            .and_then(|entry| entry.title.clone())
            .unwrap_or_else(|| format!("Chapter {}", spine_index + 1));

        log::debug!(
            "Processing chapter {}/{}: '{}' (spine_id: {}, path: {})",
            spine_index + 1,
            spine_len,
            title,
            spine_id,
            file_path
        );

        // Get HTML content - get_current_str() returns (content, mime_type)
        let (content_html, _mime_type) = doc.get_current_str().ok_or_else(|| {
            log::error!(
                "Failed to extract content for chapter {} ({})",
                spine_index,
                title
            );
            AppError::ChapterExtractionError(format!("Failed to extract chapter {}", spine_index))
        })?;

        // Extract sections from TOC
        let toc_sections = toc
            .get(file_path)
            .map(|entry| entry.sections.clone())
            .unwrap_or_else(|| {
                log::debug!("  No TOC sections found for chapter");
                Vec::new()
            });

        log::debug!("  Found {} TOC sections for chapter", toc_sections.len());

        // Convert to Section structs (will be matched with headings during rendering)
        let sections = toc_sections
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                log::debug!(
                    "    Section {}: '{}' (fragment_id: {:?})",
                    idx + 1,
                    s.title,
                    s.fragment_id
                );
                Section {
                    title: s.title.clone(),
                    start_line: 0,
                    fragment_id: s.fragment_id.clone(),
                }
            })
            .collect();

        chapters.push(Chapter {
            title,
            sections,
            content_lines: Vec::new(), // Will be rendered after parsing
            file_path: content_html,   // Store HTML content here for now
        });
    }

    log::info!(
        "Successfully parsed EPUB: {} chapters extracted",
        chapters.len()
    );
    Ok(Book { metadata, chapters })
}

fn parse_metadata(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> BookMetadata {
    // MetadataItem has a 'value' field that contains the actual string
    BookMetadata {
        title: doc
            .mdata("title")
            .map(|m| m.value.clone())
            .unwrap_or_else(|| "Unknown Title".to_string()),
        author: doc.mdata("creator").map(|m| m.value.clone()),
        publisher: doc.mdata("publisher").map(|m| m.value.clone()),
        publication_date: doc.mdata("date").map(|m| m.value.clone()),
        language: doc.mdata("language").map(|m| m.value.clone()),
    }
}

fn parse_toc(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> HashMap<String, TocEntry> {
    let mut toc_map = HashMap::new();

    // Get TOC from the epub crate
    let toc = doc.toc.clone();

    for nav_point in toc {
        process_nav_point(&nav_point, &mut toc_map, None);
    }

    toc_map
}

fn process_nav_point(
    nav_point: &epub::doc::NavPoint,
    toc_map: &mut HashMap<String, TocEntry>,
    parent_base_path: Option<String>,
) {
    // Extract the content path (this is the resource ID)
    let content_str = nav_point.content.to_string_lossy().to_string();

    // Split by '#' to get base path and optional fragment ID
    let parts: Vec<&str> = content_str.splitn(2, '#').collect();
    let base_path = parts[0].to_string();
    let fragment_id = parts.get(1).map(|s| s.to_string());

    // Determine if this is a chapter-level entry or a section
    let is_chapter = parent_base_path.is_none();
    let same_file_as_parent = parent_base_path.as_ref() == Some(&base_path);

    if is_chapter {
        // Top-level entry - create or update chapter entry
        let entry = toc_map
            .entry(base_path.clone())
            .or_insert_with(|| TocEntry {
                title: Some(nav_point.label.clone()),
                sections: Vec::new(),
            });

        // If there's already a title and we have a fragment, this might be first section
        if entry.title.is_some() && parts.len() > 1 {
            // Keep existing title, this entry becomes a section
            entry.sections.push(SectionInfo {
                title: nav_point.label.clone(),
                fragment_id: fragment_id.clone(),
            });
        }
    } else if same_file_as_parent {
        // This is a section within the parent chapter
        if let Some(entry) = toc_map.get_mut(&base_path) {
            entry.sections.push(SectionInfo {
                title: nav_point.label.clone(),
                fragment_id: fragment_id.clone(),
            });
        }
    } else {
        // Different file - treat as new chapter
        let entry = toc_map
            .entry(base_path.clone())
            .or_insert_with(|| TocEntry {
                title: Some(nav_point.label.clone()),
                sections: Vec::new(),
            });

        if entry.title.is_none() {
            entry.title = Some(nav_point.label.clone());
        }
    }

    // Recursively process all children - no depth limit!
    for child in &nav_point.children {
        process_nav_point(child, toc_map, Some(base_path.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nonexistent_file() {
        let result = parse_epub("/nonexistent/path/to/book.epub");
        assert!(result.is_err());
        match result {
            Err(AppError::FileNotFound(_)) => {}
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_extract_fragment_id() {
        // Test extracting fragment from URL
        let url1 = "text/chapter1.xhtml#section-2";
        assert_eq!(url1.split('#').nth(1), Some("section-2"));

        let url2 = "chapter1.xhtml";
        assert_eq!(url2.split('#').nth(1), None);
    }

    #[test]
    fn test_spine_id_conversion() {
        // Test filename to spine ID conversion logic
        let filename = "ch003.xhtml";
        let potential_id = filename.replace('.', "_");
        assert_eq!(potential_id, "ch003_xhtml");
    }
}
