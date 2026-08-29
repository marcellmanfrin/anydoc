use anydoc::Format;

#[test]
fn strong_rtf_signature_wins_over_mhtml_header_heuristic() {
    let rtf = b"{\\rtf1\\ansi\nSnapshot-Content-Location: https://example.test/page\nContent-Type: multipart/related; boundary=\"b\"\n\nplain text}";

    assert_eq!(Format::from_bytes(rtf), Some(Format::Rtf));
}
