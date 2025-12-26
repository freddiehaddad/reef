use crate::error::{AppError, Result};
use crate::types::{Book, BookMetadata, Chapter};
use epub::doc::EpubDoc;
use std::path::Path;

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

    // Parse chapters
    let mut chapters = Vec::new();
    
    // Get the spine (reading order)
    let spine_len = doc.spine.len();
    
    for spine_index in 0..spine_len {
        #[allow(deprecated)]
        doc.set_current_page(spine_index);
        
        // Get chapter title - simplified for Phase 1
        let title = format!("Chapter {}", spine_index + 1);

        // Get HTML content - get_current_str() returns (content, mime_type)
        let (content_html, _mime_type) = doc.get_current_str()
            .ok_or_else(|| AppError::ChapterExtractionError(
                format!("Failed to extract chapter {}", spine_index)
            ))?;

        chapters.push(Chapter {
            title,
            sections: Vec::new(), // Will be populated in Phase 2
            content_lines: Vec::new(), // Will be rendered after parsing
            file_path: content_html, // Store HTML content here for Phase 1
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
