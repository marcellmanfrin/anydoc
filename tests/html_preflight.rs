use anydoc::{Format, to_markdown_bytes};

#[test]
fn repeated_unclosed_anchors_convert_successfully() {
    let mut html = String::from("<!doctype html>");
    // Deliberately above the current MAX_XML_DEPTH (256) so this remains a depth regression.
    for index in 0..300 {
        html.push_str(&format!("<a href=\"#{index}\">link"));
    }
    to_markdown_bytes(html.as_bytes(), Some(Format::Html))
        .expect("HTML5 repairs repeated anchors without excessive nesting");
}
