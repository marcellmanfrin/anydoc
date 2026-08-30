use anydoc::model::{Block, ImageSource, Inline};
use anydoc::{Format, to_document, to_markdown_bytes};

#[test]
fn frameset_html_converts_without_body_error() {
    let html = br#"<!doctype html><html><head><title>frames</title></head><frameset cols="*"><frame src="page.html"></frameset></html>"#;
    assert_eq!(to_markdown_bytes(html, Some(Format::Html)).unwrap(), "");
}

#[test]
fn relative_image_is_preserved_without_fetching() {
    let html = br#"<!doctype html><p><img src="images/pixel.png"></p>"#;
    let document = to_document(html, Some(Format::Html)).unwrap();
    match &document.blocks[0] {
        Block::Paragraph(inlines) => assert!(matches!(
            &inlines[0],
            Inline::Image { source: ImageSource::External(url), .. }
                if url == "images/pixel.png"
        )),
        other => panic!("expected paragraph, got {other:?}"),
    }
}
