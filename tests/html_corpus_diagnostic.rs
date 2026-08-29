mod common;

use common::fixture_root;

#[test]
fn print_selected_outputs() {
    let cases = [
        ("docx-text-source", fixture_root().join("docx/text.docx")),
        ("docx-text-html", fixture_root().join("html/libreoffice-docx-text.html")),
        ("numbering-source", fixture_root().join("docx/handmade-numbering.docx")),
        ("numbering-html", fixture_root().join("html/libreoffice-docx-numbering.html")),
        ("merged-source", fixture_root().join("xlsx/handmade-merged.xlsx")),
        ("merged-html", fixture_root().join("html/libreoffice-xlsx-merged.html")),
    ];
    for (name, path) in cases {
        let markdown = anydoc::to_markdown(path).unwrap();
        println!("MARKDOWN-BEGIN {name}\n{markdown}MARKDOWN-END {name}");
    }
}
