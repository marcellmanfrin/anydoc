from pathlib import Path
import json

ROOT = Path('.')


def replace_once(path, old, new):
    p = ROOT / path
    text = p.read_text()
    if old not in text:
        raise SystemExit(f'missing replacement marker in {path}: {old[:80]!r}')
    p.write_text(text.replace(old, new, 1))


def replace_section(path, start, end, replacement):
    p = ROOT / path
    text = p.read_text()
    i = text.find(start)
    if i < 0:
        raise SystemExit(f'missing start marker in {path}: {start!r}')
    j = text.find(end, i)
    if j < 0:
        raise SystemExit(f'missing end marker in {path}: {end!r}')
    p.write_text(text[:i] + replacement + text[j:])


html_section = '''pub(crate) fn parse_text_with_context(
    text: &str,
    ordered_stylesheets: Option<&[String]>,
    ctx: &dyn HtmlCtx,
    assets: Vec<Asset>,
) -> Result<Document, ConvertError> {
    preflight_html_complexity(text)?;

    let parsed = Html::parse_document(text);
    document_from_parsed_html(&parsed, ordered_stylesheets, ctx, assets)
}

pub(crate) fn document_from_parsed_html(
    parsed: &Html,
    ordered_stylesheets: Option<&[String]>,
    ctx: &dyn HtmlCtx,
    assets: Vec<Asset>,
) -> Result<Document, ConvertError> {
    let root = parsed.root_element();

    let mut css = Stylesheet::default();
    if let Some(stylesheets) = ordered_stylesheets {
        for stylesheet in stylesheets {
            css.add(stylesheet);
        }
    } else {
        for style in root.descendent_elements().filter(|e| e.value().name() == "style") {
            css.add(&style.text().collect::<String>());
        }
    }

    let body = root
        .descendent_elements()
        .find(|e| e.value().name() == "body")
        .ok_or_else(|| ConvertError::malformed("HTML parser produced no body element"))?;

    let mut node_count = 0usize;
    let body = adapt_element(body, 1, &mut node_count)?;
    let blocks = crate::shared::html::to_blocks(&body, &css, ctx)?;

    Ok(Document { assets, blocks, ..Document::default() })
}

'''
replace_section(
    'src/formats/html.rs',
    'pub(crate) fn parse_text_with_context(',
    'pub(crate) fn decode_html(',
    html_section,
)
replace_once(
    'src/formats/html.rs',
    '''    if name == "a"\n        && let Some(position) = open.iter().rposition(|candidate| candidate.as_ref() == "a")\n    {\n        open.truncate(position);\n    }''',
    '''    if name == "a"\n        && let Some(position) = open.iter().rposition(|candidate| candidate.as_ref() == "a")\n    {\n        open.remove(position);\n    }''',
)

replace_once(
    'src/lib.rs',
    '''    pub fn from_bytes(bytes: &[u8]) -> Option<Format> {\n        if formats::mhtml::looks_like_mhtml(bytes) {\n            Some(Format::Mhtml)\n        } else {\n            formats::detect::from_bytes(bytes)\n        }\n    }''',
    '''    pub fn from_bytes(bytes: &[u8]) -> Option<Format> {\n        formats::detect::from_bytes(bytes)\n            .or_else(|| formats::mhtml::looks_like_mhtml(bytes).then_some(Format::Mhtml))\n    }''',
)

replace_once(
    'src/formats/mhtml.rs',
    '''use mail_parser::decoders::{base64::base64_decode, quoted_printable::quoted_printable_decode};\nuse mail_parser::{Encoding, Message, MessageParser, MessagePart, MimeHeaders};''',
    '''use mail_parser::{MessageParser, MessagePart, MimeHeaders};''',
)
replace_once(
    'src/formats/mhtml.rs',
    '''    let message = MessageParser::new()\n        .with_mime_headers()''',
    '''    preflight_base64_decoder_allocations(bytes)?;\n\n    let message = MessageParser::new()\n        .with_mime_headers()''',
)
replace_once(
    'src/formats/mhtml.rs',
    '''    let html_bytes = transfer_decoded_part_bytes(&message, html_part, "HTML root")?;\n    if html_bytes.len() as u64 > limits::MAX_ENTRY_BYTES {\n        return Err(ConvertError::ResourceLimit {\n            limit: "max_entry_bytes",\n            detail: format!(\n                "MHTML HTML root is {} bytes; maximum is {}",\n                html_bytes.len(),\n                limits::MAX_ENTRY_BYTES\n            ),\n        });\n    }''',
    '''    let html_bytes = decoded_part_bytes(html_part, "HTML root")?;''',
)
replace_once(
    'src/formats/mhtml.rs',
    '''    super::html::parse_text_with_context(&html, Some(&stylesheets), &ctx, assets)''',
    '''    super::html::document_from_parsed_html(&parsed_html, Some(&stylesheets), &ctx, assets)''',
)

