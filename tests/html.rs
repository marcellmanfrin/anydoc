use anydoc::model::{Block, ImageSource, Inline};
use anydoc::{ConvertError, Format, to_document, to_markdown_bytes};

#[test]
fn html_extensions_are_named() {
    assert_eq!(Format::from_extension("html"), Some(Format::Html));
    assert_eq!(Format::from_extension("HTM"), Some(Format::Html));
}

#[test]
fn html_doctype_is_detected_from_content() {
    let html = b"\xEF\xBB\xBF  <!DOCTYPE html><html><body><p>Hello</p></body></html>";
    assert_eq!(Format::from_bytes(html), Some(Format::Html));
}

#[test]
fn html_doctype_allows_html5_ascii_whitespace() {
    assert_eq!(Format::from_bytes(b"<!DOCTYPE\thtml><html></html>"), Some(Format::Html));
    assert_eq!(Format::from_bytes(b"<!DOCTYPE\nhtml><html></html>"), Some(Format::Html));
}

#[test]
fn html_prefix_wins_over_embedded_pdf_marker() {
    let html = b"<!doctype html><html><body>%PDF-1.7 is text here</body></html>";
    assert_eq!(Format::from_bytes(html), Some(Format::Html));
}

#[test]
fn pdf_header_before_html_marker_remains_pdf() {
    let bytes = b"  %PDF-1.7\n<html><body>not an HTML root</body></html>";
    assert_eq!(Format::from_bytes(bytes), Some(Format::Pdf));
}

#[test]
fn utf16_html_is_detected_from_content() {
    let source = "<html><body>hello</body></html>";

    let mut le = vec![0xFF, 0xFE];
    for unit in source.encode_utf16() {
        le.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(Format::from_bytes(&le), Some(Format::Html));

    let mut be = vec![0xFE, 0xFF];
    for unit in source.encode_utf16() {
        be.extend_from_slice(&unit.to_be_bytes());
    }
    assert_eq!(Format::from_bytes(&be), Some(Format::Html));
}

#[test]
fn utf16_html_detection_allows_long_leading_whitespace() {
    let source = format!("{}<html><body>hello</body></html>", " ".repeat(300));

    let mut le = vec![0xFF, 0xFE];
    for unit in source.encode_utf16() {
        le.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(Format::from_bytes(&le), Some(Format::Html));

    let mut be = vec![0xFE, 0xFF];
    for unit in source.encode_utf16() {
        be.extend_from_slice(&unit.to_be_bytes());
    }
    assert_eq!(Format::from_bytes(&be), Some(Format::Html));
}

#[test]
fn utf16le_doctype_allows_long_whitespace_between_keyword_and_name() {
    let source = format!("<!DOCTYPE{}html><html></html>", " ".repeat(80));
    let mut bytes = vec![0xFF, 0xFE];
    for unit in source.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(Format::from_bytes(&bytes), Some(Format::Html));
}

#[test]
fn utf16be_doctype_allows_long_whitespace_between_keyword_and_name() {
    let source = format!("<!DOCTYPE{}html><html></html>", " ".repeat(80));
    let mut bytes = vec![0xFE, 0xFF];
    for unit in source.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    assert_eq!(Format::from_bytes(&bytes), Some(Format::Html));
}

#[test]
fn unrelated_charset_attribute_does_not_change_decoding() {
    let html = "<!doctype html><p data-note='charset=windows-1252'>café</p>".as_bytes();
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "café\n");
}

#[test]
fn meta_looking_text_in_comment_does_not_change_decoding() {
    let html = "<!doctype html><!-- <meta charset=windows-1252> --><p>café</p>".as_bytes();
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "café\n");
}

#[test]
fn meta_looking_text_in_script_does_not_change_decoding() {
    let html =
        "<!doctype html><script>const fake = '<meta charset=windows-1252>';</script><p>café</p>"
            .as_bytes();
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "café\n");
}

#[test]
fn html_node_limit_covers_nodes_outside_body_before_dom_materialization() {
    let mut html = String::from("<!doctype html><html><head>");
    for _ in 0..2_000_001 {
        html.push_str("<!---->");
    }
    html.push_str("</head><body>ok</body></html>");

    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert!(matches!(error, ConvertError::ResourceLimit { limit: "max_xml_nodes", .. }));
}

#[test]
fn malformed_html5_is_repaired_before_conversion() {
    let html = br#"<!doctype html><html><body><h1>Hello</h1><p>first<p><strong>second</strong></body></html>"#;
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "# Hello\n\nfirst\n\n**second**\n");
}

