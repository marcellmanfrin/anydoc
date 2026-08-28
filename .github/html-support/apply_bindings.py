from pathlib import Path

root = Path('.')

def rep(path, old, new):
    p = root / path
    s = p.read_text()
    if old not in s:
        if new in s:
            return
        raise RuntimeError(f'{path}: missing expected text: {old[:80]!r}')
    p.write_text(s.replace(old, new, 1))

# Node binding + CLI
rep('node/src/lib.rs', '    epub,\n    xlsx,', '    epub,\n    html,\n    xlsx,')
rep('node/src/lib.rs', '            Format::epub => anydoc::Format::Epub,\n            Format::xlsx', '            Format::epub => anydoc::Format::Epub,\n            Format::html => anydoc::Format::Html,\n            Format::xlsx')
rep('node/src/lib.rs', '            anydoc::Format::Epub => Format::epub,\n            anydoc::Format::Excel', '            anydoc::Format::Epub => Format::epub,\n            anydoc::Format::Html => Format::html,\n            anydoc::Format::Excel')
rep('node/cli.js', "const FORMATS = 'doc, docx, odt, pdf, ppt, pptx, rtf, epub, xlsx, ods, odp, csv'", "const FORMATS = 'doc, docx, odt, pdf, ppt, pptx, rtf, epub, html, xlsx, ods, odp, csv'")

# Python binding + handwritten stub
rep('python/src/lib.rs', 'const FORMATS: [(&str, anydoc::Format); 12] = [', 'const FORMATS: [(&str, anydoc::Format); 13] = [')
rep('python/src/lib.rs', '    ("epub", anydoc::Format::Epub),\n    ("xlsx"', '    ("epub", anydoc::Format::Epub),\n    ("html", anydoc::Format::Html),\n    ("xlsx"')
rep('python/anydoc/_anydoc.pyi', '"rtf", "epub", "xlsx"', '"rtf", "epub", "html", "xlsx"')

# WASM binding
rep('wasm/src/lib.rs', '    Epub = "epub",\n    Xlsx', '    Epub = "epub",\n    Html = "html",\n    Xlsx')
rep('wasm/src/lib.rs', '            Format::Epub => anydoc::Format::Epub,\n            Format::Xlsx', '            Format::Epub => anydoc::Format::Epub,\n            Format::Html => anydoc::Format::Html,\n            Format::Xlsx')
rep('wasm/src/lib.rs', '            anydoc::Format::Epub => Format::Epub,\n            anydoc::Format::Excel', '            anydoc::Format::Epub => Format::Epub,\n            anydoc::Format::Html => Format::Html,\n            anydoc::Format::Excel')

# Public format table
rep('README.md', '| EPUB             | `.epub`                                                    |\n| CSV', '| EPUB             | `.epub`                                                    |\n| HTML             | `.html`, `.htm`                                             |\n| CSV')
