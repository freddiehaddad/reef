use crate::error::{AppError, Result};
use crate::types::{Book, BookMetadata, Chapter, Section};
use epub::doc::EpubDoc;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct TocEntry {
    title: Option<String>,
    sections: Vec<Section>,
}

pub fn parse_epub<P: AsRef<Path>>(path: P) -> Result<Book> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    
    // Check if file exists
    if !path.as_ref().exists() {
        return Err(AppError::FileNotFound(path_str));
    }

    // Open EPUB
    let mut doc = EpubDoc::new(&path)
        .map_err(|e| AppError::InvalidEpub(format!("{}", e)))?;

    // Parse metadata
    let metadata = parse_metadata(&doc);

    // Parse TOC to get chapter and section titles
    let toc = parse_toc(&doc);

    // Parse chapters
    let mut chapters = Vec::new();
    
    // Get the spine (reading order)
    let spine_len = doc.spine.len();
    
    for spine_index in 0..spine_len {
        #[allow(deprecated)]
        doc.set_current_page(spine_index);
        
        // Get chapter title from TOC, fallback to generic title
        let spine_id = doc.get_current_id().unwrap_or_default();
        let title = toc.get(&spine_id)
            .and_then(|entry| entry.title.clone())
            .unwrap_or_else(|| format!("Chapter {}", spine_index + 1));

        // Get HTML content - get_current_str() returns (content, mime_type)
        let (content_html, _mime_type) = doc.get_current_str()
            .ok_or_else(|| AppError::ChapterExtractionError(
                format!("Failed to extract chapter {}", spine_index)
            ))?;

        // Extract sections from TOC
        let sections = toc.get(&spine_id)
            .map(|entry| entry.sections.clone())
            .unwrap_or_default();

        chapters.push(Chapter {
            title,
            sections,
            content_lines: Vec::new(), // Will be rendered after parsing
            file_path: content_html, // Store HTML content here for now
        });
    }

    Ok(Book {
        metadata,
        chapters,
    })
}

fn parse_metadata(doc: &EpubDoc<std::io::BufReader<std::fs::File>>) -> BookMetadata {
    // MetadataItem has a 'value' field that contains the actual string
    BookMetadata {
        title: doc.mdata("title")
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
        // Extract the content path (this is the resource ID)
        let content_str = nav_point.content.to_string_lossy().to_string();
        
        // Remove fragment identifier (#...) from content to get the base path
        let base_path = content_str.split('#').next().unwrap_or(&content_str).to_string();
        
        // Create TOC entry with title
        let entry = TocEntry {
            title: Some(nav_point.label.clone()),
            sections: Vec::new(), // We'll populate sections later if needed
        };
        
        toc_map.insert(base_path, entry);
    }
    
    toc_map
}