mhtml_helpers = '''fn preflight_base64_decoder_allocations(bytes: &[u8]) -> Result<(), ConvertError> {
    preflight_base64_decoder_allocations_with_limit(bytes, limits::MAX_ENTRY_BYTES)
}

fn preflight_base64_decoder_allocations_with_limit(
    bytes: &[u8],
    max_entry_bytes: u64,
) -> Result<(), ConvertError> {
    let mut offset = 0usize;
    let mut in_headers = true;
    let mut transfer_header = false;
    let mut transfer_is_base64 = false;

    while offset < bytes.len() {
        let newline = bytes[offset..].iter().position(|&byte| byte == b'\\n');
        let line_end = newline.map_or(bytes.len(), |index| offset + index);
        let next_offset = newline.map_or(bytes.len(), |_| line_end + 1);
        let mut line = &bytes[offset..line_end];
        if line.last() == Some(&b'\\r') {
            line = &line[..line.len() - 1];
        }

        if in_headers {
            if line.is_empty() {
                if transfer_is_base64 {
                    let reserve = (bytes.len().saturating_sub(next_offset) as u64 / 4)
                        .saturating_mul(3);
                    if reserve > max_entry_bytes {
                        return Err(ConvertError::ResourceLimit {
                            limit: "max_entry_bytes",
                            detail: format!(
                                "MHTML base64 decoder may reserve {reserve} bytes before locating the MIME boundary; maximum is {max_entry_bytes}"
                            ),
                        });
                    }
                }
                in_headers = false;
                transfer_header = false;
                transfer_is_base64 = false;
            } else if matches!(line.first(), Some(b' ' | b'\\t')) {
                if transfer_header && ascii_trim(line).eq_ignore_ascii_case(b"base64") {
                    transfer_is_base64 = true;
                }
            } else if let Some(colon) = line.iter().position(|&byte| byte == b':') {
                transfer_header = line[..colon]
                    .eq_ignore_ascii_case(b"content-transfer-encoding");
                transfer_is_base64 = transfer_header
                    && ascii_trim(&line[colon + 1..]).eq_ignore_ascii_case(b"base64");
            } else {
                in_headers = false;
                transfer_header = false;
                transfer_is_base64 = false;
            }
        } else if line.starts_with(b"--") && line.len() > 2 {
            in_headers = true;
            transfer_header = false;
            transfer_is_base64 = false;
        }

        offset = next_offset;
    }

    Ok(())
}

fn ascii_trim(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn decoded_part_bytes(part: &MessagePart<'_>, label: &str) -> Result<Vec<u8>, ConvertError> {
    let bytes = part.contents();
    if bytes.len() as u64 > limits::MAX_ENTRY_BYTES {
        return Err(ConvertError::ResourceLimit {
            limit: "max_entry_bytes",
            detail: format!(
                "MHTML {label} is {} bytes; maximum is {}",
                bytes.len(),
                limits::MAX_ENTRY_BYTES
            ),
        });
    }
    Ok(bytes.to_vec())
}

'''
replace_section(
    'src/formats/mhtml.rs',
    'fn transfer_decoded_part_bytes(',
    'fn html_resource_base(',
    mhtml_helpers,
)

mhtml_path = ROOT / 'src/formats/mhtml.rs'
mhtml_text = mhtml_path.read_text()
if 'fn base64_decoder_preflight_rejects_large_reserve_before_mime_parse()' not in mhtml_text:
    mhtml_text += '''\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn base64_decoder_preflight_rejects_large_reserve_before_mime_parse() {\n        let input = b"MIME-Version: 1.0\\r\\nContent-Type: multipart/related; boundary=\\\"b\\\"\\r\\n\\r\\n--b\\r\\nContent-Type: text/html\\r\\nContent-Transfer-Encoding: base64\\r\\n\\r\\nQQ==\\r\\n--b\\r\\nContent-Type: text/plain\\r\\n\\r\\n0123456789abcdef\\r\\n--b--\\r\\n";\n        assert!(matches!(\n            preflight_base64_decoder_allocations_with_limit(input, 8),\n            Err(ConvertError::ResourceLimit { limit: "max_entry_bytes", .. })\n        ));\n    }\n\n    #[test]\n    fn base64_decoder_preflight_allows_small_remaining_input() {\n        let input = b"Content-Transfer-Encoding: base64\\r\\n\\r\\nQQ==\\r\\n";\n        assert!(preflight_base64_decoder_allocations_with_limit(input, 64).is_ok());\n    }\n}\n'''
    mhtml_path.write_text(mhtml_text)

