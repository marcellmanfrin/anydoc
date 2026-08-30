use anydoc::{Format, to_markdown_bytes};

fn flat_mhtml_with_boundary_parameter(levels: usize) -> Vec<u8> {
    let mut out = String::from(
        "Snapshot-Content-Location: https://example.test/page\r\nMIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"root\"\r\n\r\n--root\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>ok</p>\r\n--root\r\nContent-Type: text/plain; boundary=\"fake0\"\r\n\r\n",
    );
    for level in 0..levels {
        out.push_str(&format!(
            "--fake{level}\r\nContent-Type: text/plain; boundary=\"fake{}\"\r\n\r\n",
            level + 1
        ));
    }
    out.push_str("ordinary text\r\n--root--\r\n");
    out.into_bytes()
}

#[test]
fn non_multipart_boundary_parameter_does_not_count_as_mime_nesting() {
    let input = flat_mhtml_with_boundary_parameter(70);
    assert_eq!(to_markdown_bytes(&input, Some(Format::Mhtml)).unwrap(), "ok\n");
}
