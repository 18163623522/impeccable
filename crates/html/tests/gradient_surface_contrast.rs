//! How the static engine resolves the surface and the ink of a piece of
//! text for low-contrast: translucent fills, gradient tiles that cover no
//! glyph, the gradient named as the source, and the ink's alpha and the
//! opacity of the boxes around it blended before scoring.

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
    let fixture = repo_root().join("tests/fixtures/antipatterns/gradient-surface-contrast.html");
    assert!(fixture.is_file(), "missing fixture at {}", fixture.display());
    let html = std::fs::read_to_string(&fixture).unwrap();
    low_contrast_snippets(&html, &fixture)
}

#[test]
fn fixture_flags_every_should_flag_case() {
    let snippets = fixture_snippets();
    for (case, expected) in [
        ("label on the light band of its gradient", "text #fffdf7 on #fde68a (gradient on a.band-button)"),
        ("copy on a gradient too light everywhere", "text #d1d5dc on #e5e7eb (gradient on div.pale-panel)"),
        ("link filled by a covering gradient", "text #d4d4d8 on #e4e4e7 (gradient on a.fill-link)"),
        ("label on a faint orange tint", "text #ea580d on #fdeee7"),
        ("span at half opacity", "text #b5b9c0 on #ffffff"),
        ("copy in a translucent ink", "text #acaeb3 on #ffffff"),
        ("copy inside a faded wrapper", "text #9399a1 on #ffffff"),
    ] {
        assert!(
            snippets.iter().any(|s| s.ends_with(expected)),
            "{case} should flag as `{expected}`, got {snippets:?}"
        );
    }
}

#[test]
fn fixture_passes_every_should_pass_case_the_static_engine_can_read() {
    let snippets = fixture_snippets();
    for (case, color) in [
        ("link with a full-width gradient underline", "#1e293b"),
        ("dark label on a faint dark tint", "#0f172b"),
        ("span at nine tenths opacity", "#111828"),
        ("copy in a nearly opaque ink", "#111827"),
    ] {
        assert!(
            !snippets.iter().any(|s| s.contains(color)),
            "{case} ({color}) should not flag, got {snippets:?}"
        );
    }
    // No layout: the gradient is scored at its worst stop, as it always was.
    // The cascade carries no `background-size` longhand either, so only an
    // underline written in the `background` shorthand can be read as one.
    for (case, color) in [
        ("label on the dark band of its gradient", "#fffdf8"),
        ("pill far from a radial glow", "#475569"),
        ("link with a collapsed gradient underline", "#0f172a"),
    ] {
        assert!(
            snippets.iter().any(|s| s.contains(color)),
            "{case} ({color}) keeps the worst-stop verdict here, got {snippets:?}"
        );
    }
}

#[test]
fn the_shared_walk_still_skips_faint_fills_for_other_rules() {
    // The contrast walk composites a 10% tint; the glow and hover checks
    // keep the walk they had, which the oracle pins.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
.chip { background: rgba(0, 0, 0, 0.1); color: #767676; font-size: 14px; padding: 4px; }
</style></head>
<body><span class="chip">Chip</span></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/tint.html"));
    assert!(snippets.iter().any(|s| s.ends_with("text #767676 on #e6e6e6")), "{snippets:?}");
}

#[test]
fn a_faded_box_with_its_own_fill_fades_that_fill_too() {
    // A dark card at half opacity over white: the card reads mid-grey, and
    // white copy on it is scored against that grey, not against black.
    let html = r#"<!DOCTYPE html>
<html><head><style>
body { background: #ffffff; }
.card { opacity: 0.5; background: rgba(0, 0, 0, 0.9); padding: 12px; }
.copy { color: #ffffff; font-size: 14px; }
</style></head>
<body><div class="card"><p class="copy">Copy on a faded dark card</p></div></body></html>
"#;
    let snippets = low_contrast_snippets(html, Path::new("/tmp/faded-card.html"));
    assert!(snippets.iter().any(|s| s.ends_with("text #ffffff on #8c8c8c")), "{snippets:?}");
}
