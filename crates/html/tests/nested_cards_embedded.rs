//! The file scan's half of decision r4-p17-nested-cards-embedded-content: a
//! box that frames embedded content, and any box whose outer card is a
//! dialog, is not a nested card.
//!
//! The file scan has no layout, so it cannot measure how much of a box a
//! figure covers. It reads a figure as an `<svg>` or a `<canvas>` in a
//! `<figure>` or beside a `<figcaption>`, and a monospace output block as one
//! run of text in the tags that set one (`pre`, `code`, `samp`, `kbd`,
//! `output`) or a declared monospace face. Decision r4-p16 (fill-only inner boxes) needs
//! nothing here: this engine has only ever counted a box with a four-sided
//! border or a shadow as a card.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

fn nested(body: &str) -> usize {
    let html = format!(
        "<!DOCTYPE html><html><head><style>body {{ font-family: Inter, sans-serif; }}</style></head><body>{body}</body></html>"
    );
    detect_html_source(
        &html,
        Path::new("/nonexistent/dir/x.html"),
        &DetectHtmlOptions::default(),
    )
    .into_iter()
    .filter(|f| f.antipattern == "nested-cards")
    .count()
}

const OUTER: &str = "<div class=\"border rounded-2xl\"><p>An outer card with copy of its own.</p>";

fn in_card(inner: &str) -> String {
    format!("{OUTER}{inner}</div>")
}

#[test]
fn ordinary_inner_cards_still_report() {
    assert_eq!(
        nested(&in_card(
            "<div class=\"border rounded-xl\"><p>An inner card with a paragraph and a control.</p><button>Save</button></div>"
        )),
        1
    );
    // An icon svg without a caption is not a figure.
    assert_eq!(
        nested(&in_card(
            "<div class=\"border rounded-xl\"><svg width=\"24\" height=\"24\"></svg><p>Feature copy beside an icon.</p></div>"
        )),
        1
    );
    // A stack of rows set in a monospace face is ordinary text.
    assert_eq!(
        nested(&in_card(
            "<div class=\"border rounded-xl\" style=\"font-family: 'JetBrains Mono', monospace\">\
             <div>Vertex AI speech to speech</div><div>Deepgram speech to text</div><div>Gemini Flash brain</div></div>"
        )),
        1
    );
    // A single inline code run inside ordinary copy is not an output block.
    assert_eq!(
        nested(&in_card(
            "<div class=\"border rounded-xl\"><p>Run <code>npm i</code> to install the package, then open the dashboard.</p></div>"
        )),
        1
    );
}

#[test]
fn figures_players_and_output_blocks_are_embedded_content() {
    for inner in [
        "<figure class=\"border rounded-xl\"><svg width=\"446\" height=\"168\"></svg><figcaption>Late 2024: open weights at 40% of the best.</figcaption></figure>",
        "<div class=\"border rounded-xl\"><canvas width=\"446\" height=\"168\"></canvas><figcaption>Traffic by hour, last week.</figcaption></div>",
        "<div class=\"border rounded-xl\"><audio src=\"a.mp3\"></audio><p>Sample audio, speaker A</p></div>",
        "<div class=\"border rounded-xl\"><button aria-label=\"Play\"></button><div role=\"slider\" aria-label=\"Audio progress\"></div><span>0:25 of 3:10</span></div>",
        "<div class=\"border rounded-xl\"><pre>PHISHING INVESTIGATION REPORT\n==============================\nSeverity: high</pre></div>",
        "<div class=\"border rounded-xl\"><p>agent setup</p><p style=\"font-family: 'JetBrains Mono', monospace\">Signup for an account and get an API key with <a href=\"/auth\">context.dev/auth.md</a>, then follow the quickstart to integrate it into the codebase.</p></div>",
    ] {
        assert_eq!(nested(&in_card(inner)), 0, "{inner}");
    }
}

#[test]
fn a_monospace_page_keeps_its_cards() {
    assert_eq!(
        nested(
            "<div class=\"border rounded-2xl\" style=\"font-family: 'JetBrains Mono', monospace\"><p>An outer card set in mono.</p>\
             <div class=\"border rounded-xl\" style=\"font-family: 'JetBrains Mono', monospace\"><p>An inner card in the same face.</p></div></div>"
        ),
        1
    );
}

#[test]
fn a_dialog_outer_card_holds_no_nested_cards() {
    let inner = "<div class=\"border rounded-xl\"><p>An inner card with copy of its own.</p></div>";
    for outer in [
        "<div class=\"border rounded-2xl\" role=\"dialog\" aria-label=\"Welcome\"><p>Welcome to the brief.</p>",
        "<div class=\"border rounded-2xl\" aria-modal=\"true\"><p>Welcome to the brief.</p>",
        "<dialog open class=\"border rounded-2xl\"><p>Welcome to the brief.</p>",
    ] {
        let close = if outer.starts_with("<dialog") { "</dialog>" } else { "</div>" };
        assert_eq!(nested(&format!("{outer}{inner}{close}")), 0, "{outer}");
    }
    // The panel inside a modal overlay.
    assert_eq!(
        nested(&format!(
            "<div role=\"dialog\" aria-modal=\"true\">{}</div>",
            in_card(inner)
        )),
        0
    );
    assert_eq!(
        nested(&format!("<div role=\"region\">{}</div>", in_card(inner))),
        1
    );
}
