use anydoc::model::{AssetId, Block, ImageSource, Inline};
use anydoc::{ConvertError, Format, to_document, to_markdown_bytes};

fn mhtml_fixture(source: &str) -> Vec<u8> {
    source.replace('\n', "\r\n").into_bytes()
}

#[test]
fn mhtml_extensions_are_named() {
    assert_eq!(Format::from_extension("mhtml"), Some(Format::Mhtml));
    assert_eq!(Format::from_extension("MHT"), Some(Format::Mhtml));
}

#[test]
fn multipart_related_html_is_detected_as_mhtml() {
    let mhtml = mhtml_fixture(
        r#"From: <Saved by Blink>
Snapshot-Content-Location: https://example.test/page
MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html

<!doctype html><p>Hello</p>
--b--
"#,
    );
    assert_eq!(Format::from_bytes(&mhtml), Some(Format::Mhtml));
}

#[test]
fn generic_multipart_alternative_email_is_not_mhtml() {
    let email = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="b"

--b
Content-Type: text/plain

Hello
--b
Content-Type: text/html

<p>Hello</p>
--b--
"#,
    );
    assert_eq!(Format::from_bytes(&email), None);
}

#[test]
fn quoted_printable_html_root_converts() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html; charset=windows-1252
Content-Transfer-Encoding: quoted-printable
Content-Location: https://example.test/page

<!doctype html><p>caf=E9</p>
--b--
"#,
    );
    assert_eq!(to_markdown_bytes(&mhtml, None).unwrap(), "café\n");
}

#[test]
fn start_parameter_selects_the_related_html_root() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; start="<root@id>"; boundary="b"

--b
Content-Type: text/html
Content-ID: <other@id>

<p>wrong root</p>
--b
Content-Type: text/html
Content-ID: <root@id>

<p>right root</p>
--b--
"#,
    );
    assert_eq!(to_markdown_bytes(&mhtml, None).unwrap(), "right root\n");
}

#[test]
fn related_mhtml_without_html_root_is_malformed() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/css

p { font-weight: bold }
--b--
"#,
    );
    let error = to_markdown_bytes(&mhtml, Some(Format::Mhtml)).unwrap_err();
    assert!(matches!(error, ConvertError::Malformed { .. }));
}

#[test]
fn linked_css_is_resolved_from_cid() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html

<!doctype html><link rel="stylesheet" href="cid:style@id"><p class="hidden">drop me</p><p class="strong">keep me</p>
--b
Content-Type: text/css; charset=utf-8
Content-ID: <style@id>

.hidden { display: none }
.strong { font-weight: bold }
--b--
"#,
    );
    assert_eq!(to_markdown_bytes(&mhtml, None).unwrap(), "**keep me**\n");
}

#[test]
fn cid_image_becomes_an_embedded_document_asset() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html

<!doctype html><p><img alt="pixel" src="cid:image@id"></p>
--b
Content-Type: image/png
Content-ID: <image@id>
Content-Transfer-Encoding: base64

AAECAw==
--b--
"#,
    );
    let document = to_document(&mhtml, None).unwrap();
    assert_eq!(document.assets.len(), 1);
    assert_eq!(document.assets[0].media_type, "image/png");
    assert_eq!(document.assets[0].bytes, [0, 1, 2, 3]);
    match &document.blocks[0] {
        Block::Paragraph(inlines) => match &inlines[0] {
            Inline::Image { alt, source: ImageSource::Asset(AssetId(0)) } => {
                assert_eq!(alt, "pixel");
            }
            other => panic!("expected embedded image, got {other:?}"),
        },
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn content_location_image_becomes_an_embedded_document_asset() {
    let mhtml = mhtml_fixture(
        r#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html

<!doctype html><p><img alt="pixel" src="https://example.test/pixel.png"></p>
--b
Content-Type: image/png
Content-Location: https://example.test/pixel.png
Content-Transfer-Encoding: base64

AAECAw==
--b--
"#,
    );
    let document = to_document(&mhtml, None).unwrap();
    assert_eq!(document.assets.len(), 1);
    match &document.blocks[0] {
        Block::Paragraph(inlines) => assert!(matches!(
            &inlines[0],
            Inline::Image { source: ImageSource::Asset(AssetId(0)), .. }
        )),
        other => panic!("expected paragraph, got {other:?}"),
    }
}
