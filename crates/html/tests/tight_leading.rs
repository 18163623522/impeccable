//! The tight-leading floor over the static engine: it measures multi-line
//! body copy and nothing else. Review of the rule's output on real pages found
//! it applied to display type, to heading text that sits in a child anchor or
//! span, to text that never renders, and to pages that set the floor exactly.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    std::env::var("IMPECCABLE_PUBLIC_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn leading_snippets(html: &str, path: &Path) -> Vec<String> {
    detect_html_source(html, path, &DetectHtmlOptions::default())
        .into_iter()
        .filter(|f| f.antipattern == "tight-leading")
        .map(|f| f.snippet)
        .collect()
}

fn scan(html: &str) -> Vec<String> {
    leading_snippets(html, Path::new("/tmp/tight-leading.html"))
}

fn page(head: &str, body: &str) -> String {
    format!("<!DOCTYPE html><html><head><style>{head}</style></head><body>{body}</body></html>")
}

const COPY: &str = "This paragraph is reading copy that is comfortably longer than the fifty characters the check asks for before it measures leading.";

#[test]
fn fixture_flag_and_pass_cases() {
    let fixture = repo_root().join("tests/fixtures/antipatterns/tight-leading.html");
    assert!(fixture.is_file(), "missing fixture at {}", fixture.display());
    let html = std::fs::read_to_string(&fixture).unwrap();
    let snippets = leading_snippets(&html, &fixture);

    // The five should-flag cases, each set at a ratio of its own.
    for ratio in ["1.25", "1.20", "1.17", "1.14", "1.10"] {
        assert!(
            snippets.iter().any(|s| s.contains(ratio)),
            "expected a tight-leading finding at {ratio}x, got {snippets:?}"
        );
    }
    // Display type (1.13), heading link (1.13), role=heading card title
    // (1.27), the paragraph at exactly the floor (1.30), the display:none
    // variant (1.07) and the clipped screen-reader copy (1.00) all pass.
    for ratio in ["1.13", "1.27", "1.30", "1.07", "1.00"] {
        assert!(
            !snippets.iter().any(|s| s.contains(ratio)),
            "pass case at {ratio}x should not flag, got {snippets:?}"
        );
    }
    // The single-line case is the one the static engine cannot judge: it has
    // no layout, so it still reports the 1.15x label. The browser engine's
    // wrap test is pinned in crates/core.
    assert_eq!(
        snippets.len(),
        6,
        "expected five flags plus the documented single-line static hit, got {snippets:?}"
    );
}

#[test]
fn display_type_sets_its_own_leading() {
    let flagged = scan(&page(
        "p { width: 300px; font-size: 18px; line-height: 21.6px; }",
        &format!("<p>{COPY}</p>"),
    ));
    assert_eq!(flagged.len(), 1, "18px body copy at 1.2 flags: {flagged:?}");

    let display = scan(&page(
        "p { width: 600px; font-size: 32px; line-height: 38.4px; }",
        &format!("<p>{COPY}</p>"),
    ));
    assert!(display.is_empty(), "32px display type passes: {display:?}");
}

#[test]
fn heading_exemption_follows_the_ancestor() {
    let link = scan(&page(
        "h3 { font-size: 30px; line-height: 34px; } a { color: #111; }",
        &format!("<h3><a href=\"/story\">{COPY}</a></h3>"),
    ));
    assert!(link.is_empty(), "heading text in a child anchor passes: {link:?}");

    let role = scan(&page(
        "span { font-size: 22px; line-height: 28px; }",
        &format!("<div><span role=\"heading\" aria-level=\"3\">{COPY}</span></div>"),
    ));
    assert!(role.is_empty(), "role=heading card title passes: {role:?}");

    // A block of reading copy nested inside a heading is body copy, and the
    // ancestor exemption must not reach it.
    let nested = scan(&page(
        "h3 { font-size: 16px; } p { width: 300px; font-size: 16px; line-height: 17.6px; }",
        &format!("<h3><p>{COPY}</p></h3>"),
    ));
    assert_eq!(nested.len(), 1, "paragraph nested in a heading still flags: {nested:?}");
}

#[test]
fn exactly_at_the_floor_never_flags() {
    for (size, leading) in [("18px", "23.4px"), ("16px", "20.8px"), ("14px", "18.2px")] {
        let hits = scan(&page(
            &format!("p {{ width: 300px; font-size: {size}; line-height: {leading}; }}"),
            &format!("<p>{COPY}</p>"),
        ));
        assert!(hits.is_empty(), "{size}/{leading} is exactly 1.3: {hits:?}");
    }
}

#[test]
fn source_text_is_never_measured() {
    let hits = scan(&page(
        "body { font-size: 16px; line-height: 16px; } .gone { display: none; } .off { visibility: hidden; }",
        &format!(
            "<script>window.__DATA__ = {{ id: \"a\", label: \"{COPY}\" }};</script>\
             <noscript>&lt;iframe src=\"https://example.test/t\" height=\"0\"&gt;&lt;/iframe&gt; {COPY}</noscript>\
             <p class=\"gone\">{COPY}</p><p class=\"off\">{COPY}</p>"
        ),
    ));
    assert!(hits.is_empty(), "non-rendered text has no leading to judge: {hits:?}");
}
