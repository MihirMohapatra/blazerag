use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileFormat {
    Pdf,
    Html,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Encoding {
    Utf8,
    Base64,
}

impl FromStr for FileFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdf" => Ok(Self::Pdf),
            "html" | "htm" => Ok(Self::Html),
            "md" | "markdown" => Ok(Self::Markdown),
            _ => anyhow::bail!("unsupported format: {s} (expected pdf, html, or markdown)"),
        }
    }
}

impl FromStr for Encoding {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "utf-8" | "utf8" | "text" => Ok(Self::Utf8),
            "base64" => Ok(Self::Base64),
            _ => anyhow::bail!("unsupported encoding: {s} (expected utf-8 or base64)"),
        }
    }
}

pub fn parse_text(content: &str, format: FileFormat, encoding: Encoding) -> anyhow::Result<String> {
    let decoded = match encoding {
        Encoding::Utf8 => content.to_string(),
        Encoding::Base64 => {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(content)
                .map_err(|e| anyhow::anyhow!("Base64 decode failed: {e}"))?;
            String::from_utf8(bytes).map_err(|e| {
                anyhow::anyhow!("Content is not valid UTF-8 after base64 decode: {e}")
            })?
        }
    };

    match format {
        FileFormat::Pdf => extract_pdf(&decoded),
        FileFormat::Html => extract_html(&decoded),
        FileFormat::Markdown => extract_markdown(&decoded),
    }
}

fn extract_pdf(content: &str) -> anyhow::Result<String> {
    let bytes = content.as_bytes();
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| anyhow::anyhow!("PDF extraction failed: {e}"))?;
    Ok(text)
}

fn extract_html(content: &str) -> anyhow::Result<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(content);
    let selectors = [
        "p",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "li",
        "td",
        "th",
        "blockquote",
        "pre",
        "code",
        "a",
        "figcaption",
        "summary",
    ];

    let mut text = String::with_capacity(content.len() / 2);
    for sel_str in &selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for element in document.select(&sel) {
                let inner: String = element.text().collect::<Vec<_>>().join(" ");
                let trimmed = inner.trim();
                if !trimmed.is_empty() {
                    text.push_str(trimmed);
                    text.push('\n');
                }
            }
        }
    }

    if text.is_empty() {
        anyhow::bail!("No extractable text found in HTML");
    }
    Ok(text)
}

fn extract_markdown(content: &str) -> anyhow::Result<String> {
    use pulldown_cmark::{Event, Parser, TagEnd};

    let parser = Parser::new(content);
    let mut text = String::with_capacity(content.len());
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                if matches!(tag, pulldown_cmark::Tag::CodeBlock(_)) {
                    in_code_block = true;
                }
            }
            Event::End(tag_end) => {
                if matches!(tag_end, TagEnd::CodeBlock) {
                    in_code_block = false;
                    text.push('\n');
                }
                if matches!(
                    tag_end,
                    TagEnd::Paragraph | TagEnd::Heading { .. } | TagEnd::List { .. }
                ) {
                    text.push('\n');
                }
            }
            Event::Text(t) | Event::Code(t) => {
                text.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
            }
            _ => {}
        }
    }

    if in_code_block {
        text.push_str(content);
        return Ok(text);
    }

    let text = text.trim().to_string();
    if text.is_empty() {
        anyhow::bail!("No extractable text found in Markdown");
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_simple() {
        let md = "# Hello\n\nThis is **bold** and `code`.";
        let text = extract_markdown(md).unwrap();
        assert!(text.contains("Hello"));
        assert!(text.contains("bold"));
        assert!(text.contains("code"));
    }

    #[test]
    fn test_parse_markdown_list() {
        let md = "- item one\n- item two\n- item three";
        let text = extract_markdown(md).unwrap();
        assert!(text.contains("item one"));
        assert!(text.contains("item two"));
    }

    #[test]
    fn test_parse_html_simple() {
        let html = "<html><body><p>Hello world</p><h1>Title</h1></body></html>";
        let text = extract_html(html).unwrap();
        assert!(text.contains("Hello world"));
        assert!(text.contains("Title"));
    }

    #[test]
    fn test_parse_html_empty() {
        let html = "<html><body></body></html>";
        let result = extract_html(html);
        assert!(result.is_err());
    }

    #[test]
    fn test_encoding_from_str() {
        assert_eq!("utf-8".parse::<Encoding>().unwrap(), Encoding::Utf8);
        assert_eq!("base64".parse::<Encoding>().unwrap(), Encoding::Base64);
        assert!("invalid".parse::<Encoding>().is_err());
    }

    #[test]
    fn test_format_from_str() {
        assert_eq!("pdf".parse::<FileFormat>().unwrap(), FileFormat::Pdf);
        assert_eq!("html".parse::<FileFormat>().unwrap(), FileFormat::Html);
        assert_eq!("htm".parse::<FileFormat>().unwrap(), FileFormat::Html);
        assert_eq!("md".parse::<FileFormat>().unwrap(), FileFormat::Markdown);
        assert_eq!(
            "markdown".parse::<FileFormat>().unwrap(),
            FileFormat::Markdown
        );
        assert!("invalid".parse::<FileFormat>().is_err());
    }

    #[test]
    fn test_base64_roundtrip() {
        let text = "Hello, world!";
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        let decoded = parse_text(&encoded, FileFormat::Markdown, Encoding::Base64).unwrap();
        assert_eq!(decoded, text);
    }
}
