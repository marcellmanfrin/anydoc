use anydoc::Format;

fn utf16_with_bom(source: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian { vec![0xFF, 0xFE] } else { vec![0xFE, 0xFF] };
    for unit in source.encode_utf16() {
        if little_endian {
            bytes.extend_from_slice(&unit.to_le_bytes());
        } else {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
    }
    bytes
}

#[test]
fn utf16le_doctype_allows_long_whitespace_between_keyword_and_name() {
    let source = format!("<!DOCTYPE{}html><html></html>", " ".repeat(80));
    let bytes = utf16_with_bom(&source, true);
    assert_eq!(Format::from_bytes(&bytes), Some(Format::Html));
}

#[test]
fn utf16be_doctype_allows_long_whitespace_between_keyword_and_name() {
    let source = format!("<!DOCTYPE{}html><html></html>", " ".repeat(80));
    let bytes = utf16_with_bom(&source, false);
    assert_eq!(Format::from_bytes(&bytes), Some(Format::Html));
}
