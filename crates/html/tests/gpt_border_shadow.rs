//! `gpt-thin-border-wide-shadow` in the file scan: the hairline-and-halo pair
//! is reported only where a row of sibling cards repeats it.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

fn snippets(html: &str) -> Vec<String> {
    detect_html_source(
        html,
        Path::new("/tmp/gpt-border-shadow.html"),
        &DetectHtmlOptions::default(),
    )
    .into_iter()
    .filter(|f| f.antipattern == "gpt-thin-border-wide-shadow")
    .map(|f| f.snippet)
    .collect()
}

/// `n` sibling cards, each with a hairline on every side and `shadow`.
fn row(n: usize, shadow: &str) -> String {
    let card = format!(
        "<div class=\"card\" style=\"width:180px;height:140px;border:1px solid #e5e7eb;box-shadow:{shadow}\">Card</div>"
    );
    format!(
        "<!DOCTYPE html><html><body><div class=\"row\">{}</div></body></html>",
        card.repeat(n)
    )
}

fn repo_root() -> std::path::PathBuf {
    std::env::var("IMPECCABLE_PUBLIC_REPO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

#[test]
fn a_row_of_three_flags_and_a_pair_does_not() {
    let halo = "0 0 40px rgba(15,23,42,0.18)";
    assert!(snippets(&row(1, halo)).is_empty());
    assert!(snippets(&row(2, halo)).is_empty());
    assert_eq!(
        snippets(&row(3, halo)),
        vec![
            "1px border + 40px shadow blur",
            "1px border + 40px shadow blur",
            "1px border + 40px shadow blur"
        ]
    );
}

#[test]
fn offset_tight_and_inset_shadows_stay_silent() {
    for shadow in [
        "0 8px 40px rgba(15,23,42,0.22)",
        "0 30px 60px -40px rgba(15,23,42,0.35)",
        "0 0 24px rgba(15,23,42,0.18)",
        "inset 0 0 40px rgba(15,23,42,0.18)",
    ] {
        assert!(
            snippets(&row(4, shadow)).is_empty(),
            "{shadow} should not read as the repeated signature"
        );
    }
}

#[test]
fn a_row_of_cards_with_different_tags_is_not_a_row() {
    let html = "<!DOCTYPE html><html><body><div class=\"row\">\
<div style=\"border:1px solid #e5e7eb;box-shadow:0 0 40px rgba(15,23,42,0.18)\">A</div>\
<section style=\"border:1px solid #e5e7eb;box-shadow:0 0 40px rgba(15,23,42,0.18)\">B</section>\
<aside style=\"border:1px solid #e5e7eb;box-shadow:0 0 40px rgba(15,23,42,0.18)\">C</aside>\
</div></body></html>";
    assert!(snippets(html).is_empty());
}

#[test]
fn fixture_flag_and_pass_columns() {
    let fixture = repo_root().join("tests/fixtures/antipatterns/gpt-thin-border-wide-shadow.html");
    let html = std::fs::read_to_string(&fixture).unwrap();
    let found: Vec<String> = detect_html_source(&html, &fixture, &DetectHtmlOptions::default())
        .into_iter()
        .filter(|f| f.antipattern == "gpt-thin-border-wide-shadow")
        .map(|f| f.snippet)
        .collect();
    // The two flag rows and nothing from the pass column.
    assert_eq!(
        found,
        vec![
            "1px border + 40px shadow blur",
            "1px border + 40px shadow blur",
            "1px border + 40px shadow blur",
            "1px border + 48px shadow blur",
            "1px border + 48px shadow blur",
            "1px border + 48px shadow blur",
        ],
        "fixture columns moved"
    );
}
