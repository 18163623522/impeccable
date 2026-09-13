//! `all-caps-body` only fires on an uppercase run that reads as a sentence.
//! Short single-line labels, CTAs, kickers and footer legal lines set in caps
//! are a convention; the corpus judged every one of them harmless.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    if let Ok(p) = std::env::var("IMPECCABLE_PUBLIC_REPO") {
        return PathBuf::from(p);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> Option<PathBuf> {
    let p = repo_root().join("tests/fixtures/antipatterns/all-caps-body.html");
    p.is_file().then_some(p)
}

/// 1-indexed line the marker text sits on.
fn line_of(src: &str, needle: &str) -> f64 {
    src.lines()
        .position(|l| l.contains(needle))
        .map(|i| (i + 1) as f64)
        .unwrap_or_else(|| panic!("fixture has no {needle:?}"))
}

#[test]
fn fixture_columns_split_on_run_length() {
    let Some(path) = fixture() else {
        eprintln!("fixture not found; skipping");
        return;
    };
    let src = std::fs::read_to_string(&path).unwrap();
    let findings = detect_html_source(&src, &path, &DetectHtmlOptions::default());
    let lines: Vec<f64> = findings
        .iter()
        .filter(|f| f.antipattern == "all-caps-body")
        .map(|f| f.line)
        .collect();

    let divide = line_of(&src, "id=\"should-pass\"");
    assert_eq!(lines.len(), 3, "expected the three caps paragraphs, got {lines:?}");
    assert!(
        lines.iter().all(|&l| l < divide),
        "a should-pass element was flagged: {lines:?} against the divide at {divide}"
    );
}

#[test]
fn short_single_line_caps_label_passes() {
    let html = r#"<!DOCTYPE html>
<html><head><style>
.cta { text-transform: uppercase; font-size: 12px; line-height: 18px; }
</style></head>
<body><a class="cta" href="/blog">How Mintlify is scaling sales-led GTM</a></body></html>
"#;
    let findings = detect_html_source(html, Path::new("/tmp/caps.html"), &DetectHtmlOptions::default());
    assert!(
        !findings.iter().any(|f| f.antipattern == "all-caps-body"),
        "37-char single-line CTA should stay silent, got {findings:?}"
    );
}

#[test]
fn long_caps_run_flags() {
    let html = r#"<!DOCTYPE html>
<html><head><style>
.blurb { text-transform: uppercase; font-size: 15px; line-height: 24px; width: 440px; }
</style></head>
<body><p class="blurb">Every plan includes unlimited seats, priority support and a full audit trail for your whole team.</p></body></html>
"#;
    let findings = detect_html_source(html, Path::new("/tmp/caps.html"), &DetectHtmlOptions::default());
    assert!(
        findings.iter().any(|f| f.antipattern == "all-caps-body"),
        "96-char caps paragraph should flag, got {findings:?}"
    );
}