replace_once(
    'python/anydoc/__init__.py',
    '''    "doc", "docx", "odt", "pdf", "ppt", "pptx", "rtf", "epub", "xlsx", "ods", "odp", "csv"''',
    '''    "doc", "docx", "odt", "pdf", "ppt", "pptx", "rtf", "epub", "html", "mhtml", "xlsx", "ods", "odp", "csv"''',
)

replace_once(
    'tests/mhtml_charset.rs',
    '''<!doctype html><meta http-equiv=3D\"Content-Type\" content=3D\"text/html; charset=3Dwindows-1252\"><p>Contrata=E7=E3o dever=E1</p>''',
    '''<!doctype html><meta http-equiv=3D\"Content-Type\" content=3D\"text/html; charset=3Diso-8859-2\"><p>=A3</p>''',
)
replace_once(
    'tests/mhtml_charset.rs',
    '''    assert_eq!(to_markdown_bytes(mhtml, Some(Format::Mhtml)).unwrap(), "Contratação deverá\\n");''',
    '''    assert_eq!(to_markdown_bytes(mhtml, Some(Format::Mhtml)).unwrap(), "Ł\\n");''',
)

(ROOT / 'tests/mhtml_resources.rs').write_text('''use anydoc::model::{Block, ImageSource, Inline};\nuse anydoc::{Format, to_document, to_markdown_bytes};\n\nfn mhtml_fixture(source: &str) -> Vec<u8> {\n    source.replace('\\n', "\\r\\n").into_bytes()\n}\n\n#[test]\nfn bare_relative_image_does_not_match_content_id() {\n    let mhtml = mhtml_fixture(\n        r#"MIME-Version: 1.0\nContent-Type: multipart/related; type=\"text/html\"; boundary=\"b\"\n\n--b\nContent-Type: text/html; charset=utf-8\n\n<!doctype html><p><img alt=\"pixel\" src=\"image@id\"></p>\n--b\nContent-Type: image/png\nContent-ID: <image@id>\nContent-Transfer-Encoding: base64\n\nAAECAw==\n--b--\n"#,\n    );\n    let document = to_document(&mhtml, Some(Format::Mhtml)).unwrap();\n    match &document.blocks[0] {\n        Block::Paragraph(inlines) => {\n            assert!(matches!(&inlines[0], Inline::Image { source: ImageSource::Unavailable, .. }))\n        }\n        other => panic!("expected paragraph, got {other:?}"),\n    }\n}\n\n#[test]\nfn content_location_fragment_is_ignored_for_resource_lookup() {\n    let mhtml = mhtml_fixture(\n        r#"MIME-Version: 1.0\nContent-Type: multipart/related; type=\"text/html\"; boundary=\"b\"\n\n--b\nContent-Type: text/html; charset=utf-8\nContent-Location: https://example.test/page.html\n\n<!doctype html><link rel=\"stylesheet\" href=\"https://example.test/site.css\"><p class=\"strong\">keep me</p>\n--b\nContent-Type: text/css; charset=utf-8\nContent-Location: https://example.test/site.css#saved\n\n.strong { font-weight: bold }\n--b--\n"#,\n    );\n    assert_eq!(to_markdown_bytes(&mhtml, Some(Format::Mhtml)).unwrap(), "**keep me**\\n");\n}\n''')

(ROOT / 'tests/html_preflight.rs').write_text('''use anydoc::{Format, to_markdown_bytes};\n\n#[test]\nfn repeated_unclosed_anchors_convert_successfully() {\n    let mut html = String::from("<!doctype html>");\n    for index in 0..300 {\n        html.push_str(&format!("<a href=\\\"#{index}\\\">link"));\n    }\n    to_markdown_bytes(html.as_bytes(), Some(Format::Html))\n        .expect("HTML5 repairs repeated anchors without excessive nesting");\n}\n''')

(ROOT / 'tests/format_detection.rs').write_text('''use anydoc::Format;\n\n#[test]\nfn strong_rtf_signature_wins_over_mhtml_header_heuristic() {\n    let rtf = b"{\\\\rtf1\\\\ansi\\nSnapshot-Content-Location: https://example.test/page\\nContent-Type: multipart/related; boundary=\\\"b\\\"\\n\\nplain text}";\n\n    assert_eq!(Format::from_bytes(rtf), Some(Format::Rtf));\n}\n''')

