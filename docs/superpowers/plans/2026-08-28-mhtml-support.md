# MHTML/MHT Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert raw MHTML/MHT input directly to anydoc's document model/Markdown, resolving embedded CSS and image resources offline.

**Architecture:** Add `Format::Mhtml` and a MIME container frontend backed by `mail-parser` 0.11.8. Refactor the HTML frontend only enough to accept a caller-supplied semantic context, additional stylesheets, and prebuilt assets; standalone HTML behavior remains unchanged.

**Tech Stack:** Rust 1.88, `mail-parser` 0.11.8, existing `scraper`/`html5ever`, existing `Document`/`HtmlCtx` model, GitHub Actions for build/test execution.

**Spec:** `docs/superpowers/specs/2026-08-28-mhtml-support-design.md`

## Global Constraints

- MHTML conversion is fully offline: no HTTP/resource fetching and no JavaScript execution.
- `.mhtml` and `.mht` are exposed in Rust, Node, Python, WASM, and CLI format enums/lists.
- Generic email with HTML content must not be detected as MHTML; byte detection requires a top-level `multipart/related` HTML aggregate.
- Existing `MAX_TOTAL_BYTES`, `MAX_ENTRY_BYTES`, and `MAX_ASSET_TOTAL_BYTES` are enforced.
- Standalone HTML output must remain unchanged.
- The user-supplied SEI MHTML is a mandatory final integration fixture/test input, but its private contents are not committed to the public repository.

---

### Task 1: Format identity and detection

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/formats/detect.rs`
- Test: `tests/mhtml.rs`

**Interfaces:**
- Produces: `Format::Mhtml`, `Format::from_extension("mhtml"|"mht")`, `Format::from_bytes(raw_mhtml) == Some(Format::Mhtml)`.

- [ ] Write failing tests for `.mhtml`/`.mht`, Chrome/Blink `multipart/related; type="text/html"`, and a generic `multipart/alternative` email that must remain `None`.
- [ ] Run `cargo test --test mhtml` and verify RED because `Format::Mhtml` does not exist.
- [ ] Add the enum/extension mapping and conservative MIME header detection.
- [ ] Run `cargo test --test mhtml` and verify the detection tests pass.

### Task 2: MIME container parsing and HTML delegation

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Create: `src/formats/mhtml.rs`
- Modify: `src/formats/mod.rs`
- Modify: `src/formats/html.rs`
- Test: `tests/mhtml.rs`

**Interfaces:**
- `mhtml::parse(bytes: &[u8]) -> Result<Document, ConvertError>`.
- Internal HTML entry point accepts HTML UTF-8 bytes/text, an `HtmlCtx`, additional CSS strings, and initial assets.

- [ ] Add failing tests for quoted-printable root HTML conversion and malformed MHTML without any HTML root.
- [ ] Run the focused tests and verify RED with unsupported/unimplemented parsing.
- [ ] Add `mail-parser = { version = "0.11.8", features = ["full_encoding"] }` and implement MIME parsing with `MessageParser::new().with_mime_headers()`.
- [ ] Select the `start=` Content-ID HTML root when present; otherwise use the first HTML body part.
- [ ] Refactor `html.rs` so standalone `parse()` delegates to a reusable internal conversion entry point without changing standalone behavior.
- [ ] Run focused tests and verify GREEN.

### Task 3: Embedded CSS and images

**Files:**
- Modify: `src/formats/mhtml.rs`
- Modify: `src/formats/html.rs`
- Test: `tests/mhtml.rs`

**Interfaces:**
- Resource index keys: normalized `cid:<id>`, bare Content-ID, and exact Content-Location.
- Resolved image MIME parts become `ImageSource::Asset(AssetId)` and `Document::assets` entries.
- Resolved linked `text/css` parts are fed to the existing `Stylesheet` before block conversion.

- [ ] Add failing test where CSS referenced by `cid:` sets `display:none` and bold semantics.
- [ ] Add failing tests for an image referenced by `cid:` and by `Content-Location`, asserting `Document::assets` payload/media type and `ImageSource::Asset`.
- [ ] Run focused tests and verify RED.
- [ ] Implement resource indexing, asset size accounting, CSS collection, and `MhtmlCtx` resolution.
- [ ] Run focused tests and verify GREEN.

### Task 4: Public bindings and regression checks

**Files:**
- Modify: Node/Python/WASM format mappings and type stubs where `Format::Html` was previously added.
- Modify: CLI accepted-format list.
- Modify: README supported-format table only if final PR includes documentation.
- Test: Node/Python binding tests plus Rust HTML/MHTML suites.

- [ ] Add failing binding tests that explicitly name/detect `mhtml`.
- [ ] Update each binding enum/mapping/list.
- [ ] Run `cargo test --locked`, `cargo fmt --all -- --check`, and workspace clippy with `-D warnings`.
- [ ] Build/test Node, Python wheel/unittests, and WASM exactly as the existing CI does.
- [ ] Re-run `cargo test --test html` to prove standalone HTML behavior remains green.

### Task 5: Real SEI MHTML validation

**Files:**
- No public fixture commit; use the conversation upload locally.

**Interfaces:**
- Raw 505,883-byte MHTML is supplied directly to the Python binding built from `feature/mhtml-support`.

- [ ] Build a Python wheel from the exact validated branch commit and download it as a GitHub Actions artifact.
- [ ] Install that wheel in an isolated local environment.
- [ ] Run `anydoc.format_from_bytes(raw_mhtml)` and assert `mhtml`.
- [ ] Run `anydoc.to_markdown_bytes(raw_mhtml)` directly, save the Markdown, and record byte count/SHA-256.
- [ ] Compare meaningful document text/structure with the previously validated extracted-HTML baseline; explain any intentional difference caused by resolving MHTML resources.
- [ ] Only after all checks pass, clean temporary workflows/process docs if they are not appropriate for the contribution branch.