#[test]
fn html5_table_inserts_implicit_structure() {
    let html = br#"<!doctype html><table><tr><th>A<th>B<tr><td>1<td>2</table>"#;
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "| A | B |\n| --- | --- |\n| 1 | 2 |\n");
}

#[test]
fn style_blocks_feed_the_existing_semantic_css_subset() {
    let html = br#"<!doctype html><style>
        .hidden { display: none }
        .strong { font-weight: bold }
    </style><p class=hidden>drop me</p><p class=strong>keep me</p>"#;
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "**keep me**\n");
}

#[test]
fn meta_charset_decodes_legacy_html() {
    let mut html = b"<!doctype html><meta charset=windows-1252><p>caf".to_vec();
    html.push(0xE9);
    html.extend_from_slice(b"</p>");
    let markdown = to_markdown_bytes(&html, None).unwrap();
    assert_eq!(markdown, "caf\u{e9}\n");
}

#[test]
fn scripts_are_not_document_content() {
    let html = br#"<!doctype html><p>before</p><script>document.write('not content')</script><p>after</p>"#;
    let markdown = to_markdown_bytes(html, None).unwrap();
    assert_eq!(markdown, "before\n\nafter\n");
}

#[test]
fn quoted_mime_parameter_semicolon_does_not_fake_charset() {
    let mut html = br#"<!doctype html><meta http-equiv="content-type" content='text/html; note="x;charset=utf-8"; charset=windows-1252'><p>"#.to_vec();
    html.push(0x80);
    html.extend_from_slice(b"</p>");
    let markdown = to_markdown_bytes(&html, Some(Format::Html)).unwrap();
    assert_eq!(markdown, "€\n");
}

#[test]
fn optional_li_end_tags_do_not_count_as_nested_depth() {
    let mut html = String::from("<!doctype html><ul>");
    for i in 0..300 {
        html.push_str(&format!("<li>item {i}"));
    }
    html.push_str("</ul>");
    let markdown = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap();
    assert!(markdown.contains("item 299"));
}

#[test]
fn optional_p_end_tags_do_not_count_as_nested_depth() {
    let mut html = String::from("<!doctype html>");
    for i in 0..300 {
        html.push_str(&format!("<p>paragraph {i}"));
    }
    let markdown = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap();
    assert!(markdown.contains("paragraph 299"));
}

#[test]
fn meta_charset_after_first_kib_is_still_honored() {
    let mut html = b"<!doctype html><html><head><style>/*".to_vec();
    html.extend(std::iter::repeat_n(b'x', 2048));
    html.extend_from_slice(b"*/</style><meta charset=iso-8859-2></head><body><p>");
    html.push(0xA3);
    html.extend_from_slice(b"</p></body></html>");
    let markdown = to_markdown_bytes(&html, Some(Format::Html)).unwrap();
    assert_eq!(markdown, "Ł\n");
}

