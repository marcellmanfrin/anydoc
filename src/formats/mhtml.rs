//! MHTML/MHT MIME container frontend.

use crate::error::ConvertError;
use crate::model::{AnchorId, Asset, AssetId, Document, ImageSource, LinkTarget};
use crate::package::limits;
use crate::shared::html::HtmlCtx;
use crate::shared::uri::is_absolute_uri;
use mail_parser::{MessageParser, MimeHeaders};
use scraper::Html;
use std::collections::HashMap;

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
        return Err(ConvertError::malformed("MHTML root is not multipart/related"));
    }

    let start = message
        .content_type()
        .and_then(|content_type| content_type.attribute("start"))
        .map(normalize_content_id);

    let (_, html_part) = if let Some(start) = start {
        message.parts.iter().enumerate().find(|(_, part)| {
            part.is_content_type("text", "html")
                && part
                    .content_id()
                    .map(normalize_content_id)
                    .is_some_and(|content_id| content_id == start)
        })
    } else {
        message.parts.iter().enumerate().find(|(_, part)| part.is_content_type("text", "html"))
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

    let resource_index = build_resource_index(&message.parts);
    let extra_css = collect_linked_css(html, &message.parts, &resource_index)?;
    let (assets, image_assets) = collect_image_assets(&message.parts)?;
    let ctx = MhtmlCtx { image_assets };

    super::html::parse_text_with_context(html, &extra_css, &ctx, assets)
}

fn build_resource_index(parts: &[mail_parser::MessagePart<'_>]) -> HashMap<String, usize> {
    let mut index = HashMap::new();
    for (part_index, part) in parts.iter().enumerate() {
        for key in resource_keys(part) {
            index.entry(key).or_insert(part_index);
        }
    }
    index
}

fn collect_linked_css(
    html: &str,
    parts: &[mail_parser::MessagePart<'_>],
    resource_index: &HashMap<String, usize>,
) -> Result<Vec<String>, ConvertError> {
    let parsed = Html::parse_document(html);
    let root = parsed.root_element();
    let mut stylesheets = Vec::new();

    for link in root.descendent_elements().filter(|element| element.value().name() == "link") {
        let rel = link.value().attr("rel").unwrap_or("");
        if !rel.split_ascii_whitespace().any(|token| token.eq_ignore_ascii_case("stylesheet")) {
            continue;
        }
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let Some(&part_index) = resource_index.get(&canonical_reference(href)) else {
            continue;
        };
        let part = &parts[part_index];
        if !part.is_content_type("text", "css") {
            continue;
        }
        let Some(css) = part.text_contents() else {
            continue;
        };
        if css.len() as u64 > limits::MAX_ENTRY_BYTES {
            return Err(ConvertError::ResourceLimit {
                limit: "max_entry_bytes",
                detail: format!(
                    "MHTML stylesheet is {} bytes; maximum is {}",
                    css.len(),
                    limits::MAX_ENTRY_BYTES
                ),
            });
        }
        stylesheets.push(css.to_owned());
    }

    Ok(stylesheets)
}

fn collect_image_assets(
    parts: &[mail_parser::MessagePart<'_>],
) -> Result<(Vec<Asset>, HashMap<String, AssetId>), ConvertError> {
    let mut assets = Vec::new();
    let mut image_assets = HashMap::new();
    let mut total_bytes = 0u64;

    for (part_index, part) in parts.iter().enumerate() {
        let Some(content_type) = part.content_type() else {
            continue;
        };
        if !content_type.ctype().eq_ignore_ascii_case("image") {
            continue;
        }

        let bytes = part.contents();
        if bytes.len() as u64 > limits::MAX_ENTRY_BYTES {
            return Err(ConvertError::ResourceLimit {
                limit: "max_entry_bytes",
                detail: format!(
                    "MHTML image part is {} bytes; maximum is {}",
                    bytes.len(),
                    limits::MAX_ENTRY_BYTES
                ),
            });
        }
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if total_bytes > limits::MAX_ASSET_TOTAL_BYTES as u64 {
            return Err(ConvertError::ResourceLimit {
                limit: "max_asset_total_bytes",
                detail: format!(
                    "MHTML embedded images retain {total_bytes} bytes; maximum is {}",
                    limits::MAX_ASSET_TOTAL_BYTES
                ),
            });
        }

        let id = AssetId(assets.len());
        let subtype = content_type.subtype().unwrap_or("octet-stream");
        let origin_part = part
            .content_location()
            .or_else(|| part.content_id())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("mhtml-part-{part_index}"));
        assets.push(Asset {
            id,
            media_type: format!("{}/{subtype}", content_type.ctype()),
            origin_part,
            bytes: bytes.to_vec(),
        });
        for key in resource_keys(part) {
            image_assets.entry(key).or_insert(id);
        }
    }

    Ok((assets, image_assets))
}

fn resource_keys(part: &mail_parser::MessagePart<'_>) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(content_id) = part.content_id() {
        let id = normalize_content_id(content_id);
        if !id.is_empty() {
            keys.push(format!("cid:{id}"));
            keys.push(id);
        }
    }
    if let Some(location) = part.content_location() {
        let location = canonical_reference(location);
        if !location.is_empty() && !keys.contains(&location) {
            keys.push(location);
        }
    }
    keys
}

fn canonical_reference(value: &str) -> String {
    let value = value.trim();
    if let Some(content_id) = value.strip_prefix("cid:") {
        format!("cid:{}", normalize_content_id(content_id))
    } else {
        value.to_owned()
    }
}

fn normalize_content_id(value: &str) -> String {
    let value = value.trim();
    let value = value.strip_prefix("cid:").unwrap_or(value);
    value.trim_matches(|c| matches!(c, '<' | '>')).to_string()
}

struct MhtmlCtx {
    image_assets: HashMap<String, AssetId>,
}

impl HtmlCtx for MhtmlCtx {
    fn link_target(&self, href: &str) -> Option<LinkTarget> {
        let href = href.trim();
        if href.is_empty() {
            return None;
        }
        if let Some(fragment) = href.strip_prefix('#') {
            let fragment = crate::package::path::decode_fragment(fragment);
            return Some(LinkTarget::Anchor(fragment));
        }
        Some(if is_absolute_uri(href) {
            LinkTarget::External(href.to_owned())
        } else {
            LinkTarget::Relative(href.to_owned())
        })
    }

    fn image_source(&self, src: &str) -> Result<Option<ImageSource>, ConvertError> {
        let src = src.trim();
        if src.is_empty() {
            return Ok(None);
        }
        if let Some(&asset_id) = self.image_assets.get(&canonical_reference(src)) {
            return Ok(Some(ImageSource::Asset(asset_id)));
        }
        if src.get(..4).is_some_and(|prefix| prefix.eq_ignore_ascii_case("cid:")) {
            return Ok(Some(ImageSource::Unavailable));
        }
        Ok(is_absolute_uri(src).then(|| ImageSource::External(src.to_owned())))
    }

    fn anchor_id(&self, raw: &str) -> AnchorId {
        raw.to_owned()
    }
}
