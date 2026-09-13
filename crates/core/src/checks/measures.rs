//! Port of cli/engine/rules/checks.mjs (see checks/mod.rs for the split):
//! the plain-data helpers and pure gates of Sections 4-6. Element / document
//! adapters live in the `html` crate.
//!
//! Style-reading helpers take a [`StyleMap`]: any lookup from the JS
//! camelCase computed-style property name (`borderTopWidth`, `clipPath`) to
//! its string value, so a jsdom-style map, a real cascade, and a test
//! `HashMap` all fit.

use crate::color::{self, Rgba};

use crate::js::{self, math_max, math_min3, math_round, number_to_string, to_fixed, WS, WS_CHARS};

use crate::js_ext_b::{slice_utf16_prefix, utf16_len};

use once_cell::sync::Lazy;
use regex::Regex;

/// The CSS value helpers, style traits and plain-data types these checks are
/// written against are shared; re-exported so `checks::measures` stays one path.
pub use impeccable_foundation::css::measures::*;

/// JS `\d` is ASCII only.
const D: &str = "[0-9]";

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

/// The chromatic stop a spotlight-glow declaration glows with, or `None` when
/// the value is not one of those declarations: opaque at its far end, too
/// many colors, too solid a center, or grayscale.
pub fn radial_spotlight_stop(gradient_value: Option<&str>) -> Option<Rgba> {
    let stops = parse_radial_gradient_stops(gradient_value)?;
    if stops.len() < 2 {
        return None;
    }
    let last = &stops[stops.len() - 1];
    let last_alpha = if last.transparent {
        0.0
    } else {
        last.color.map(|c| c.alpha_or_one()).unwrap_or(1.0)
    };
    if last_alpha > 0.05 {
        return None;
    }
    let colored: Vec<&GradientStop> = stops
        .iter()
        .filter(|s| !s.transparent && matches!(s.color, Some(c) if c.alpha_or_one() > 0.05))
        .collect();
    if colored.is_empty() {
        return None;
    }
    if colored.len() > 2 {
        return None;
    }
    if colored
        .iter()
        .any(|s| s.color.map(|c| c.alpha_or_one()).unwrap_or(1.0) >= 0.45)
    {
        return None;
    }
    // The stop the glow reads as is its brightest chromatic one, not the first
    // one declared: two gradients that paint the same cloud may list their
    // stops either way round. Ties keep declaration order.
    let mut brightest: Option<Rgba> = None;
    let mut brightest_luminance = f64::NEG_INFINITY;
    for s in colored
        .iter()
        .filter(|s| color::has_chroma(s.color.as_ref(), Some(24.0)))
    {
        let c = s.color.expect("colored stop has a color");
        let l = color::relative_luminance(&c);
        if l > brightest_luminance {
            brightest_luminance = l;
            brightest = Some(c);
        }
    }
    brightest
}

/// What an element's `background-image` offers as a surface a glow could be
/// measured against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackdropLayer {
    /// Nothing is painted here; the surface is further up the tree.
    Absent,
    /// Something is painted, but no one color names it: a photo, or a
    /// gradient carrying a translucent or unreadable stop.
    Unreadable,
    /// The mean color of an opaque gradient.
    Color(Rgba),
}

/// Read the top paint layer of a `background-image` value as the surface a
/// glow over it sits on. A hero that paints itself with a flat color is what
/// `resolveBackground` already answers; this is the other common case, a hero
/// painted with a gradient, which names no single color until its stops are
/// averaged.
pub fn backdrop_layer(background_image: Option<&str>) -> BackdropLayer {
    let Some(raw) = background_image else {
        return BackdropLayer::Absent;
    };
    let value = js::trim(raw);
    if value.is_empty() {
        return BackdropLayer::Absent;
    }
    let mut top: Option<String> = None;
    for layer in color::split_top_level_commas(value) {
        let lower = js::to_lower_case(&layer);
        if lower.contains("url(") || lower.contains("gradient(") {
            top = Some(layer);
            break;
        }
    }
    let Some(layer) = top else {
        return BackdropLayer::Absent;
    };
    let lower = js::to_lower_case(&layer);
    if lower.contains("url(") {
        return BackdropLayer::Unreadable;
    }
    // A `transparent` stop lets the surface under this one through, and
    // `parseGradientColors` does not report it, so the layer is only opaque
    // when no stop names it.
    if lower.contains("transparent") {
        return BackdropLayer::Unreadable;
    }
    let stops = color::parse_gradient_colors(Some(&layer));
    if stops.is_empty() || stops.iter().any(|c| c.alpha_or_one() < 0.99) {
        return BackdropLayer::Unreadable;
    }
    let n = stops.len() as f64;
    let sum = stops.iter().fold((0.0, 0.0, 0.0), |acc, c| {
        (acc.0 + c.r, acc.1 + c.g, acc.2 + c.b)
    });
    BackdropLayer::Color(Rgba::new(sum.0 / n, sum.1 / n, sum.2 / n, 1.0))
}

