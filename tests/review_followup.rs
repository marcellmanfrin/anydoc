use anydoc::model::{Block, ImageSource, Inline};
use anydoc::{ConvertError, Format, to_document, to_markdown_bytes};

fn mhtml_fixture(source: &str) -> Vec<u8> {
    source.replace('\n', "\r\n").into_bytes()
}

#[test]
fn bare_relative_image_does_not_match_content_id() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html; charset=utf-8

<!doctype html><p><img alt="pixel" src="image@id"></p>
--b
Content-Type: image/png
Content-ID: <image@id>
Content-Transfer-Encoding: base64

AAECAw==
--b--
"#,
    );
    let document = to_document(&mhtml, Some(Format::Mhtml)).unwrap();
    match &document.blocks[0] {
        Block::Paragraph(inlines) => assert!(matches!(
            &inlines[0],
            Inline::Image { source: ImageSource::Unavailable, .. }
        )),
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn content_location_fragment_is_ignored_for_resource_lookup() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html; charset=utf-8
Content-Location: https://example.test/page.html

<!doctype html><link rel="stylesheet" href="https://example.test/site.css"><p class="strong">keep me</p>
--b
Content-Type: text/css; charset=utf-8
Content-Location: https://example.test/site.css#saved

.strong { font-weight: bold }
--b--
"#,
    );
    assert_eq!(to_markdown_bytes(&mhtml, Some(Format::Mhtml)).unwrap(), "**keep me**\n");
}

#[test]
fn repeated_unclosed_anchors_do_not_trigger_depth_limit() {
    let mut html = String::from("<!doctype html>");
    for index in 0..300 {
        html.push_str(&format!("<a href=\"#{index}\">link"));
    }
    let result = to_markdown_bytes(html.as_bytes(), Some(Format::Html));
    assert!(
        !matches!(result, Err(ConvertError::ResourceLimit { limit: "max_xml_depth", .. })),
        "HTML5 repairs repeated anchors; preflight must not reject them as excessive depth"
    );
}
