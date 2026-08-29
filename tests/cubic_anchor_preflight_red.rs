use anydoc::{ConvertError, Format, to_markdown_bytes};

#[test]
fn intervening_blocks_between_unclosed_anchors_still_hit_preflight_depth_limit() {
    let mut html = String::from("<!doctype html>");
    for index in 0..300 {
        html.push_str(&format!("<a href=\"#{index}\"><div>"));
    }

    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
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