#[test]
fn protocol_relative_image_is_preserved_as_external() {
    let html = br#"<!doctype html><p><img alt="pixel" src="//cdn.example.test/image.png"></p>"#;
    let document = to_document(html, Some(Format::Html)).unwrap();
    match &document.blocks[0] {
        Block::Paragraph(inlines) => assert!(matches!(
            &inlines[0],
            Inline::Image { source: ImageSource::External(url), .. }
                if url == "//cdn.example.test/image.png"
        )),
        other => panic!("expected paragraph, got {other:?}"),
    }
}

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
fn alternating_headings_are_implicitly_closed_before_preflight_depth_counting() {
    const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
    let mut html = String::from("<!doctype html>");
    for i in 0..300 {
        let heading = HEADINGS[i % HEADINGS.len()];
        html.push_str(&format!("<{heading}>heading {i}"));
    }

    let markdown = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap();
    assert!(markdown.contains("heading 299"));
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

#[test]
fn nested_list_wrappers_preserve_structural_children() {
    let html = br#"<!doctype html>
        <ol><ol>
          <li><h2>Nested heading</h2></li>
          <table><tr><td>A</td></tr></table>
        </ol></ol>"#;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert!(markdown.contains("Nested heading"), "{markdown:?}");
    assert!(markdown.contains("| A |"), "{markdown:?}");
}

#[test]
fn repeated_unclosed_anchors_do_not_trigger_depth_limit() {
    let mut html = String::from("<!doctype html>");
    for index in 0..300 {
        html.push_str(&format!("<a href=\"#{index}\">link"));
    }
    let result = to_markdown_bytes(html.as_bytes(), Some(Format::Html));
    assert!(
        result.is_ok(),
        "HTML5 repairs repeated anchors; preflight must not reject them: {result:?}"
    );
}

#[test]
fn intervening_blocks_between_unclosed_anchors_still_hit_preflight_depth_limit() {
    let mut html = String::from("<!doctype html>");
    for index in 0..300 {
        html.push_str(&format!("<a href=\"#{index}\"><div>"));
    }

    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

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

#[test]
fn paragraph_div_pairs_do_not_accumulate_paragraph_depth() {
    // html5ever implicitly closes each <p> when the <div> arrives, so the
    // real DOM nests 200 divs - well under max_xml_depth.
    let mut html = String::from("<!doctype html><body>");
    for _ in 0..200 {
        html.push_str("<p>text<div>");
    }
    html.push_str("</body>");
    assert!(to_markdown_bytes(html.as_bytes(), Some(Format::Html)).is_ok());
}

#[test]
fn deeply_nested_paragraph_div_pairs_are_rejected_before_dom() {
    let mut html = String::from("<!doctype html><body>");
    for _ in 0..300 {
        html.push_str("<p>text<div>");
    }
    html.push_str("</body>");
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert!(matches!(error, ConvertError::ResourceLimit { limit: "max_xml_depth", .. }));
}

#[test]
fn bare_hash_link_is_preserved_as_relative_url() {
    let html = br##"<!doctype html><p><a href="#">top</a></p>"##;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert_eq!(markdown, "[top](#)\n");
}

#[test]
fn foreign_content_breakout_does_not_bypass_depth_limit() {
    // <p> breaks out of <svg>; the HTML <path/> tags that follow must not
    // be honored as foreign self-closing, otherwise the depth accounting
    // never runs and the DOM is constructed before any limit fires.
    let mut html = String::from("<!doctype html><body><svg><p>");
    for _ in 0..300 {
        html.push_str("<path/>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn headings_with_open_inlines_are_rejected_before_dom_construction() {
    // html5ever pops the current node only when it is already a heading, so
    // unclosed inline elements between headings really do nest in the DOM.
    // The preflight mirrors the parser and must reject the shape before DOM
    // construction.
    let mut html = String::from("<!doctype html><body>");
    for i in 0..300 {
        html.push_str(&format!("<h{}><span>t", (i % 6) + 1));
    }
    html.push_str("</body>");
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn optgroup_start_closes_previous_option_and_group() {
    let mut html = String::from("<!doctype html><body><select>");
    for _ in 0..300 {
        html.push_str("<optgroup><option>a");
    }
    html.push_str("</select></body>");
    assert!(to_markdown_bytes(html.as_bytes(), Some(Format::Html)).is_ok());
}

#[test]
fn row_group_start_closes_previous_row_group() {
    let mut html = String::from("<!doctype html><body><table>");
    for _ in 0..300 {
        html.push_str("<tbody><tr><td>x");
    }
    html.push_str("</table></body>");
    assert!(to_markdown_bytes(html.as_bytes(), Some(Format::Html)).is_ok());
}

#[test]
fn text_directly_under_list_is_preserved() {
    let html = br##"<!doctype html><ul>stray bullet<li>b item</ul><ol start="2">stray ordered<li>o item</ol>"##;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert!(markdown.contains("stray bullet"), "got: {markdown:?}");
    assert!(markdown.contains("stray ordered"), "got: {markdown:?}");
    assert!(markdown.contains("b item"), "got: {markdown:?}");
    assert!(markdown.contains("o item"), "got: {markdown:?}");
}

#[test]
fn foreign_content_title_does_not_swallow_nested_markup() {
    // html5ever parses an svg <title>'s children as markup (svg title is an
    // HTML integration point), so the preflight must count them and reject
    // pathological nesting before DOM construction instead of switching to
    // the HTML RCDATA state.
    let mut html = String::from("<!doctype html><body><svg><title>");
    for _ in 0..300 {
        html.push_str("<div>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn html_title_still_swallows_markup_as_raw_text() {
    // The HTML (non-foreign) <title> keeps RCDATA semantics: nested markup is
    // text, not elements, and the document converts normally.
    let html = br#"<!doctype html><html><head><title><div>not markup</div></title></head><body><p>ok</p></body></html>"#;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert!(markdown.contains("ok"), "got: {markdown:?}");
}

#[test]
fn foreign_content_script_does_not_swallow_nested_markup() {
    // html5ever does not switch the tokenizer to ScriptData for scripts
    // inside foreign content, so nested markup inside <svg><script> really
    // becomes elements and the preflight must count it.
    let mut html = String::from("<!doctype html><body><svg><script>");
    for _ in 0..300 {
        html.push_str("<div>");
    }
    html.push_str("</script></svg></body>");
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn html_script_still_swallows_markup_as_script_data() {
    let html = br#"<!doctype html><html><head><script>var x = "<div>";</script></head><body><p>ok</p></body></html>"#;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert!(markdown.contains("ok"), "got: {markdown:?}");
}

#[test]
fn foreign_content_option_tags_do_not_get_html_implied_closes() {
    // Inside <svg>, repeated <option> elements are ordinary foreign elements
    // that genuinely nest; the preflight must not apply the HTML select
    // insertion rule there.
    let mut html = String::from("<!doctype html><body><svg>");
    for _ in 0..300 {
        html.push_str("<option>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn nested_foreign_roots_fully_exit_on_breakout() {
    // html5ever pops every foreign root on a breakout tag; leaving the outer
    // svg on the modeled stack would treat later <path/> tags as foreign
    // self-closing and bypass the depth guard.
    let mut html = String::from("<!doctype html><body><svg><math><p>");
    for _ in 0..300 {
        html.push_str("<path/>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn template_body_does_not_capture_document_conversion() {
    let html = br#"<!doctype html><html><head><template><body>template body</body></template></head><body><p>real body</p></body></html>"#;
    let markdown = to_markdown_bytes(html, Some(Format::Html)).unwrap();
    assert_eq!(markdown, "real body\n");
}

#[test]
fn depth_limit_aligns_with_the_post_parse_walk_at_the_boundary() {
    // 255 nested divs under an explicit body sit exactly at the limit the
    // DOM walk accepts (body = depth 1, innermost div = depth 256).
    let mut ok = String::from("<!doctype html><html><body>");
    for _ in 0..255 {
        ok.push_str("<div>");
    }
    ok.push_str("x</body></html>");
    assert!(to_markdown_bytes(ok.as_bytes(), Some(Format::Html)).is_ok());

    // One more level exceeds it and must be rejected before DOM construction.
    let mut too_deep = String::from("<!doctype html><html><body>");
    for _ in 0..256 {
        too_deep.push_str("<div>");
    }
    too_deep.push_str("x</body></html>");
    let error = to_markdown_bytes(too_deep.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn void_hr_closes_open_paragraph_in_preflight() {
    let mut html = String::from("<!doctype html><body>");
    for _ in 0..300 {
        html.push_str("<p>x<hr>");
    }
    html.push_str("</body>");
    assert!(to_markdown_bytes(html.as_bytes(), Some(Format::Html)).is_ok());
}

#[test]
fn foreign_void_elements_are_pushed_and_counted() {
    // Void HTML names that are not foreign-content breakout tags (input,
    // param, source, ...) are ordinary foreign elements inside <svg> that
    // html5ever pushes; 300 of them nest and must be rejected before DOM
    // construction.
    let mut html = String::from("<!doctype html><body><svg>");
    for _ in 0..300 {
        html.push_str("<input>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn out_of_scope_end_tag_does_not_pop_real_nesting() {
    // html5ever ignores </div> here (a td scope marker sits above the div),
    // so the spans keep nesting; the preflight must not truncate the stack
    // on the ignored end tag.
    let mut html = String::from("<!doctype html><body><div><table><td>");
    for _ in 0..200 {
        html.push_str("<span>");
    }
    html.push_str("</div>");
    for _ in 0..100 {
        html.push_str("<span>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn duplicate_body_tag_does_not_move_the_depth_baseline() {
    // html5ever ignores the second <body>; if the preflight pushed it, the
    // body-relative depth baseline would move up and undercount the divs.
    let mut html = String::from("<!doctype html><body>");
    for _ in 0..130 {
        html.push_str("<div>");
    }
    html.push_str("<body>");
    for _ in 0..130 {
        html.push_str("<div>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn paragraph_end_tag_respects_button_scope() {
    // A <button> above the open <p> puts it out of button scope, so
    // html5ever leaves the stack alone for </p>; truncating there would pop
    // the button and the 150 nested divs and undercount the later divs.
    let mut html = String::from("<!doctype html><body><p><button>");
    for _ in 0..150 {
        html.push_str("<div>");
    }
    html.push_str("</p>");
    for _ in 0..150 {
        html.push_str("<div>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}

#[test]
fn table_end_tag_respects_template_scope() {
    // template is a table-scope marker: html5ever ignores </table> while the
    // template is above it, so the spans keep nesting.
    let mut html = String::from("<!doctype html><body><table><template>");
    for _ in 0..100 {
        html.push_str("<span>");
    }
    html.push_str("</table>");
    for _ in 0..160 {
        html.push_str("<span>");
    }
    let error = to_markdown_bytes(html.as_bytes(), Some(Format::Html)).unwrap_err();
    assert_preflight_depth_limit(error);
}
