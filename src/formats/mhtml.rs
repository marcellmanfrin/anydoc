//! MHTML/MHT MIME container frontend.

use crate::error::ConvertError;
use crate::model::Document;

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

pub(crate) fn parse(_bytes: &[u8]) -> Result<Document, ConvertError> {
    Err(ConvertError::Unsupported("MHTML parsing is not implemented yet".into()))
}
