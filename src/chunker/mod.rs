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
        if let Some(overlap_text) = suffix_by_chars(chunk[0], config.chunk_overlap) {
            let combined = format!("{}{}", overlap_text, chunk[1]);
            overlapped.push(combined);
        }
    }
    if let Some(last) = chunks.last() {
        overlapped.push(last.to_string());
    }

    overlapped
}

fn suffix_by_chars(text: &str, max_chars: usize) -> Option<&str> {
    if max_chars == 0 {
        return None;
    }

    let start = text
        .char_indices()
        .rev()
        .nth(max_chars.saturating_sub(1))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    Some(&text[start..])
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

    #[test]
    fn test_multibyte_overlap_does_not_panic() {
        let config = ChunkerConfig {
            chunk_size: 24,
            chunk_overlap: 5,
        };
        let text = "Rust handles multilingual text: नमस्ते दुनिया. More content follows.";
        let chunks = chunk_text(text, &config);
        assert!(!chunks.is_empty());
        assert!(chunks
            .iter()
            .all(|chunk| chunk.is_char_boundary(chunk.len())));
    }

    #[test]
    fn test_suffix_by_chars_handles_multibyte_text() {
        assert_eq!(suffix_by_chars("abcé😀z", 3), Some("é😀z"));
        assert_eq!(suffix_by_chars("नमस्ते", 99), Some("नमस्ते"));
        assert_eq!(suffix_by_chars("नमस्ते", 0), None);
    }
}