review_followup = ROOT / 'tests/review_followup.rs'
if review_followup.exists():
    review_followup.unlink()

replace_once(
    'tests/html_corpus.rs',
    '''#[test]\nfn libreoffice_xlsx_merged_table_matches_source_markdown() {\n    let source = to_markdown(fixture_root().join("xlsx").join("handmade-merged.xlsx")).unwrap();\n    let html = to_markdown(html_fixture("libreoffice-xlsx-merged.html")).unwrap();\n    assert_eq!(html, source);\n}''',
    '''#[test]\nfn libreoffice_xlsx_merged_table_preserves_real_export_content() {\n    let html = to_markdown(html_fixture("libreoffice-xlsx-merged.html")).unwrap();\n    for value in ["Merged across", "padded", "tall", "b2", "3.5", "b3"] {\n        assert!(html.contains(value), "HTML Markdown missing {value:?}");\n    }\n}''',
)

replace_once(
    'README.md',
    'Fast Rust library that converts documents (Word, PowerPoint, Excel, OpenDocument, RTF, EPUB, CSV, and PDF) into clean GitHub-Flavored Markdown.',
    'Fast Rust library that converts documents (Word, PowerPoint, Excel, OpenDocument, RTF, EPUB, HTML, MHTML, CSV, and PDF) into clean GitHub-Flavored Markdown.',
)
replace_once(
    'README.md',
    'The format is read from the file content, using the marker its specification designates: the PDF header, the RTF open group, OLE stream names, the ZIP package mimetype and content types. CSV has no such marker, so the extension or an explicit format names it instead.',
    'The format is read from the file content, using the marker its specification designates: the PDF header, the RTF open group, OLE stream names, ZIP package mimetype/content types, HTML document markers, and the conservative `multipart/related` snapshot headers used by MHTML. CSV has no such marker, so the extension or an explicit format names it instead.',
)
replace_once(
    'README.md',
    '''  ├─► format parser          → one per format (doc, docx, ppt, pptx, xls,\n  │                            xlsx, odt/ods/odp, rtf, epub, csv)''',
    '''  ├─► format parser          → one per format (doc, docx, ppt, pptx, xls,\n  │                            xlsx, odt/ods/odp, rtf, epub, html, mhtml, csv)''',
)

replace_once(
    'node/README.md',
    'Convert Word, PowerPoint, Excel, OpenDocument, RTF, EPUB, CSV, and PDF files into clean GitHub-Flavored Markdown.',
    'Convert Word, PowerPoint, Excel, OpenDocument, RTF, EPUB, HTML, MHTML, CSV, and PDF files into clean GitHub-Flavored Markdown.',
)
replace_once(
    'node/README.md',
    '''| EPUB             | `.epub`                                                    |\n| CSV              | `.csv`                                                     |''',
    '''| EPUB             | `.epub`                                                    |\n| HTML             | `.html`, `.htm`                                             |\n| MHTML            | `.mhtml`, `.mht`                                           |\n| CSV              | `.csv`                                                     |''',
)
replace_once(
    'node/README.md',
    'The format is read from the file content, using the marker its specification designates: the PDF header, the RTF open group, OLE stream names, the ZIP package mimetype and content types. CSV has no such marker, so detection returns `null` for it and the extension, or an explicit format, names it instead.',
    'The format is read from the file content, using the marker its specification designates: the PDF header, the RTF open group, OLE stream names, ZIP package mimetype/content types, HTML document markers, and the conservative `multipart/related` snapshot headers used by MHTML. CSV has no such marker, so detection returns `null` for it and the extension, or an explicit format, names it instead.',
)

package_path = ROOT / 'node/package.json'
package = json.loads(package_path.read_text())
package['description'] = 'Convert documents (doc, docx, odt, rtf, epub, html, mhtml, pdf, presentations, spreadsheets, csv) to GitHub-Flavored Markdown'
for keyword in ['html', 'mhtml']:
    if keyword not in package['keywords']:
        package['keywords'].append(keyword)
package_path.write_text(json.dumps(package, indent=2) + '\n')

(ROOT / 'python/tests/test_format_alias.py').write_text('''import typing\nimport unittest\n\nimport anydoc\n\n\nclass FormatAliasTests(unittest.TestCase):\n    def test_public_format_alias_includes_html_and_mhtml(self):\n        formats = typing.get_args(anydoc.Format)\n        self.assertIn("html", formats)\n        self.assertIn("mhtml", formats)\n\n\nif __name__ == "__main__":\n    unittest.main()\n''')
