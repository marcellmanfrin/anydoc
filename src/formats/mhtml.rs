//! MHTML/MHT MIME container frontend.

use crate::error::ConvertError;
use crate::model::Document;
use crate::package::limits;
use mail_parser::{MessageParser, MimeHeaders};

const HEADER_SCAN_LIMIT: usize = 64 * 1024;

pub(crate) fn looks_like_mhtml(bytes: &[u8]) -> bool {
    let header = mime_header_block(bytes);
    let unfolded = unfold_headers(header);
    let lower = unfolded.to_ascii_lowercase();
    let snapshot = lower.lines().any(|line| line.starts_with("snapshot-content-location:"));

    let Some(content_type) = lower.lines().find_map(|line| line.strip_prefix("content-type:")) else {
        return false;
    };
    let compact: String = content_type.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if !compact.starts_with("multipart/related") {
        return false;
    }

    snapshot
        || compact.contains("type=\"text/html\"")
        || compact.contains("type=text/html")
        || compact.contains("type='text/html'")
}

fn mime_header_block(bytes: &[u8]) -> &[u8] {
    let bytes = &bytes[..bytes.len().min(HEADER_SCAN_LIMIT)];
    let crlf = bytes.windows(4).position(|w| w == b"\r\n\r\n");
    let lf = bytes.windows(2).position(|w| w == b"\n\n");
    let end = match (crlf, lf) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => bytes.len(),
    };
    &bytes[..end]
}

fn unfold_headers(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    for raw in text.split('\n') {
        let line = raw.trim_end_matches('\r');
        if line.starts_with([' ', '\t']) {
            out.push(' ');
            out.push_str(line.trim());
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Document, ConvertError> {
    if bytes.len() as u64 > limits::MAX_TOTAL_BYTES {
        return Err(ConvertError::ResourceLimit {
            limit: "max_total_bytes",
            detail: format!(
                "MHTML input is {} bytes; maximum is {}",
                bytes.len(),
                limits::MAX_TOTAL_BYTES
            ),
        });
    }

    let message = MessageParser::new().with_mime_headers().parse(bytes).ok_or_else(|| {
        ConvertError::malformed("MHTML MIME structure could not be parsed")
    })?;

    if !message.is_content_type("multipart", "related") {
        return Err(ConvertError::malformed(
            "MHTML root is not multipart/related",
        ));
    }

    let start = message
        .content_type()
        .and_then(|content_type| content_type.attribute("start"))
        .map(normalize_content_id);

    let html_part = if let Some(start) = start {
        message.parts.iter().find(|part| {
            part.is_content_type("text", "html")
                && part
                    .content_id()
                    .map(normalize_content_id)
                    .is_some_and(|content_id| content_id == start)
        })
    } else {
        message.parts.iter().find(|part| part.is_content_type("text", "html"))
    }
    .ok_or_else(|| ConvertError::malformed("MHTML contains no HTML root part"))?;

    let html = html_part
        .text_contents()
        .ok_or_else(|| ConvertError::malformed("MHTML HTML root is not decodable text"))?;
    if html.len() as u64 > limits::MAX_ENTRY_BYTES {
        return Err(ConvertError::ResourceLimit {
            limit: "max_entry_bytes",
            detail: format!(
                "MHTML HTML root is {} bytes; maximum is {}",
                html.len(),
                limits::MAX_ENTRY_BYTES
            ),
        });
    }

    super::html::parse(html.as_bytes())
}

fn normalize_content_id(value: &str) -> String {
    let value = value.trim();
    let value = value.strip_prefix("cid:").unwrap_or(value);
    value.trim_matches(|c| matches!(c, '<' | '>')).to_string()
}
