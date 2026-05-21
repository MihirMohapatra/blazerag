use text_splitter::TextSplitter;

#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 64,
        }
    }
}

pub fn chunk_text(text: &str, config: &ChunkerConfig) -> Vec<String> {
    let splitter = TextSplitter::new(config.chunk_size);
    let chunks: Vec<&str> = splitter.chunks(text).collect();

    if config.chunk_overlap == 0 || chunks.len() <= 1 {
        return chunks.into_iter().map(String::from).collect();
    }

    let mut overlapped = Vec::new();
    for chunk in chunks.windows(2) {
        overlapped.push(chunk[0].to_string());
        let overlap_len = config.chunk_overlap.min(chunk[0].len());
        if overlap_len > 0 {
            let overlap_text = &chunk[0][chunk[0].len() - overlap_len..];
            let combined = format!("{}{}", overlap_text, chunk[1]);
            overlapped.push(combined);
        }
    }
    if let Some(last) = chunks.last() {
        overlapped.push(last.to_string());
    }

    overlapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_basic() {
        let config = ChunkerConfig {
            chunk_size: 50,
            chunk_overlap: 0,
        };
        let text = "Hello world. This is a test document. It has several sentences.";
        let chunks = chunk_text(text, &config);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    #[test]
    fn test_chunk_with_overlap() {
        let config = ChunkerConfig {
            chunk_size: 50,
            chunk_overlap: 10,
        };
        let text = "A. B. C. D. E. F. G. H. I. J. K. L. M. N. O. P.";
        let chunks = chunk_text(text, &config);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_empty_text() {
        let config = ChunkerConfig::default();
        let chunks = chunk_text("", &config);
        assert!(chunks.is_empty());
    }
}
