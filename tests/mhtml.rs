use anydoc::Format;

#[test]
fn mhtml_extensions_are_named() {
    assert_eq!(Format::from_extension("mhtml"), Some(Format::Mhtml));
    assert_eq!(Format::from_extension("MHT"), Some(Format::Mhtml));
}

#[test]
fn multipart_related_html_is_detected_as_mhtml() {
    let mhtml = br#"From: <Saved by Blink>
Snapshot-Content-Location: https://example.test/page
MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html

<!doctype html><p>Hello</p>
--b--
"#;
    assert_eq!(Format::from_bytes(mhtml), Some(Format::Mhtml));
}

#[test]
fn generic_multipart_alternative_email_is_not_mhtml() {
    let email = br#"MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="b"

--b
Content-Type: text/plain

Hello
--b
Content-Type: text/html

<p>Hello</p>
--b--
"#;
    assert_eq!(Format::from_bytes(email), None);
}
