from pathlib import Path
import shutil

root = Path('.')

def rep(path, old, new):
    p = root / path
    s = p.read_text()
    if s.count(old) != 1:
        raise RuntimeError(f'{path}: expected one match for {old[:50]!r}, got {s.count(old)}')
    p.write_text(s.replace(old, new, 1))

rep('Cargo.toml', 'quick-xml = "0.41.0"\n', 'quick-xml = "0.41.0"\nscraper = { version = "0.27.0", default-features = false }\n')
rep('src/lib.rs', '    /// EPUB 2 and 3 (`.epub`).\n    Epub,\n', '    /// EPUB 2 and 3 (`.epub`).\n    Epub,\n    /// Standalone HTML5 (`.html`, `.htm`). JavaScript is not executed.\n    Html,\n')
rep('src/lib.rs', '            "epub" => Format::Epub,\n', '            "epub" => Format::Epub,\n            "html" | "htm" => Format::Html,\n')
rep('src/formats/mod.rs', 'mod epub;\n', 'mod epub;\nmod html;\n')
rep('src/formats/mod.rs', '        Format::Epub => epub::parse(bytes),\n', '        Format::Epub => epub::parse(bytes),\n        Format::Html => html::parse(bytes),\n')
old = '''    if bytes[..bytes.len().min(1024)].windows(5).any(|w| w == b"%PDF-") {\n        return Some(Format::Pdf);\n    }\n    None\n}\n'''
new = '''    if bytes[..bytes.len().min(1024)].windows(5).any(|w| w == b"%PDF-") {\n        return Some(Format::Pdf);\n    }\n    if looks_like_html(bytes) {\n        return Some(Format::Html);\n    }\n    None\n}\n\nfn looks_like_html(bytes: &[u8]) -> bool {\n    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);\n    let bytes = bytes.trim_ascii_start();\n    html_prefix(bytes, b"<!doctype html") || html_prefix(bytes, b"<html")\n}\n\nfn html_prefix(bytes: &[u8], prefix: &[u8]) -> bool {\n    let Some(head) = bytes.get(..prefix.len()) else { return false; };\n    head.eq_ignore_ascii_case(prefix)\n        && bytes.get(prefix.len()).is_none_or(|b| b.is_ascii_whitespace() || matches!(b, b'>' | b'/'))\n}\n'''
rep('src/formats/detect.rs', old, new)
rep('src/formats/detect.rs', '        assert_eq!(from_bytes(b"{\\\\rtf1\\\\ansi hi}"), Some(Format::Rtf));\n', '        assert_eq!(from_bytes(b"{\\\\rtf1\\\\ansi hi}"), Some(Format::Rtf));\n        assert_eq!(from_bytes(b"<!DOCTYPE html><html></html>"), Some(Format::Html));\n        assert_eq!(from_bytes(b"\\xEF\\xBB\\xBF  <HTML><body>x</body></HTML>"), Some(Format::Html));\n')
shutil.copyfile('.github/html-support/html.rs', 'src/formats/html.rs')
shutil.copyfile('.github/html-support/tests-html.rs', 'tests/html.rs')