/// The least effective alpha (the declared stop alpha times the element's
/// opacity) a glow can carry and still read as a glow rather than as a tint
/// of the ground beneath it. On a scan corpus of live sites the layers two
/// judges read as decorative ground carried 0.05 to 0.14; the one a reviewer
/// called harmful carried 0.16.
pub const RADIAL_GLOW_MIN_EFFECTIVE_ALPHA: f64 = 0.14;

/// The least contrast a glow's brightest point can have against the surface
/// it paints on before it stops reading as a cloud floating over that
/// surface. Same corpus: every wash the judges waved through measured 1.23 or
/// less, the harmful one measured 1.33.
pub const RADIAL_GLOW_MIN_CONTRAST: f64 = 1.30;

/// Whether a declared spotlight glow is prominent enough to read as one: it
/// survives its element's opacity, it lifts the surface it paints on, and it
/// sits behind text rather than off on its own. A faint wash is a tonal shift
/// in the ground, not the hero spotlight this rule names.
///
/// `behind_text` is a closure because answering it costs a walk of the
/// document's elements, and in the browser a forced layout per element; the
/// two measurements the caller hands over are an ancestor walk each. The
/// tests run in the order written, cheapest first, so a wash that fails on
/// alpha never pays for the walk.
///
/// A backdrop of `None` is a surface no cascade can name (a photograph, an
/// unreadable stack). The contrast test is skipped rather than failed: the
/// glow still has to carry its alpha and sit behind copy, and silencing every
/// glow over a photo would drop the pattern's own hero case.
pub fn radial_glow_is_prominent(
    stop: &Rgba,
    p: &RadialGlowProminence,
    behind_text: impl FnOnce() -> bool,
) -> bool {
    let opacity = if p.opacity.is_finite() {
        p.opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let effective = stop.alpha_or_one() * opacity;
    if effective < RADIAL_GLOW_MIN_EFFECTIVE_ALPHA {
        return false;
    }
    if let Some(backdrop) = p.backdrop {
        let peak =
            color::composite_color_over(&Rgba::new(stop.r, stop.g, stop.b, effective), &backdrop);
        if color::contrast_ratio(&peak, &backdrop) < RADIAL_GLOW_MIN_CONTRAST {
            return false;
        }
    }
    behind_text()
}

/// JS: checks.mjs#checkRadialSpotlight. Pure gate over the declaration; the
/// element adapters gate its hit on [`radial_glow_is_prominent`], which needs
/// measurements this signature does not carry. A caller reaching this through
/// the pure wasm export therefore gets the declaration test alone and owes
/// itself the prominence gate. `label` is a stable identifier the fixture test
/// keys on.
pub fn check_radial_spotlight(input: &RadialSpotlightInput) -> Vec<Finding> {
    let Some(cc) = radial_spotlight_stop(input.gradient_value) else {
        return vec![];
    };
    if !(input.width >= 240.0 && input.height >= 160.0) {
        return vec![];
    }
    let alpha = to_fixed(cc.alpha_or_one(), 2);
    let name = match input.label {
        Some(l) if !l.is_empty() => l,
        _ => "section",
    };
    vec![Finding::new(
        "radial-spotlight-glow",
        format!(
            "radial-gradient spotlight glow \"{}\" ({} a{} → transparent) on {}x{} surface",
            name,
            color::color_to_hex(Some(&cc)),
            alpha,
            number_to_string(math_round(input.width)),
            number_to_string(math_round(input.height))
        ),
    )]
}

// ─── Cream / beige palette ──────────────────────────────────────────────────

/// JS: checks.mjs#isCreamColor. A warm, lightly-tinted off-white.
pub fn is_cream_color(rgb: Option<&Rgba>) -> bool {
    let Some(c) = rgb else { return false };
    let (r, g, b) = (c.r, c.g, c.b);
    if math_min3(r, g, b) < 209.0 {
        return false;
    }
    if !(r >= g && g >= b) {
        return false;
    }
    let warmth = r - b;
    warmth >= 6.0 && warmth <= 48.0
}

/// JS: checks.mjs#creamFromClassList. The Tailwind background token that
/// renders as a cream surface, or `None`.
pub fn cream_from_class_list(cls: Option<&str>) -> Option<String> {
    re!(ARB_RE, r"(?-u:\b)bg-\[([^\]]+)\]");
    let cls = cls?;
    if cls.is_empty() {
        return None;
    }
    if let Some(arb) = ARB_RE.captures(cls) {
        let inner = arb.get(1).map(|m| m.as_str()).unwrap_or("");
        let spaced = inner.replace('_', " ");
        if is_cream_color(color::parse_any_color(Some(&spaced)).as_ref()) {
            return Some(format!("bg-[{}]", inner));
        }
    }
    for (tok, hex) in TAILWIND_BG_HEX {
        let re = Regex::new(&format!(
            "(?:^|{ws}){}(?:$|{ws})",
            regex::escape(tok),
            ws = WS
        ))
        .expect("tailwind token regex");
        if re.is_match(cls) && is_cream_color(color::parse_any_color(Some(hex)).as_ref()) {
            return Some(tok.to_string());
        }
    }
    None
}

// ─── Oversized hero headline ────────────────────────────────────────────────
const OVERSIZED_H1_FONT_PX: f64 = 72.0;

const OVERSIZED_H1_MIN_CHARS: usize = 40;
const OVERSIZED_H1_MIN_VIEWPORT_HEIGHT_RATIO: f64 = 0.28;
const OVERSIZED_H1_MIN_VIEWPORT_AREA_RATIO: f64 = 0.25;

/// JS: checks.mjs#checkOversizedH1.
pub fn check_oversized_h1(input: &OversizedH1Input) -> Vec<Finding> {
    if input.tag != "h1" {
        return vec![];
    }
    let text_len = utf16_len(input.heading_text);
    if input.font_size >= OVERSIZED_H1_FONT_PX && text_len >= OVERSIZED_H1_MIN_CHARS {
        let mut viewport_detail = String::new();
        if let Some(rect) = input.rect {
            if input.viewport_width > 0.0 && input.viewport_height > 0.0 {
                let height_ratio = rect.height / input.viewport_height;
                let area_ratio =
                    (rect.width * rect.height) / (input.viewport_width * input.viewport_height);
                let dominates = height_ratio >= OVERSIZED_H1_MIN_VIEWPORT_HEIGHT_RATIO
                    || area_ratio >= OVERSIZED_H1_MIN_VIEWPORT_AREA_RATIO;
                if !dominates {
                    return vec![];
                }
                viewport_detail =
                    format!(", {}vh", number_to_string(math_round(height_ratio * 100.0)));
            }
        }
        return vec![Finding::new(
            "oversized-h1",
            format!(
                "{}px h1, {} chars{} \"{}\"",
                number_to_string(math_round(input.font_size)),
                text_len,
                viewport_detail,
                slice_utf16_prefix(input.heading_text, 60)
            ),
        )];
    }
    vec![]
}

/// JS: checks.mjs#checkGptThinBorderWideShadow.
pub fn check_gpt_thin_border_wide_shadow(input: &GptBorderShadowInput) -> Vec<Finding> {
    let mut visible_thin: Vec<f64> = Vec::new();
    for (index, &width) in input.border_widths.iter().enumerate() {
        let color = input
            .border_colors
            .and_then(|cs| cs.get(index))
            .and_then(|c| c.as_deref())
            .filter(|c| !c.is_empty());
        let alpha = css_color_alpha(color);
        if width > 0.0 && width <= 1.5 && alpha >= 0.28 {
            visible_thin.push(width);
        }
    }
    let mut max_border = 0.0f64;
    for &w in &visible_thin {
        max_border = math_max(max_border, w);
    }
    let blur = shadow_max_blur_px(input.box_shadow, Some(0.12));
    if visible_thin.len() >= 2 && blur >= 16.0 {
        return vec![Finding::new(
            "gpt-thin-border-wide-shadow",
            format!(
                "{}px border + {}px shadow blur",
                number_to_string(max_border),
                number_to_string(math_round(blur))
            ),
        )];
    }
    vec![]
}

// ─── Clipped overflow / screen-reader-only text ─────────────────────────────

/// JS: checks.mjs#positionedStyleImpliesEscape. A positioned child's inset
/// declarations read as pushing it outside its clipping parent (negative
/// offset or a full 100% offset).
pub fn positioned_style_implies_escape(style: &dyn StyleMap) -> bool {
    re!(
        NEG_RE,
        format!(r"(?:^|[{ws}(])-+(?:{d}|\.)", ws = WS_CHARS, d = D)
    );
    re!(
        FULL_RE,
        format!(r"(?:^|[{ws}(])100(?:\.0+)?%", ws = WS_CHARS)
    );
    const PROPS: [&str; 11] = [
        "top",
        "right",
        "bottom",
        "left",
        "inset",
        "insetBlock",
        "insetInline",
        "insetBlockStart",
        "insetBlockEnd",
        "insetInlineStart",
        "insetInlineEnd",
    ];
    for prop in PROPS {
        let Some(v) = style.prop(prop) else { continue };
        if v.is_empty() {
            continue;
        }
        let value = js::to_lower_case(js::trim(&v));
        if NEG_RE.is_match(&value) {
            return true;
        }
        if FULL_RE.is_match(&value) {
            return true;
        }
    }
    false
}

/// JS: checks.mjs#checkContentHiddenAtRest. Pure threshold check over a
/// `measureHiddenTextDOM()` result.
pub fn check_content_hidden_at_rest(input: &ContentHiddenInput) -> Vec<Finding> {
    if input.total_chars < 200.0 || input.hidden_chars < 150.0 {
        return vec![];
    }
    let share = input.hidden_chars / input.total_chars;
    if share <= 0.3 {
        return vec![];
    }
    let sample = match input.hidden_samples.first() {
        Some(s) => format!(" (e.g. \"{}\")", s),
        None => String::new(),
    };
    vec![Finding::new(
        "content-hidden-at-rest",
        format!(
            "{}% of page text ({} of {} chars) stays at opacity 0 / visibility hidden after reveal handlers ran{}",
            number_to_string(math_round(share * 100.0)),
            number_to_string(input.hidden_chars),
            number_to_string(input.total_chars),
            sample
        ),
    )]
}

// ─── Text occlusion helper ──────────────────────────────────────────────────

/// JS: checks.mjs#isOpaqueDecoratedBox. A near-solid background fill or
/// two-plus visible borders make a box hide whatever sits behind it.
pub fn is_opaque_decorated_box(cs: Option<&dyn StyleMap>) -> bool {
    let Some(cs) = cs else { return false };
    let bg_raw = prop_or_empty(cs, "backgroundColor");
    if let Some(bg) = color::parse_any_color(Some(&bg_raw)) {
        if bg.alpha_or_one() > 0.6 {
            return true;
        }
    }
    let mut border_sides = 0usize;
    for side in ["Top", "Right", "Bottom", "Left"] {
        let w = parse_float_or_zero(cs.prop(&format!("border{}Width", side)).as_deref());
        if w <= 0.0 {
            continue;
        }
        let bc_raw = prop_or_empty(cs, &format!("border{}Color", side));
        if let Some(bc) = color::parse_any_color(Some(&bc_raw)) {
            if bc.alpha_or_one() > 0.3 {
                border_sides += 1;
            }
        }
    }
    border_sides >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn style(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn stop_hex(gradient: &str) -> Option<String> {
        radial_spotlight_stop(Some(gradient)).map(|c| color::color_to_hex(Some(&c)))
    }

    #[test]
    fn radial_spotlight_stop_takes_the_brightest_chromatic_stop() {
        // Two colored stops, either way round: the bright one names the glow.
        let bright_first =
            "radial-gradient(circle, rgba(120,220,255,0.40) 0%, rgba(20,20,90,0.30) 45%, transparent 75%)";
        let bright_second =
            "radial-gradient(circle, rgba(20,20,90,0.30) 0%, rgba(120,220,255,0.40) 45%, transparent 75%)";
        assert_eq!(stop_hex(bright_first).as_deref(), Some("#78dcff"));
        assert_eq!(stop_hex(bright_second).as_deref(), Some("#78dcff"));

        // Equal luminance keeps declaration order, which is what the frozen
        // vectors of the same hue at two alphas recorded.
        let same_hue =
            "radial-gradient(circle,rgba(255,90,120,0.28) 0%,rgba(255,90,120,0.12) 45%,transparent 75%)";
        assert_eq!(stop_hex(same_hue).as_deref(), Some("#ff5a78"));

        // Grayscale stops are not a glow at all.
        assert_eq!(
            stop_hex("radial-gradient(circle, rgba(200,200,200,0.30), transparent 70%)"),
            None
        );
    }

    #[test]
    fn backdrop_layer_reads_an_opaque_gradient_as_its_mean() {
        assert_eq!(backdrop_layer(None), BackdropLayer::Absent);
        assert_eq!(backdrop_layer(Some("none")), BackdropLayer::Absent);
        assert_eq!(
            backdrop_layer(Some("linear-gradient(180deg, #000000, #202020)")),
            BackdropLayer::Color(Rgba::new(16.0, 16.0, 16.0, 1.0))
        );
        // A photograph names no color, and neither does a gradient that lets
        // the surface under it through.
        assert_eq!(
            backdrop_layer(Some("url(\"/hero.jpg\")")),
            BackdropLayer::Unreadable
        );
        assert_eq!(
            backdrop_layer(Some("linear-gradient(180deg, #000000, transparent)")),
            BackdropLayer::Unreadable
        );
        assert_eq!(
            backdrop_layer(Some(
                "linear-gradient(180deg, rgba(0,0,0,0.4), rgba(32,32,32,0.4))"
            )),
            BackdropLayer::Unreadable
        );
        // The top paint layer decides; a gradient under a photo does not.
        assert_eq!(
            backdrop_layer(Some("url(/hero.jpg), linear-gradient(#000000, #202020)")),
            BackdropLayer::Unreadable
        );
    }

    #[test]
    fn radial_glow_is_prominent_orders_its_tests() {
        let stop = Rgba::new(0.0, 209.0, 239.0, 0.16);
        let dark = Rgba::new(4.0, 12.0, 19.0, 1.0);
        let lit = RadialGlowProminence {
            opacity: 1.0,
            backdrop: Some(dark),
        };
        assert!(radial_glow_is_prominent(&stop, &lit, || true));
        assert!(!radial_glow_is_prominent(&stop, &lit, || false));

        // A wash fails on alpha without ever asking about copy.
        let wash = Rgba::new(26.0, 189.0, 226.0, 0.10);
        assert!(!radial_glow_is_prominent(&wash, &lit, || {
            panic!("alpha is cheaper than the text walk and is tested first")
        }));

        // So does a bright stop an ancestor's opacity scales away.
        let scaled = RadialGlowProminence {
            opacity: 0.35,
            backdrop: Some(dark),
        };
        let bright = Rgba::new(0.0, 209.0, 239.0, 0.38);
        assert!(!radial_glow_is_prominent(&bright, &scaled, || {
            panic!("opacity is cheaper than the text walk and is tested first")
        }));

        // A surface no cascade can name skips the contrast test rather than
        // failing it: the alpha and the copy still have to carry the glow.
        let unnamed = RadialGlowProminence {
            opacity: 1.0,
            backdrop: None,
        };
        assert!(radial_glow_is_prominent(&stop, &unnamed, || true));
        assert!(!radial_glow_is_prominent(&stop, &unnamed, || false));
        assert!(!radial_glow_is_prominent(&wash, &unnamed, || true));

        // A pastel on white is declared strong and reads as nothing.
        let white = RadialGlowProminence {
            opacity: 1.0,
            backdrop: Some(Rgba::new(255.0, 255.0, 255.0, 1.0)),
        };
        let pastel = Rgba::new(63.0, 227.0, 223.0, 0.20);
        assert!(!radial_glow_is_prominent(&pastel, &white, || true));
    }

    // Expected values below were produced by running the JS functions in Node.

    #[test]
    fn css_color_is_transparent_cases() {
        assert!(css_color_is_transparent(None));
        assert!(css_color_is_transparent(Some("")));
        assert!(css_color_is_transparent(Some("  Transparent ")));
        assert!(css_color_is_transparent(Some("rgba(0, 0, 0, 0)")));
        assert!(css_color_is_transparent(Some("rgba(10,20,30,0.04)")));
        assert!(!css_color_is_transparent(Some("rgba(10,20,30,0.5)")));
        assert!(!css_color_is_transparent(Some("#fff")));
        assert!(css_color_is_transparent(Some("rgba(1, 2, 3, 0.00)")));
        assert!(!css_color_is_transparent(Some("notacolor")));
    }

    #[test]
    fn colors_nearly_match_cases() {
        assert!(colors_nearly_match(
            Some("#fff"),
            Some("rgb(254, 255, 253)")
        ));
        assert!(!colors_nearly_match(
            Some("#fff"),
            Some("rgb(250, 255, 255)")
        ));
        assert!(!colors_nearly_match(Some("#fff"), Some("nope")));
        assert!(!colors_nearly_match(
            Some("rgba(0,0,0,0.5)"),
            Some("rgba(0,0,0,0.6)")
        ));
        assert!(colors_nearly_match(
            Some("rgba(0,0,0,0.5)"),
            Some("rgba(0,0,0,0.52)")
        ));
    }

    #[test]
    fn parse_radial_gradient_stops_cases() {
        assert_eq!(parse_radial_gradient_stops(None), None);
        assert_eq!(
            parse_radial_gradient_stops(Some("linear-gradient(red, blue)")),
            None
        );
        assert_eq!(
            parse_radial_gradient_stops(Some(
                "repeating-radial-gradient(circle, #000 0 2px, transparent 2px 4px)"
            )),
            None
        );
        let stops = parse_radial_gradient_stops(Some(
            "radial-gradient(circle at 50% 50%, rgba(80,111,255,0.26), transparent 44%)",
        ))
        .unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].color, Some(Rgba::new(80.0, 111.0, 255.0, 0.26)));
        assert!(!stops[0].transparent);
        assert_eq!(stops[1].color, None);
        assert!(stops[1].transparent);
        assert_eq!(
            parse_radial_gradient_stops(Some("radial-gradient(#fff, #000")),
            None
        );
        assert_eq!(
            parse_radial_gradient_stops(Some("radial-gradient(circle, #fff)")),
            None
        );
    }

    #[test]
    fn shadow_layer_alpha_cases() {
        assert_eq!(shadow_layer_alpha("0 0 40px rgba(0,0,0,.5)"), 0.5);
        assert_eq!(shadow_layer_alpha("0 0 40px"), 1.0);
        assert_eq!(shadow_layer_alpha("0 0 40px transparent"), 0.0);
        assert_eq!(shadow_layer_alpha("0 0 4px #00000080"), 0.5019607843137255);
        assert_eq!(shadow_layer_alpha("inset 0 1px black"), 1.0);
        assert_eq!(shadow_layer_alpha("0 0 4px currentcolor"), 1.0);
    }

    #[test]
    fn shadow_max_blur_px_defaults() {
        assert_eq!(shadow_max_blur_px(None, None), 0.0);
        assert_eq!(
            shadow_max_blur_px(Some("0 0 20px rgba(0,0,0,0.05)"), None),
            20.0
        );
        assert_eq!(
            shadow_max_blur_px(Some("0 0 20px rgba(0,0,0,0.05)"), Some(0.12)),
            0.0
        );
        assert_eq!(
            shadow_max_blur_px(Some("0 1px 2px black, 0 0 30px hsl(200, 50%, 50%)"), None),
            30.0
        );
        assert_eq!(shadow_max_blur_px(Some("0px 0px 10px"), None), 10.0);
    }

    #[test]
    fn css_color_alpha_cases() {
        assert_eq!(css_color_alpha(None), 0.0);
        assert_eq!(css_color_alpha(Some("transparent")), 0.0);
        assert_eq!(css_color_alpha(Some("rgba(0,0,0,0.5)")), 0.5);
        assert_eq!(css_color_alpha(Some("#fff")), 1.0);
        assert_eq!(css_color_alpha(Some("garbage")), 1.0);
    }

    #[test]
    fn border_from_style_cases() {
        let s = style(&[
            ("borderTopWidth", "1px"),
            ("borderRightWidth", "0px"),
            ("borderBottomWidth", "abc"),
            ("borderTopColor", "red"),
        ]);
        assert_eq!(border_widths_from_style(&s), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            border_colors_from_style(&s),
            [
                "red".to_string(),
                String::new(),
                String::new(),
                String::new()
            ]
        );
    }

    #[test]
    fn positioned_style_implies_escape_cases() {
        assert!(positioned_style_implies_escape(&style(&[("top", "-10px")])));
        assert!(positioned_style_implies_escape(&style(&[("left", "100%")])));
        assert!(positioned_style_implies_escape(&style(&[(
            "inset",
            "auto calc(100% + 4px)"
        )])));
        assert!(!positioned_style_implies_escape(&style(&[("top", "10px")])));
        assert!(!positioned_style_implies_escape(&style(&[("left", "50%")])));
        assert!(!positioned_style_implies_escape(&style(&[("left", "")])));
        assert!(!positioned_style_implies_escape(&style(&[])));
        assert!(!positioned_style_implies_escape(&style(&[(
            "left",
            "calc(50%-1px)"
        )])));
    }

    #[test]
    fn metric_length_cases() {
        assert_eq!(metric_length_px(LengthInput::Number(3.5), 16.0), Some(3.5));
        assert_eq!(metric_length_px(LengthInput::Number(f64::NAN), 16.0), None);
        assert_eq!(
            metric_length_px(LengthInput::Number(f64::INFINITY), 16.0),
            None
        );
        assert_eq!(metric_length_px(LengthInput::Text("2em"), 10.0), Some(20.0));
        assert_eq!(metric_length_px(LengthInput::Text("auto"), 10.0), None);
        assert_eq!(metric_length_px(LengthInput::Missing, 10.0), None);
        assert_eq!(
            first_metric_length_px(
                16.0,
                &[
                    LengthInput::Missing,
                    LengthInput::Text("auto"),
                    LengthInput::Text("1px"),
                    LengthInput::Number(9.0)
                ]
            ),
            Some(1.0)
        );
        assert_eq!(first_metric_length_px(16.0, &[]), None);
    }

    #[test]
    fn expand_box_shorthand_cases() {
        assert_eq!(expand_box_shorthand(&["a"]), vec!["a", "a", "a", "a"]);
        assert_eq!(expand_box_shorthand(&["a", "b"]), vec!["a", "b", "a", "b"]);
        assert_eq!(
            expand_box_shorthand(&["a", "b", "c"]),
            vec!["a", "b", "c", "b"]
        );
        assert_eq!(
            expand_box_shorthand(&["a", "b", "c", "d", "e"]),
            vec!["a", "b", "c", "d"]
        );
    }

    #[test]
    fn clipped_by_inset_cases() {
        assert!(clipped_by_inset(Some("inset(50%)")));
        assert!(clipped_by_inset(Some("inset(0% 50% 0% 50%)")));
        assert!(clipped_by_inset(Some("inset(50% 0%)")));
        assert!(clipped_by_inset(Some("INSET(0% 50% 0% 50% round 4px)")));
        assert!(clipped_by_inset(Some("inset(49.5% 0% 50.5%)")));
        // Every value must be a percentage: a unitless 0 fails the whole gate.
        assert!(!clipped_by_inset(Some("inset(100% 0 0 0)")));
        assert!(!clipped_by_inset(Some("inset(50% 0)")));
        assert!(!clipped_by_inset(Some("inset(10% 20%)")));
        assert!(!clipped_by_inset(Some("inset(50% 0% 49%)")));
        assert!(!clipped_by_inset(Some("inset(50px)")));
        assert!(!clipped_by_inset(Some("inset()")));
        assert!(!clipped_by_inset(Some("circle(0)")));
        assert!(!clipped_by_inset(None));
    }

    #[test]
    fn clipped_by_rect_cases() {
        assert!(clipped_by_rect(Some("rect(0 0 0 0)")));
        assert!(clipped_by_rect(Some("rect(0, 0, 0, 0)")));
        assert!(clipped_by_rect(Some("rect(1px, 1px, 1px, 1px)")));
        assert!(!clipped_by_rect(Some("rect(0 10px 10px 0)")));
        assert!(!clipped_by_rect(Some("rect(0 auto auto 0)")));
        assert!(!clipped_by_rect(Some("rect(0 0 0)")));
        assert!(!clipped_by_rect(Some("auto")));
        assert!(!clipped_by_rect(None));
        assert!(clipped_by_rect(Some("rect(0 1em 0 2em)")));
    }

    #[test]
    fn is_screen_reader_only_text_style_cases() {
        assert!(!is_screen_reader_only_text_style(
            None,
            &SrOnlyMetrics::default()
        ));
        let sr = style(&[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
            ("overflow", "hidden"),
        ]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr),
            &SrOnlyMetrics::default()
        ));
        let sr_no_clip = style(&[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
        ]);
        assert!(!is_screen_reader_only_text_style(
            Some(&sr_no_clip),
            &SrOnlyMetrics::default()
        ));
        let sr_clip = style(&[("clip", "rect(0 0 0 0)")]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr_clip),
            &SrOnlyMetrics::default()
        ));
        let sr_clip_path = style(&[("webkitClipPath", "inset(50%)")]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr_clip_path),
            &SrOnlyMetrics::default()
        ));
        let big = style(&[
            ("position", "absolute"),
            ("width", "100px"),
            ("height", "1px"),
            ("overflow", "hidden"),
        ]);
        assert!(!is_screen_reader_only_text_style(
            Some(&big),
            &SrOnlyMetrics::default()
        ));
        // Metrics win over the style widths.
        assert!(is_screen_reader_only_text_style(
            Some(&big),
            &SrOnlyMetrics {
                width: Some(1.0),
                height: Some(1.0),
                ..Default::default()
            }
        ));
        // A 0 font size falls back to 16 for em math.
        let em = style(&[
            ("position", "absolute"),
            ("fontSize", "0px"),
            ("width", "0.1em"),
            ("height", "0.1em"),
            ("overflowY", "clip"),
        ]);
        assert!(is_screen_reader_only_text_style(
            Some(&em),
            &SrOnlyMetrics::default()
        ));
    }

    #[test]
    fn cream_from_class_list_cases() {
        assert_eq!(cream_from_class_list(None), None);
        assert_eq!(cream_from_class_list(Some("")), None);
        assert_eq!(
            cream_from_class_list(Some("min-h-screen bg-amber-50 text-stone-900")),
            Some("bg-amber-50".to_string())
        );
        assert_eq!(cream_from_class_list(Some("bg-stone-50")), None);
        assert_eq!(
            cream_from_class_list(Some("p-4 bg-[#f5f0e6]")),
            Some("bg-[#f5f0e6]".to_string())
        );
        assert_eq!(
            cream_from_class_list(Some("bg-[rgb(245_240_230)]")),
            Some("bg-[rgb(245_240_230)]".to_string())
        );
        assert_eq!(cream_from_class_list(Some("bg-[#ffffff]")), None);
        assert_eq!(cream_from_class_list(Some("bg-amber-500")), None);
        assert_eq!(
            cream_from_class_list(Some("bg-[#ffffff] bg-orange-50")),
            Some("bg-orange-50".to_string())
        );
        assert_eq!(cream_from_class_list(Some("xbg-amber-50")), None);
    }

    #[test]
    fn is_opaque_decorated_box_cases() {
        assert!(!is_opaque_decorated_box(None));
        assert!(is_opaque_decorated_box(Some(&style(&[(
            "backgroundColor",
            "rgb(255, 255, 255)"
        )]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[(
            "backgroundColor",
            "rgba(255, 255, 255, 0.5)"
        )]))));
        assert!(is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgb(0, 0, 0)"),
            ("borderBottomWidth", "1px"),
            ("borderBottomColor", "rgb(0, 0, 0)"),
        ]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgb(0, 0, 0)"),
            ("borderBottomWidth", "0px"),
            ("borderBottomColor", "rgb(0, 0, 0)"),
        ]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgba(0, 0, 0, 0.2)"),
            ("borderBottomWidth", "1px"),
            ("borderBottomColor", "rgba(0, 0, 0, 0.2)"),
        ]))));
    }
}
