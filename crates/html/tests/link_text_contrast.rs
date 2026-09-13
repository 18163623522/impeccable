//! Contrast scoring for links, spans and the other SAFE_TAGS elements that
//! paint their own text without painting their own surface.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    std::env::var("IMPECCABLE_PUBLIC_REPO")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn low_contrast_snippets(html: &str, path: &Path) -> Vec<String> {
    detect_html_source(html, path, &DetectHtmlOptions::default())
        .into_iter()
        .filter(|f| f.antipattern == "low-contrast")
        .map(|f| f.snippet)
        .collect()
}

fn fixture_snippets() -> Vec<String> {
    let fixture = repo_root().join("tests/fixtures/antipatterns/link-text-contrast.html");
    assert!(
        fixture.is_file(),
        "missing fixture at {}",
        fixture.display()
    );
    let html = std::fs::read_to_string(&fixture).unwrap();
    low_contrast_snippets(&html, &fixture)
}

#[test]
fn fixture_flags_every_should_flag_case() {
    let snippets = fixture_snippets();
    for (case, color) in [
        ("accent link", "#f37b2e"),
        ("footer imprint span", "#8a8a8a"),
        ("badge label in a filled anchor", "#ea580c"),
        ("list item", "#9b9b9b"),
        ("table cell", "#8d99a6"),
        ("ghost button", "#7c8494"),
        ("form label", "#949494"),
        ("paragraph with an inherited run", "#999999"),
        ("repeated nav link", "#888888"),
    ] {
        assert!(
            snippets.iter().any(|s| s.contains(color)),
            "{case} ({color}) should flag, got {snippets:?}"
        );
    }
}

#[test]
fn fixture_passes_every_should_pass_case() {
    let snippets = fixture_snippets();
    for (case, color) in [
        ("readable blue link", "#0b4fbc"),
        ("large link above the large-text threshold", "#838383"),
        ("screen-reader-only text", "#b1b1b1"),
        ("icon glyph", "#b2b2b2"),
        ("link with no text of its own", "#b3b3b3"),
        ("emoji-only span", "#b4b4b4"),
        ("disabled control", "#b6b6b6"),
    ] {
        assert!(
            !snippets.iter().any(|s| s.contains(color)),
            "{case} ({color}) should not flag, got {snippets:?}"
        );
    }
    // The paragraph is scored; the span repeating its colour is not.
    assert_eq!(
        snippets.iter().filter(|s| s.contains("#999999")).count(),
        1,
        "an inherited run must not repeat its paragraph's finding, {snippets:?}"
    );
    // Eight links, one colour, one finding.
    assert_eq!(
        snippets.iter().filter(|s| s.contains("#888888")).count(),
        1,
        "a repeated link colour must be reported once, {snippets:?}"
    );
    // The should-pass column contributes nothing at all, not just no
    // contrast findings: the fixture is a clean negative control.
    let fixture = repo_root().join("tests/fixtures/antipatterns/link-text-contrast.html");
    let html = std::fs::read_to_string(&fixture).unwrap();
    let all = detect_html_source(&html, &fixture, &DetectHtmlOptions::default());
    assert_eq!(all.len(), snippets.len(), "{all:?}");
}

#[test]
fn a_large_link_above_the_threshold_would_fail_at_body_size() {
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
a.small { color: #838383; font-size: 14px; }
</style></head>
<body><a class="small" href="/plans">Compare the plans</a></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/large-link.html"));
    assert!(
        snippets.iter().any(|s| s.contains("#838383")),
        "the fixture's large link passes on size, not on colour: {snippets:?}"
    );
}

#[test]
fn a_glyph_only_ancestor_does_not_silence_its_run() {
    // `<a><span>label</span> arrow</a>`: nothing scores the anchor, so the
    // span has to report its own colour.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
a.more { color: #949494; font-size: 14px; }
</style></head>
<body><a class="more" href="/x"><span>Read more about the service</span> &#8594;</a></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/glyph.html"));
    assert_eq!(
        snippets.iter().filter(|s| s.contains("#949494")).count(),
        1,
        "{snippets:?}"
    );
}

#[test]
fn a_run_on_its_own_surface_keeps_its_own_verdict() {
    // White copy on a light section, repeated inside a green card: the
    // ancestor's verdict is against a different background, so the run is
    // scored where it stands.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
section { color: #ffffff; background: #1f2937; font-size: 14px; padding: 16px; }
div.card { background: #16a34a; padding: 12px; }
span.cta { font-size: 14px; }
</style></head>
<body><section>Try the demo<div class="card"><span class="cta">Book a slot</span></div></section></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/surface.html"));
    assert!(
        snippets.iter().any(|s| s.contains("#ffffff on #16a34a")),
        "{snippets:?}"
    );
}

#[test]
fn a_disabled_control_is_not_scored() {
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
button { color: #b0b0b0; background: transparent; border: 0; font-size: 14px; }
</style></head>
<body><button disabled>Generate</button><button aria-disabled="true">Export</button></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/disabled.html"));
    assert!(snippets.is_empty(), "{snippets:?}");
}

#[test]
fn a_background_read_as_the_text_colour_is_not_a_report() {
    // The static engine resolves the label's background to the page's own
    // white because it cannot see the photo behind it. 1.0:1 with identical
    // hexes is a resolution artefact, not a finding.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
div.hero { background-image: url(/hero.jpg); padding: 40px; }
a.lang { color: #ffffff; font-size: 14px; }
</style></head>
<body><div class="hero"><a class="lang" href="/en">English</a></div></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/hero.html"));
    assert!(snippets.is_empty(), "{snippets:?}");
}

#[test]
fn a_run_that_sets_its_own_colour_is_scored() {
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
p { color: #1f2937; font-size: 14px; }
span.pale { color: #a3a3a3; }
</style></head>
<body><p>Readable copy <span class="pale">with a pale run</span></p></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/link-text-contrast.html"));
    assert!(
        snippets.iter().any(|s| s.contains("#a3a3a3")),
        "{snippets:?}"
    );
}

#[test]
fn a_styled_control_still_reports_its_surface_rules() {
    // The tag gate stays for everything but the contrast verdict: a span
    // with its own fill keeps the full check, gray-on-color included.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
span.chip { background: #b6322d; color: #5c5449; font-size: 14px; padding: 4px 8px; }
</style></head>
<body><span class="chip">Severity low</span></body></html>
"#;
    let findings = detect_html_source(
        html,
        Path::new("/tmp/chip.html"),
        &DetectHtmlOptions::default(),
    );
    let ids: Vec<&str> = findings.iter().map(|f| f.antipattern.as_str()).collect();
    assert!(ids.contains(&"low-contrast"), "{findings:?}");
    assert!(ids.contains(&"gray-on-color"), "{findings:?}");
}
