use anydoc::{Format, to_markdown_bytes};

#[test]
fn non_multipart_boundary_parameter_does_not_count_as_mime_nesting() {
    let mut mhtml = String::from(
        "MIME-Version: 1.0\r\nSnapshot-Content-Location: https://example.test/page\r\nContent-Type: multipart/related; boundary=\"outer\"\r\n\r\n--outer\r\nContent-Type: text/plain; boundary=\"fake0\"\r\n\r\n",
    );
    for depth in 0..70 {
        mhtml.push_str(&format!(
            "--fake{depth}\r\nContent-Type: text/plain; boundary=\"fake{}\"\r\n\r\n",
            depth + 1
        ));
    }
    mhtml.push_str(
        "payload\r\n--outer\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<!doctype html><p>ok</p>\r\n--outer--\r\n",
    );

    assert_eq!(
        to_markdown_bytes(mhtml.as_bytes(), Some(Format::Mhtml)).unwrap(),
        "ok\n"
    );
}
