use anydoc::{ConvertError, Format, to_markdown_bytes};

fn assert_preflight_depth_limit(error: ConvertError) {
    match error {
        ConvertError::ResourceLimit { limit, detail } => {
            assert_eq!(limit, "max_xml_depth");
            assert!(
                detail.contains("before DOM construction"),
                "expected preflight depth rejection, got: {detail}"
            );
        }
        other => panic!("expected max_xml_depth resource limit, got {other:?}"),
    }
}

#[test]
fn non_void_self_closing_html_tags_still_count_toward_preflight_depth() {
    let mut html = String::from("<!doctype html>");
    for _ in 0..300 {
        html.push_str("<div/>");
    }

    assert_preflight_depth_limit(
        to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err(),
    );
}

#[test]
fn successive_headings_are_implicitly_closed_before_preflight_depth_counting() {
    let mut html = String::from("<!doctype html>");
    for i in 0..300 {
        html.push_str(&format!("<h1>heading {i}"));
    }

    let markdown = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap();
    assert!(markdown.contains("# heading 299"));
}

#[test]
fn foreign_self_closing_svg_elements_do_not_accumulate_html_depth() {
    let mut html = String::from("<!doctype html><svg>");
    for _ in 0..300 {
        html.push_str("<path/>");
    }
    html.push_str("</svg><p>ok</p>");

    let markdown = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap();
    assert!(markdown.contains("ok"));
}

#[test]
fn html_inside_svg_foreign_object_still_counts_self_closing_non_void_depth() {
    let mut html = String::from("<!doctype html><svg><foreignObject>");
    for _ in 0..300 {
        html.push_str("<div/>");
    }
    html.push_str("</foreignObject></svg>");

    assert_preflight_depth_limit(
        to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err(),
    );
}
