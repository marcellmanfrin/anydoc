# MHTML/MHT Support Design

## Goal

Add offline MHTML/MHT conversion as a container frontend layered on top of the standalone HTML frontend. Raw `.mhtml`/`.mht` bytes must convert directly to the same semantic `Document`/Markdown path used by HTML, while embedded resources referenced through `Content-ID` or `Content-Location` remain self-contained where the document model can represent them.

## Architecture

`Format::Mhtml` identifies `.mhtml`/`.mht` and MIME `multipart/related` input whose related root is HTML. `src/formats/mhtml.rs` uses `mail-parser` 0.11.8 with MIME headers enabled to decode multipart structure, transfer encoding, and character sets. It selects the related HTML root (honoring the top-level `start=` Content-ID when present, otherwise the first HTML body part), builds a resource index from all MIME parts, and delegates semantic conversion to the existing HTML frontend.

The HTML frontend gains an internal entry point that accepts an `HtmlCtx`, preloaded stylesheet text, and prebuilt document assets. Standalone HTML continues to call that entry point with an empty resource set and its existing `StandaloneCtx`; no behavior change is intended for `Format::Html`.

## Resource resolution

MHTML resources are indexed by normalized `Content-ID` (`cid:` form and bare ID) and `Content-Location`. `<link rel=stylesheet href=...>` references that resolve to `text/css` MIME parts are decoded and appended to the existing `Stylesheet` before `shared::html::to_blocks()`. Image `src` values resolving to MIME image parts become `ImageSource::Asset(AssetId)` and retain exact decoded payload bytes plus MIME type and origin. Absolute resources not present in the archive remain external URLs; unresolved relative/cid resources become unavailable rather than triggering network access.

No remote HTTP fetches, JavaScript execution, browser layout, or filesystem-relative resource loading are added.

## Detection

Extension mapping recognizes `.mhtml` and `.mht`. Content detection recognizes a MIME header block whose top-level `Content-Type` is `multipart/related` with `type=text/html`, plus Chrome/Blink MHTML (`Snapshot-Content-Location` and multipart/related) when present. Generic email messages that merely contain an HTML alternative must not be classified as MHTML.

## Limits and errors

Raw MHTML input is capped by the existing `MAX_TOTAL_BYTES`; each decoded part is capped by `MAX_ENTRY_BYTES`; retained embedded assets are capped cumulatively by `MAX_ASSET_TOTAL_BYTES`. Invalid MIME or a related document with no HTML root returns `Malformed`. Resource decoding failures do not crash conversion; unresolved optional resources are skipped/unavailable.

## Testing

TDD coverage includes: extension/content detection; quoted-printable HTML root; base64/quoted-printable resource decoding; `start=` root selection; CSS resolution by CID; image resolution by CID and Content-Location into `Document::assets`; no-network behavior for unresolved URLs; malformed MHTML without HTML; and regression that standalone HTML output is unchanged. Final validation must run the actual user-supplied SEI MHTML directly through the corrected Python binding and compare its rendered Markdown with the known extracted-HTML baseline where resource semantics do not intentionally differ.