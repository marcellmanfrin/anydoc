use anydoc::{ConvertError, Format, to_markdown_bytes};

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

#[test]
fn quoted_printable_html_root_converts() {
    let mhtml = br#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/html; charset=windows-1252
Content-Transfer-Encoding: quoted-printable
Content-Location: https://example.test/page

<!doctype html><p>caf=E9</p>
--b--
"#;
    assert_eq!(to_markdown_bytes(mhtml, None).unwrap(), "café\n");
}

#[test]
fn start_parameter_selects_the_related_html_root() {
    let mhtml = br#"MIME-Version: 1.0
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
"#;
    assert_eq!(to_markdown_bytes(mhtml, None).unwrap(), "right root\n");
}

#[test]
fn related_mhtml_without_html_root_is_malformed() {
    let mhtml = br#"MIME-Version: 1.0
Content-Type: multipart/related; type="text/html"; boundary="b"

--b
Content-Type: text/css

p { font-weight: bold }
--b--
"#;
    let error = to_markdown_bytes(mhtml, Some(Format::Mhtml)).unwrap_err();
    assert!(matches!(error, ConvertError::Malformed { .. }));
}
