//! `radial-spotlight-glow` over the static engine: a declared glow is only a
//! finding when it is prominent — bright against the surface it paints on,
//! not scaled away by opacity, and sitting behind copy.

use impeccable_html::{detect_html_source, DetectHtmlOptions};
use std::path::Path;

fn glow_hits(body_css: &str, body_html: &str) -> usize {
    let html = format!(
        "<!DOCTYPE html><html><head><style>{body_css}</style></head><body>{body_html}</body></html>"
    );
    let opts = DetectHtmlOptions::default();
    detect_html_source(&html, Path::new("/nonexistent/dir/x.html"), &opts)
        .into_iter()
        .filter(|f| f.antipattern == "radial-spotlight-glow")
        .count()
}

const DARK: &str = "body { background-color: #0b0d13; } \
    .glow { width: 900px; height: 600px; }";

#[test]
fn bright_glow_behind_copy_flags() {
    let css = format!(
        "{DARK} .glow {{ background-image: radial-gradient(circle, rgba(0,209,239,0.26), transparent 70%); }}"
    );
    assert_eq!(glow_hits(&css, "<div class=\"glow\"><h2>Headline</h2></div>"), 1);
}

#[test]
fn faint_wash_on_its_own_ground_stays_silent() {
    let css = format!(
        "{DARK} .glow {{ background-image: radial-gradient(circle, rgba(26,189,226,0.10), transparent 70%); }}"
    );
    assert_eq!(glow_hits(&css, "<div class=\"glow\"><h2>Headline</h2></div>"), 0);
}

#[test]
fn pastel_glow_on_a_white_surface_stays_silent() {
    let css = "body { background-color: #ffffff; } \
        .glow { width: 900px; height: 600px; \
        background-image: radial-gradient(circle, rgba(63,227,223,0.20), transparent 65%); }";
    assert_eq!(glow_hits(css, "<div class=\"glow\"><h2>Headline</h2></div>"), 0);
}

#[test]
fn element_opacity_scales_the_glow_away() {
    let css = format!(
        "{DARK} .glow {{ opacity: 0.35; background-image: radial-gradient(circle, rgba(0,209,239,0.38), transparent 70%); }}"
    );
    assert_eq!(glow_hits(&css, "<div class=\"glow\"><h2>Headline</h2></div>"), 0);
}

#[test]
fn a_glow_with_no_copy_over_it_is_surface_treatment() {
    let css = format!(
        "{DARK} .glow {{ background-image: radial-gradient(circle, rgba(0,209,239,0.26), transparent 70%); }}"
    );
    assert_eq!(glow_hits(&css, "<figure class=\"glow\"></figure>"), 0);
}

#[test]
fn an_overlay_layer_inherits_the_copy_of_the_section_it_covers() {
    let css = format!(
        "{DARK} .hero {{ position: relative; background-color: #0b0d13; }} \
         .glow {{ position: absolute; background-image: radial-gradient(circle, rgba(0,209,239,0.26), transparent 70%); }}"
    );
    assert_eq!(
        glow_hits(
            &css,
            "<section class=\"hero\"><h2>Headline</h2><div class=\"glow\"></div></section>"
        ),
        1
    );
}
