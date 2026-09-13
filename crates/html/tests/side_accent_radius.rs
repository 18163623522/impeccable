//! The side-accent gate through the static cascade: a card is rounded by the
//! `border-radius` shorthand, by the corner longhands a utility framework
//! emits, or not at all, and an unreadable radius is unknown rather than
//! square.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

fn side_tab_snippets(card_css: &str) -> Vec<String> {
    let html = format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>t</title>\
<style>.card{{width:400px;height:120px;padding:16px;background:#fdfdfd;{card_css}}}</style>\
</head><body><div class=\"card\"><h3>Card</h3><p>Body copy.</p></div></body></html>"
    );
    let opts = DetectHtmlOptions::default();
    let findings = detect_html_source(&html, Path::new("/nonexistent/dir/page.html"), &opts);
    let value = serde_json::to_value(&findings).unwrap();
    value
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["antipattern"] == "side-tab")
        .map(|f| f["snippet"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn a_square_card_with_a_side_rule_stays_silent() {
    assert!(side_tab_snippets("border-left:4px solid #6366f1;").is_empty());
    assert!(side_tab_snippets("border-left:4px solid #6366f1;border-radius:0;").is_empty());
    // Rounded only where the stripe runs: still square where it counts.
    assert!(
        side_tab_snippets("border-left:4px solid #6366f1;border-radius:10px 0 0 10px;").is_empty()
    );
    // The same shape written as longhands.
    assert!(side_tab_snippets(
        "border-left:4px solid #6366f1;border-top-left-radius:10px;border-bottom-left-radius:10px;"
    )
    .is_empty());
}

#[test]
fn corner_longhands_round_a_card_the_shorthand_never_names() {
    // `border-l-4 border-indigo-500 rounded-r-lg` compiles to exactly this.
    assert_eq!(
        side_tab_snippets(
            "border-left-width:4px;border-left-style:solid;border-left-color:#6366f1;\
border-top-right-radius:.5rem;border-bottom-right-radius:.5rem;"
        ),
        vec!["border-left: 4px".to_string()]
    );
    // A longhand that rounds a corner the shorthand squared off counts too.
    assert_eq!(
        side_tab_snippets(
            "border-left:4px solid #6366f1;border-radius:0;border-top-right-radius:10px;\
border-bottom-right-radius:10px;"
        ),
        vec!["border-left: 4px".to_string()]
    );
}

#[test]
fn an_unreadable_radius_keeps_the_finding() {
    // calc(), var() and units this cannot resolve are unknown, not square.
    for css in [
        "border-left:4px solid #6366f1;border-radius:calc(0.5rem);",
        "border-left:4px solid #6366f1;border-radius:var(--r);",
        "border-left:4px solid #6366f1;border-radius:1vw;",
        "border-left:4px solid #6366f1;border-top-right-radius:calc(8px);",
    ] {
        assert_eq!(side_tab_snippets(css).len(), 1, "css {css:?}");
    }
    // A unit spelled in capitals is the same unit.
    assert_eq!(
        side_tab_snippets("border-left:4px solid #6366f1;border-radius:0.5REM;").len(),
        1
    );
}

#[test]
fn a_rounded_card_with_a_side_accent_still_reports() {
    assert_eq!(
        side_tab_snippets("border-left:4px solid #6366f1;border-radius:10px;"),
        vec!["border-left: 4px + border-radius: 10px".to_string()]
    );
    assert_eq!(
        side_tab_snippets("border-left:4px solid #6366f1;border-radius:0 10px 10px 0;"),
        vec!["border-left: 4px".to_string()]
    );
}
