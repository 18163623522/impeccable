//! Section 5 per-element browser adapters (`checkElement*DOM`) from
//! `checks.mjs`, plus their DOM-facing helpers. See browser/mod.rs for the
//! full list this module owns.

#![allow(unused_imports)]

use super::background::{
    read_own_background_color, resolve_background_info, resolve_background_info_skipping_images, resolve_gradient_stops, BackgroundInfo,
};
use super::dom::{
    class_attr, class_attr_or_prop, closest_or_none, direct_text, has_direct_text_longer_than,
    matches_or_false, pf0, safe_id, style_px, tag_lower, Dom, ElId, ElStyle, Rect,
};
use super::driver::{browser_colors_close, DesignSystemConfig};
use super::BrowserFinding;
use crate::browser::quality::is_visually_hidden;
use crate::checks::measures::{
    self, border_colors_from_style, border_widths_from_style, check_oversized_h1,
    check_radial_spotlight, gpt_border_shadow_halo_blur_px, gpt_border_shadow_row_finding,
    gpt_border_shadow_row_size, gpt_border_shadow_sizes_match, gpt_thin_border_wide_shadow_pair,
    is_screen_reader_only_text_style, parse_radius_corners, GptBorderShadowInput,
    GptBorderShadowRowTree, OversizedH1Input, RadialSpotlightInput, SrOnlyMetrics,
};
use crate::checks::rules::{
    check_borders, check_colors, check_colors_deduped, check_glow, check_hero_eyebrow,
    check_icon_tile, check_italic_serif, check_motion, check_placeholder_colors,
    check_stripe_child, is_emoji_only_text, is_glyph_only_text, is_rounded_away_from_side,
    text_fill_is_transparent, BorderOpts, ColorOpts, Corners, GlowOpts, HeroEyebrowOpts,
    IconTileOpts, ItalicSerifOpts, MotionOpts, RuleHit, SafeTagTextSeen, Sides, HEADING_TAGS,
};
use crate::checks::text_rules::{
    CURSOR_FIRST_VIEWPORT_PX, CURSOR_GLYPH_RE, POPOVER_LAYER_SELECTOR,
    POSITIONED_CHILD_INTERACTIVE_SELECTOR, TEXT_OVERFLOW_SKIP_TAGS,
};
use crate::color::{
    get_hue, has_chroma, parse_any_color, parse_gradient_colors, parse_rgb, relative_luminance,
    Rgba,
};
use crate::constants::{BORDER_SAFE_TAGS, SAFE_TAGS};
use crate::js::{self, math_round, number_to_string, parse_float, parse_int, WS};
use crate::js_ext_a::num_truthy;
use crate::js_ext_b::utf16_len;
use once_cell::sync::Lazy;
use regex::Regex;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

/// JS `parseRgb(x) || parseAnyColor(x)`.
pub fn parse_rgb_or_any(value: &str) -> Option<Rgba> {
    parse_rgb(Some(value)).or_else(|| parse_any_color(Some(value)))
}

// JS `/(?:^|[\s_-])(?:active|current|selected)(?:$|[\s_-])/i`: ASCII-only
// case folding (`ci`) and the JS `\s` set (`WS`), never Rust `(?i)` / `\s`.
re!(
    ACTIVE_CLASS_RE,
    format!(
        "(?:^|[{ws}_-])(?:{a}|{c}|{s})(?:$|[{ws}_-])",
        ws = js::WS_CHARS,
        a = js::ci("active"),
        c = js::ci("current"),
        s = js::ci("selected")
    )
);

/// JS: checks.mjs#isTabContextElement(el)
pub fn is_tab_context_element(dom: &dyn Dom, el: ElId) -> bool {
    if closest_or_none(
        dom,
        el,
        "[aria-selected=\"true\"], [aria-current]:not([aria-current=\"false\"])",
    )
    .is_some()
    {
        return true;
    }
    let mut cur = Some(el);
    let mut depth = 0;
    while let Some(c) = cur {
        if depth >= 6 {
            break;
        }
        let cls = class_attr_or_prop(dom, c);
        if ACTIVE_CLASS_RE.is_match(&cls) {
            return true;
        }
        cur = dom.parent(c);
        depth += 1;
    }
    false
}

/// JS: checks.mjs#isStatusContextElement(el)
pub fn is_status_context_element(dom: &dyn Dom, el: ElId) -> bool {
    closest_or_none(
        dom,
        el,
        "[role=\"status\"], [role=\"alert\"], [role=\"alertdialog\"], [role=\"log\"], [aria-live=\"polite\"], [aria-live=\"assertive\"]",
    )
    .is_some()
}

pub const SIDES: [&str; 4] = ["Top", "Right", "Bottom", "Left"];

/// JS: checks.mjs#checkElementBordersDOM(el)
pub fn check_element_borders_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if BORDER_SAFE_TAGS.contains(&tag.as_str()) {
        return Vec::new();
    }
    let rect = dom.rect(el);
    if rect.width < 20.0 || rect.height < 20.0 {
        return Vec::new();
    }
    let mut widths = [0.0f64; 4];
    let mut colors: [String; 4] = Default::default();
    for (i, s) in SIDES.iter().enumerate() {
        widths[i] = style_px(dom, el, &format!("border{s}Width"));
        colors[i] = dom.style(el, &format!("border{s}Color"));
    }
    let own_bg = parse_rgb_or_any(&dom.style(el, "backgroundColor"));
    let badge_like = own_bg.map_or(false, |c| c.alpha_or_one() > 0.1);
    let radius_value = dom.style(el, "borderRadius");
    // Only a left or right accent is gated on the corners, so read them out
    // of the one radius value only when one of those sides carries a border.
    let corners = if widths[1] > 0.0 || widths[3] > 0.0 {
        parse_radius_corners(Some(&radius_value), rect.width)
    } else {
        None
    };
    check_borders(
        &tag,
        &Sides {
            top: widths[0],
            right: widths[1],
            bottom: widths[2],
            left: widths[3],
        },
        &Sides {
            top: Some(colors[0].as_str()),
            right: Some(colors[1].as_str()),
            bottom: Some(colors[2].as_str()),
            left: Some(colors[3].as_str()),
        },
        pf0(&radius_value),
        &BorderOpts {
            badge_like,
            status_context: is_status_context_element(dom, el),
            tab_context: is_tab_context_element(dom, el),
            corners,
        },
    )
}

// ── shared helpers ────────────────────────────────────────────────────────

re!(WS_RUN, format!("{}+", WS));

/// JS `s.replace(/\s+/g, ' ')`.
fn collapse_ws(s: &str) -> String {
    WS_RUN.replace_all(s, " ").into_owned()
}

/// JS `Math.round(x)` rendered as `${...}`.
fn round_str(x: f64) -> String {
    number_to_string(math_round(x))
}

/// JS `Math.max(r,g,b) - Math.min(r,g,b)`.
fn spread(c: &Rgba) -> f64 {
    js::math_max3(c.r, c.g, c.b) - js::math_min3(c.r, c.g, c.b)
}

fn finding_hits(v: Vec<measures::Finding>) -> Vec<RuleHit> {
    v.into_iter()
        .map(|f| RuleHit {
            id: f.id,
            snippet: f.snippet,
        })
        .collect()
}

/// JS: checks.mjs#classSelector(el)
pub fn class_selector(dom: &dyn Dom, el: ElId) -> String {
    let cls = class_attr(dom, el);
    let tokens: Vec<&str> = WS_RUN
        .split(js::trim(&cls))
        .filter(|t| !t.is_empty())
        .collect();
    let tag = {
        let t = dom.tag_name(el);
        if t.is_empty() {
            "el".to_string()
        } else {
            js::to_lower_case(&t)
        }
    };
    if tokens.is_empty() {
        tag
    } else {
        format!("{}.{}", tag, tokens.join("."))
    }
}

/// JS: checks.mjs#isRenderedForBrowserRule(el)
pub fn is_rendered_for_browser_rule(dom: &dyn Dom, el: ElId) -> bool {
    let mut cur = Some(el);
    while let Some(c) = cur {
        if dom.attr(c, "aria-hidden").as_deref() == Some("true") {
            return false;
        }
        let visibility = js::to_lower_case(&dom.style(c, "visibility"));
        if dom.style(c, "display") == "none" || visibility == "hidden" || visibility == "collapse"
        {
            return false;
        }
        if style_px(dom, c, "opacity") <= 0.01 {
            return false;
        }
        if js::to_lower_case(&dom.style(c, "contentVisibility")) == "hidden" {
            return false;
        }
        cur = dom.parent(c);
    }
    true
}

/// JS: checks.mjs#effectiveOpacityDOM(el)
pub fn effective_opacity_dom(dom: &dyn Dom, el: ElId) -> f64 {
    let mut o = 1.0f64;
    let mut cur = Some(el);
    while let Some(c) = cur {
        let raw = dom.style(c, "opacity");
        let v = if raw.is_empty() {
            "1".to_string()
        } else {
            raw
        };
        o *= parse_float(&v);
        if o <= 0.02 {
            return 0.0;
        }
        cur = dom.parent(c);
    }
    o
}

// ── pseudo-element stripes / surfaces ─────────────────────────────────────

const PSEUDOS: [&str; 2] = ["::before", "::after"];

/// `getComputedStyle(el, which)` guard: false = the JS `continue`
/// (getComputedStyle threw, returned nothing, or `content` is none/empty).
fn pseudo_present(dom: &dyn Dom, el: ElId, which: &str) -> bool {
    match dom.pseudo_style(el, which, "content") {
        None => false,
        Some(c) => c != "none" && !c.is_empty(),
    }
}

/// JS `parseFloat(ps.x) || 0`.
fn pseudo_px(dom: &dyn Dom, el: ElId, which: &str, prop: &str) -> f64 {
    pf0(&dom.pseudo_style(el, which, prop).unwrap_or_default())
}

fn pseudo_str(dom: &dyn Dom, el: ElId, which: &str, prop: &str) -> String {
    dom.pseudo_style(el, which, prop).unwrap_or_default()
}

// JS `/(?:^|[\s_-])(?:btn|button|link)(?:$|[\s\w_-])/i` (ASCII `\w`).
re!(
    BTN_LINK_CLASS_RE,
    format!(
        "(?:^|[{ws}_-])(?:{b}|{bu}|{l})(?:$|[{ws}A-Za-z0-9_-])",
        ws = js::WS_CHARS,
        b = js::ci("btn"),
        bu = js::ci("button"),
        l = js::ci("link")
    )
);

/// JS: checks.mjs#checkElementPseudoStripeDOM(el)
pub fn check_element_pseudo_stripe_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if BORDER_SAFE_TAGS.contains(&tag.as_str()) || tag == "summary" {
        return Vec::new();
    }
    if closest_or_none(dom, el, "nav, blockquote, pre").is_some() {
        return Vec::new();
    }
    if !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    let rect = dom.rect(el);
    if rect.width < 40.0 || rect.height < 20.0 {
        return Vec::new();
    }
    if is_tab_context_element(dom, el) {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for which in PSEUDOS {
        if !pseudo_present(dom, el, which) {
            continue;
        }
        let position = pseudo_str(dom, el, which, "position");
        if position != "absolute" && position != "fixed" {
            continue;
        }
        if pseudo_px(dom, el, which, "opacity") <= 0.01
            || pseudo_str(dom, el, which, "display") == "none"
        {
            continue;
        }
        let w = pseudo_px(dom, el, which, "width");
        let h = pseudo_px(dom, el, which, "height");
        if !(w > 0.0 && h > 0.0) {
            continue;
        }
        let left = parse_float(&pseudo_str(dom, el, which, "left"));
        let right = parse_float(&pseudo_str(dom, el, which, "right"));
        let top = parse_float(&pseudo_str(dom, el, which, "top"));
        let bottom = parse_float(&pseudo_str(dom, el, which, "bottom"));
        let hugs = |v: f64| v.is_finite() && v >= -2.0 && v <= 2.0;

        let mut edge: Option<&str> = None;
        let mut thickness = 0.0;
        if w >= 3.0 && w <= 12.0 && h >= rect.height - 44.0 && h >= rect.height * 0.5 {
            edge = if hugs(left) {
                Some("left")
            } else if hugs(right) {
                Some("right")
            } else {
                None
            };
            thickness = w;
        }
        if edge.is_none()
            && h >= 3.0
            && h <= 12.0
            && w >= rect.width - 44.0
            && w >= rect.width * 0.5
        {
            let cls = class_attr_or_prop(dom, el);
            if !BTN_LINK_CLASS_RE.is_match(&cls) {
                edge = if hugs(top) {
                    Some("top")
                } else if hugs(bottom) {
                    Some("bottom")
                } else {
                    None
                };
                thickness = h;
            }
        }
        let Some(edge) = edge else { continue };
        // A stripe painted down one side is the card tell only on a rounded
        // card, the same gate the border path applies. Read the corners only
        // once a side stripe is in hand.
        let side_index = match edge {
            "right" => Some(1),
            "left" => Some(3),
            _ => None,
        };
        if let Some(i) = side_index {
            let corners = parse_radius_corners(Some(&dom.style(el, "borderRadius")), rect.width);
            if !is_rounded_away_from_side(corners.as_ref(), i) {
                continue;
            }
        }
        let Some(bg) = parse_rgb_or_any(&pseudo_str(dom, el, which, "backgroundColor")) else {
            continue;
        };
        if bg.alpha_or_one() < 0.1 {
            continue;
        }
        if spread(&bg) < 30.0 {
            continue;
        }
        findings.push(RuleHit::new(
            "side-tab",
            format!(
                "{}{} — absolute {}px pseudo-element stripe ({})",
                class_selector(dom, el),
                which,
                number_to_string(thickness),
                edge
            ),
        ));
    }
    findings
}

const STRIPE_CHILD_SKIP: &str = "nav, blockquote, pre, table, button, a, select, progress, meter, [role=\"progressbar\"], [role=\"slider\"], [role=\"scrollbar\"], [role=\"separator\"], [role=\"tablist\"]";

/// JS: checks.mjs#checkElementStripeChildDOM(el)
pub fn check_element_stripe_child_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if tag != "div" && tag != "span" {
        return Vec::new();
    }
    let Some(host) = dom.parent(el) else {
        return Vec::new();
    };
    let host_tag = tag_lower(dom, host);
    if host_tag == "body" || host_tag == "html" {
        return Vec::new();
    }
    if !dom.children(el).is_empty() {
        return Vec::new();
    }
    if !js::trim(&collapse_ws(&dom.text_content(el))).is_empty() {
        return Vec::new();
    }
    if closest_or_none(dom, el, STRIPE_CHILD_SKIP).is_some() {
        return Vec::new();
    }
    if !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    if is_tab_context_element(dom, el) || is_status_context_element(dom, el) {
        return Vec::new();
    }
    let host_rect = dom.rect(host);
    if host_rect.width < 40.0 || host_rect.height < 20.0 {
        return Vec::new();
    }
    let child_rect = dom.rect(el);
    if child_rect.height < host_rect.height - 44.0 || child_rect.height < host_rect.height * 0.5 {
        return Vec::new();
    }
    let hugs = |v: f64| v.is_finite() && v.abs() <= 3.0;
    let edge = if hugs(child_rect.left - host_rect.left) {
        Some("left")
    } else if hugs(host_rect.right - child_rect.right) {
        Some("right")
    } else {
        None
    };
    let width = child_rect.width;
    let bg = parse_rgb_or_any(&dom.style(el, "backgroundColor"));
    check_stripe_child(&class_selector(dom, el), width, edge, bg)
}

/// JS: checks.mjs#readPseudoSurfaceDOM(el, rect)
pub fn read_pseudo_surface_dom(dom: &dyn Dom, el: ElId, rect: &Rect) -> Option<Rgba> {
    for which in PSEUDOS {
        if !pseudo_present(dom, el, which) {
            continue;
        }
        let position = pseudo_str(dom, el, which, "position");
        if position != "absolute" && position != "fixed" {
            continue;
        }
        // JS `(parseFloat(ps.opacity) || 1) < 0.9`
        let opacity = {
            let n = parse_float(&pseudo_str(dom, el, which, "opacity"));
            if num_truthy(n) {
                n
            } else {
                1.0
            }
        };
        if pseudo_str(dom, el, which, "display") == "none" || opacity < 0.9 {
            continue;
        }
        let w = pseudo_px(dom, el, which, "width");
        let h = pseudo_px(dom, el, which, "height");
        if w < rect.width - 4.0 || h < rect.height - 4.0 {
            continue;
        }
        let Some(bg) = parse_rgb_or_any(&pseudo_str(dom, el, which, "backgroundColor")) else {
            continue;
        };
        if bg.alpha_or_one() < 0.9 {
            continue;
        }
        return Some(bg);
    }
    None
}

// ── colors ────────────────────────────────────────────────────────────────

/// Whether an ancestor carrying direct text is one the contrast pass
/// actually scores, so a descendant sharing its colour can stand down. A
/// SAFE_TAG ancestor is only scored under the same predicate its
/// descendant is, and an ancestor whose own text is an arrow or an icon
/// glyph is not scored at all — `<a><span>Read more</span> →</a>` has to
/// report the span, because nothing reports the anchor.
fn ancestor_scores_its_text(dom: &dyn Dom, el: ElId, direct: &str) -> bool {
    if is_emoji_only_text(direct) {
        return false;
    }
    if !SAFE_TAGS.contains(&tag_lower(dom, el).as_str()) {
        return true;
    }
    !is_glyph_only_text(direct) && !is_visually_hidden(dom, el)
}

/// Whether this element's `color` comes from an ancestor the contrast pass
/// scores on its own, so repeating it here would report one washed-out
/// colour twice. The walk stops at the first ancestor painting a different
/// colour (nothing above it can be the source of this one), at the first
/// one painting a surface of its own without text on it (above that the
/// colour is judged against a different background, which is a different
/// verdict), and at a fixed depth, so it costs a handful of parent hops.
fn inherits_scored_text_color(dom: &dyn Dom, el: ElId, text_color: Option<Rgba>) -> bool {
    const MAX_ANCESTORS: usize = 12;
    let mut cur = dom.parent(el);
    for _ in 0..MAX_ANCESTORS {
        let Some(c) = cur else { return false };
        if parse_rgb_or_any(&dom.style(c, "color")) != text_color {
            return false;
        }
        let direct = direct_text(dom, c);
        if !js::trim(&direct).is_empty() {
            return ancestor_scores_its_text(dom, c, &direct);
        }
        if read_own_background_color(dom, c).map_or(false, |b| b.alpha_or_one() > 0.0) {
            return false;
        }
        cur = dom.parent(c);
    }
    false
}

/// Whether the element sits within the page's own width. A carousel's
/// off-screen slides and an off-canvas drawer are parked beside the page,
/// and the copy of the same label a visitor can actually read is scored
/// where it stands.
fn overlaps_page_width(dom: &dyn Dom, rect: &Rect) -> bool {
    let width = dom.inner_width();
    if !num_truthy(width) {
        return true;
    }
    rect.left < width && rect.left + rect.width > 0.0
}

/// An inactive control. WCAG 1.4.3 exempts them, and a ghost or transparent
/// disabled button is exactly the shape the SAFE_TAGS text path would
/// otherwise start reporting.
const DISABLED_CONTROL_SELECTOR: &str = "[disabled], [aria-disabled=\"true\"]";

/// Whether an ancestor clips its background to text, which makes this run's
/// glyphs part of that ancestor's fill. With a transparent fill the run is
/// already caught by `text_fill_is_transparent`, because the fill inherits;
/// with an opaque one the glyphs cover the gradient, and the background
/// walk still hands the check the gradient's stops as the surface. The
/// static engine asks the same question, and the two answer alike. The walk
/// stops at an ancestor painting an opaque background of its own, a real
/// surface inside the clipped box, and at a fixed depth.
fn text_clipped_by_an_ancestor(dom: &dyn Dom, el: ElId) -> bool {
    const MAX_ANCESTORS: usize = 12;
    let mut cur = dom.parent(el);
    for _ in 0..MAX_ANCESTORS {
        let Some(c) = cur else { return false };
        if js::trim(&dom.style(c, "webkitBackgroundClip")) == "text"
            || js::trim(&dom.style(c, "backgroundClip")) == "text"
        {
            return true;
        }
        if read_own_background_color(dom, c).map_or(false, |b| b.alpha_or_one() >= 0.95) {
            return false;
        }
        cur = dom.parent(c);
    }
    false
}

/// Whether a hit from the SAFE_TAGS text path is one this page should
/// print. Three things waive it, all of them the engine's knowledge rather
/// than the rule's, and all asked here: late, only for an element the rule
/// actually failed, and before the colour pair is registered as this page's
/// one report of itself.
///
/// The first is an author's inline `data-impeccable-ignore`.
///
/// The second is the wrong-layer problem. A link or a span reading over a
/// hero photo, a video, a raster section background or a dark section laid
/// beneath it is scored against whatever fill the ancestor walk could parse,
/// which is not the surface anyone reads it against: the orange that
/// measures 2.5:1 on the section's grey measures 7.7:1 on the photograph
/// actually behind it. Against a picture, or a surface the walk never read
/// and did not name, the verdict is a guess, and a wrong verdict on a real
/// element costs more than a missed one, so this path stays quiet there
/// (`resolved_surface_is_under_text`).
///
/// Those elements are not handed to the visual-contrast pass as candidates
/// either. Its collector takes the first twelve it finds in document order,
/// and a page's links and spans outnumber its headings by an order of
/// magnitude, so admitting them would spend a pixel-reading budget on the
/// smallest text on the page. Widening that pass is its own change, with
/// its own measurement.
///
/// The third is an element not painted at capture (`painted.rs`): a link in
/// a collapsed submenu, an off-screen carousel slide. The driver's paint gate
/// drops its finding after the element pass, but by then the pair would
/// already be claimed, and the visible links wearing the same colour would go
/// unreported.
fn safe_tag_text_hit_stands(
    dom: &dyn Dom,
    el: ElId,
    hit: &RuleHit,
    resolved: Option<Rgba>,
) -> bool {
    !crate::browser::driver::scoped_ignore_active(dom, el, &hit.id)
        && crate::browser::visual::resolved_surface_is_under_text(dom, el, resolved)
        && (crate::browser::painted::paint_gate(&hit.id).is_none()
            || crate::browser::painted::painted_at_capture(dom, el))
}

/// JS: checks.mjs#checkElementColorsDOM(el)
pub fn check_element_colors_dom(
    dom: &dyn Dom,
    el: ElId,
    seen: &mut SafeTagTextSeen,
) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    let rect = dom.rect(el);
    if rect.width < 10.0 || rect.height < 10.0 {
        return Vec::new();
    }
    if dom.style(el, "visibility") == "hidden" || effective_opacity_dom(dom, el) <= 0.02 {
        return Vec::new();
    }
    let direct = direct_text(dom, el);
    let has_direct_text = !js::trim(&direct).is_empty();
    let text_color = parse_rgb_or_any(&dom.style(el, "color"));
    // Only the SAFE_TAGS gate in `check_colors` reads this, so the ancestor
    // walk and the hidden-text selector run only for those tags.
    let paints_own_text = has_direct_text
        && SAFE_TAGS.contains(&tag.as_str())
        && !is_emoji_only_text(&direct)
        && !is_glyph_only_text(&direct)
        && !is_visually_hidden(dom, el)
        && !text_fill_is_transparent(&dom.style(el, "webkitTextFillColor"))
        && !text_clipped_by_an_ancestor(dom, el)
        && overlaps_page_width(dom, &rect)
        // `closest` starts at the element, so the control itself is covered.
        && closest_or_none(dom, el, DISABLED_CONTROL_SELECTOR).is_none()
        && !inherits_scored_text_color(dom, el, text_color);
    // The walk gives up on any raster image, so a link with an external-link
    // mark, or one in a list item with an arrow bullet, reads as unresolved
    // and goes unscored. On the SAFE_TAGS text path an icon is read as
    // absent; everywhere else the walk is what it always was.
    let icons = if paints_own_text {
        crate::browser::visual::icon_hosts(dom, el)
    } else {
        Vec::new()
    };
    let bg_info = if icons.is_empty() {
        resolve_background_info(dom, el)
    } else {
        resolve_background_info_skipping_images(dom, el, &|n| icons.contains(&n))
    };
    let mut effective_bg = bg_info.color;
    let mut surface_unresolved = bg_info.unresolved;
    let mut own_bg = read_own_background_color(dom, el);
    if own_bg.map_or(true, |c| c.alpha_or_one() <= 0.5) {
        if let Some(pseudo_surface) = read_pseudo_surface_dom(dom, el, &rect) {
            own_bg = Some(pseudo_surface);
            effective_bg = Some(pseudo_surface);
            surface_unresolved = false;
        }
    }
    let font_size = {
        let n = parse_float(&dom.style(el, "fontSize"));
        if num_truthy(n) {
            n
        } else {
            16.0
        }
    };
    let font_weight = {
        let n = parse_int(&dom.style(el, "fontWeight"), 10);
        if num_truthy(n) {
            n
        } else {
            400.0
        }
    };
    let bg_clip = {
        let a = dom.style(el, "webkitBackgroundClip");
        if !a.is_empty() {
            a
        } else {
            dom.style(el, "backgroundClip")
        }
    };
    let effective_bg_stops = if surface_unresolved || effective_bg.is_some() {
        None
    } else {
        resolve_gradient_stops(dom, el)
    };
    let color_opts = ColorOpts {
        tag: tag.clone(),
        text_color,
        bg_color: own_bg,
        effective_bg: if surface_unresolved {
            None
        } else {
            effective_bg
        },
        effective_bg_stops,
        font_size,
        font_weight,
        has_direct_text,
        is_emoji_only: is_emoji_only_text(&direct),
        paints_own_text,
        bg_clip: Some(bg_clip),
        bg_image: Some(dom.style(el, "backgroundImage")),
        class_list: Some(class_attr(dom, el)),
        detector_is_browser: true,
    };
    let resolved = color_opts.effective_bg;
    let mut findings = check_colors_deduped(&color_opts, seen, &mut |h: &RuleHit| {
        safe_tag_text_hit_stands(dom, el, h, resolved)
    });
    if tag == "input" || tag == "textarea" {
        let placeholder = dom.attr(el, "placeholder").unwrap_or_default();
        let placeholder = js::trim(&placeholder);
        if !placeholder.is_empty() {
            let skip = if tag == "input" {
                let t = js::to_lower_case(&dom.attr(el, "type").unwrap_or_else(|| "text".into()));
                matches!(
                    t.as_str(),
                    "hidden" | "checkbox" | "radio" | "file" | "submit" | "button" | "image"
                        | "reset" | "range" | "color"
                )
            } else {
                false
            } || !matches_or_false(dom, el, ":placeholder-shown");
            if !skip {
                if let Some(ph_raw) = dom.pseudo_style(el, "::placeholder", "color") {
                    if let Some(ph_color) = parse_rgb_or_any(&ph_raw) {
                        findings.extend(check_placeholder_colors(
                            &color_opts,
                            placeholder,
                            ph_color,
                        ));
                    }
                }
            }
        }
    }
    findings
}

// ── icon tile / italic serif / hero eyebrow ───────────────────────────────

/// JS: checks.mjs#checkElementIconTileDOM(el)
pub fn check_element_icon_tile_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if !HEADING_TAGS.contains(&tag.as_str()) {
        return Vec::new();
    }
    let Some(sibling) = dom.previous_element_sibling(el) else {
        return Vec::new();
    };
    let sib_rect = dom.rect(sibling);
    let head_rect = dom.rect(el);
    let icon_child = dom
        .query_one(
            Some(sibling),
            "svg, i[data-lucide], i[class*=\"fa-\"], i[class*=\"icon\"]",
        )
        .unwrap_or(None);
    let icon_rect = icon_child.map(|c| dom.rect(c));
    let sib_direct = direct_text(dom, sibling);
    let has_inline_emoji_icon =
        dom.children(sibling).is_empty() && is_emoji_only_text(&sib_direct);
    check_icon_tile(&IconTileOpts {
        heading_tag: tag,
        heading_text: Some(dom.text_content(el)),
        heading_top: head_rect.top,
        sibling_tag: Some(tag_lower(dom, sibling)),
        sibling_width: sib_rect.width,
        sibling_height: sib_rect.height,
        sibling_bottom: sib_rect.bottom,
        sibling_bg_color: parse_rgb(Some(&dom.style(sibling, "backgroundColor"))),
        sibling_bg_image: Some(dom.style(sibling, "backgroundImage")),
        sibling_border_width: style_px(dom, sibling, "borderTopWidth"),
        sibling_border_radius: style_px(dom, sibling, "borderRadius"),
        has_icon_child: icon_child.is_some() || has_inline_emoji_icon,
        // JS `iconRect?.width || 0`
        icon_child_width: icon_rect
            .map(|r| r.width)
            .filter(|w| num_truthy(*w))
            .unwrap_or(0.0),
    })
}

/// JS: checks.mjs#checkElementItalicSerifDOM(el)
pub fn check_element_italic_serif_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if tag != "h1" && tag != "h2" {
        return Vec::new();
    }
    // Computed typography belongs to the element that owns the text. A
    // roman heading can contain italic display text (and vice versa).
    let mut pending = vec![el];
    while let Some(node) = pending.pop() {
        if !is_rendered_for_browser_rule(dom, node) {
            continue;
        }
        if !js::trim(&direct_text(dom, node)).is_empty() {
            let hits = check_italic_serif(&ItalicSerifOpts {
                tag: tag.clone(),
                font_style: Some(dom.style(node, "fontStyle")),
                font_family: Some(dom.style(node, "fontFamily")),
                font_size: style_px(dom, node, "fontSize"),
                heading_text: Some(dom.text_content(el)),
            });
            if !hits.is_empty() {
                // Attribute one finding to the heading, even if several
                // descendants contribute italic display text.
                return hits;
            }
        }
        pending.extend(dom.children(node).into_iter().rev());
    }
    Vec::new()
}

/// JS: checks.mjs#domAccentDashPseudo(el)
pub fn dom_accent_dash_pseudo(dom: &dyn Dom, el: ElId) -> bool {
    for which in PSEUDOS {
        if !pseudo_present(dom, el, which) {
            continue;
        }
        let w = pseudo_px(dom, el, which, "width");
        let h = pseudo_px(dom, el, which, "height");
        if !(w >= 8.0 && w <= 80.0 && h >= 1.0 && h <= 6.0) {
            continue;
        }
        let Some(bg) = parse_rgb_or_any(&pseudo_str(dom, el, which, "backgroundColor")) else {
            continue;
        };
        if bg.alpha_or_one() < 0.1 {
            continue;
        }
        if spread(&bg) >= 30.0 {
            return true;
        }
    }
    false
}

/// JS: checks.mjs#checkElementHeroEyebrowDOM(el)
pub fn check_element_hero_eyebrow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if tag != "h1" {
        return Vec::new();
    }
    let Some(sibling) = dom.previous_element_sibling(el) else {
        return Vec::new();
    };
    check_hero_eyebrow(&HeroEyebrowOpts {
        heading_tag: tag,
        heading_text: Some(dom.text_content(el)),
        heading_font_size: style_px(dom, el, "fontSize"),
        heading_in_application_context: closest_or_none(
            dom,
            el,
            "[role=\"tabpanel\"], [role=\"dialog\"], [role=\"application\"], dialog",
        )
        .is_some(),
        sibling_tag: Some(tag_lower(dom, sibling)),
        sibling_text: Some(dom.text_content(sibling)),
        sibling_text_transform: Some(dom.style(sibling, "textTransform")),
        sibling_font_size: style_px(dom, sibling, "fontSize"),
        sibling_letter_spacing: style_px(dom, sibling, "letterSpacing"),
        sibling_font_weight: Some(dom.style(sibling, "fontWeight")),
        sibling_color: Some(dom.style(sibling, "color")),
        sibling_has_accent_dash_pseudo: dom_accent_dash_pseudo(dom, sibling),
    })
}

// ── motion / glow / AI palette ────────────────────────────────────────────

/// JS: checks.mjs#checkElementMotionDOM(el)
pub fn check_element_motion_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if SAFE_TAGS.contains(&tag.as_str()) {
        return Vec::new();
    }
    let timing: Vec<String> = [
        dom.style(el, "animationTimingFunction"),
        dom.style(el, "transitionTimingFunction"),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();
    check_motion(&MotionOpts {
        tag,
        transition_property: Some(dom.style(el, "transitionProperty")),
        animation_name: Some(dom.style(el, "animationName")),
        timing_functions: Some(timing.join(" ")),
        class_list: Some(class_attr(dom, el)),
    })
}

/// The gradient-ancestor average color the glow / AI-palette checks fall
/// back to (JS: the `while (cur ...)` loops in checkElementGlowDOM and
/// checkElementAIPaletteDOM). `{ r, g, b }` without an alpha, as in the JS.
fn gradient_ancestor_average(dom: &dyn Dom, start: Option<ElId>) -> Option<Rgba> {
    let mut cur = start;
    while let Some(c) = cur {
        let bg_image = dom.style(c, "backgroundImage");
        let grad = parse_gradient_colors(Some(&bg_image));
        if !grad.is_empty() {
            let (mut r, mut g, mut b) = (0.0f64, 0.0f64, 0.0f64);
            for col in &grad {
                r += col.r;
                g += col.g;
                b += col.b;
            }
            let n = grad.len() as f64;
            return Some(Rgba {
                r: math_round(r / n),
                g: math_round(g / n),
                b: math_round(b / n),
                a: None,
            });
        }
        cur = dom.parent(c);
    }
    None
}

/// JS: checks.mjs#checkElementGlowDOM(el)
pub fn check_element_glow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let box_shadow = {
        let v = dom.style(el, "boxShadow");
        if !v.is_empty() && v != "none" {
            v
        } else {
            String::new()
        }
    };
    let mut text_shadow = {
        let v = dom.style(el, "textShadow");
        if !v.is_empty() && v != "none" {
            v
        } else {
            String::new()
        }
    };
    let parent = dom.parent(el);
    if !text_shadow.is_empty() {
        if let Some(p) = parent {
            if dom.style(p, "textShadow") == text_shadow {
                text_shadow = String::new();
            }
        }
    }
    if box_shadow.is_empty() && text_shadow.is_empty() {
        return Vec::new();
    }
    let parent_bg_info = resolve_background_info(dom, parent.unwrap_or(el));
    let mut parent_bg = parent_bg_info.color;
    if parent_bg.is_none() && !parent_bg_info.unresolved {
        parent_bg = gradient_ancestor_average(dom, parent);
    }
    let rect = dom.rect(el);
    check_glow(&GlowOpts {
        box_shadow: Some(box_shadow),
        text_shadow: Some(text_shadow),
        effective_bg: parent_bg,
        element_opacity: Some(element_opacity(dom, el)),
        element_size: Some((rect.width, rect.height)),
    })
}

/// A stop painting under this much alpha is a tint over whatever sits behind
/// it, not a palette decision (Tailwind's `/5` and `/10` fills, 0.13 washes).
const AI_PALETTE_MIN_STOP_ALPHA: f64 = 0.15;
/// Past this blur radius a gradient is atmosphere: no edge and no hue
/// survives it as something a visitor reads as a color choice.
const AI_PALETTE_MAX_BLUR_PX: f64 = 24.0;
/// A gradient needs a surface. Hairline rails, 1x2px underline dots and
/// zero-boxed nav chrome paint no palette however they are declared.
const AI_PALETTE_MIN_GRADIENT_SIDE: f64 = 8.0;
const AI_PALETTE_MIN_GRADIENT_AREA: f64 = 256.0;
/// A shorter run of glyphs is a texture or a spacer, not neon type: one bit
/// of binary rain, a non-breaking space holding two icons apart.
const AI_PALETTE_MIN_TEXT_CHARS: usize = 2;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Elements that paint their own pixels over a parent's background.
const OPAQUE_MEDIA_TAGS: [&str; 5] = ["img", "video", "canvas", "picture", "object"];

/// `object-fit` values that fill the box. `contain` and `scale-down`
/// letterbox, so the gradient underneath still shows.
fn object_fit_covers(value: &str) -> bool {
    let v = js::to_lower_case(js::trim(value));
    v.is_empty() || v == "fill" || v == "cover"
}

/// The element whose `object-fit` decides a media child's coverage.
/// `<picture>` is a wrapper: it renders through the `<img>` it holds and
/// `object-fit` never applies to the wrapper, so reading the wrapper's
/// computed `fill` would count a letterboxed image as full coverage. A
/// `<picture>` holding no image paints nothing.
fn media_fit_host(dom: &dyn Dom, el: ElId) -> Option<ElId> {
    if tag_lower(dom, el) != "picture" {
        return Some(el);
    }
    dom.children(el)
        .into_iter()
        .find(|&c| tag_lower(dom, c) == "img")
}

/// A media child hides what is behind it only while it is itself switched on
/// and effectively opaque.
fn media_child_paints(dom: &dyn Dom, el: ElId) -> bool {
    if dom.style(el, "display") == "none" {
        return false;
    }
    let visibility = js::to_lower_case(&dom.style(el, "visibility"));
    if visibility == "hidden" || visibility == "collapse" {
        return false;
    }
    own_opacity(dom, el) >= 0.95
}

/// The visibility model both halves of the rule use, and the page-level
/// accent sweep with them. It climbs the ancestor chain for the switches
/// that hide a subtree outright (`display: none`, `visibility: hidden` /
/// `collapse`) and deliberately leaves out the inherited opacity product: a
/// scroll-reveal wrapper sits at `opacity: 0` in a captured snapshot while
/// its content is exactly what the visitor sees. Opacity is judged per
/// element instead, against the color that element declares.
pub fn ai_palette_is_visible(dom: &dyn Dom, el: ElId) -> bool {
    let mut cur = Some(el);
    while let Some(c) = cur {
        if dom.style(c, "display") == "none" {
            return false;
        }
        let visibility = js::to_lower_case(&dom.style(c, "visibility"));
        if visibility == "hidden" || visibility == "collapse" {
            return false;
        }
        cur = dom.parent(c);
    }
    true
}

/// The element's own `opacity`, defaulting to 1 when the style is absent.
/// Deliberately not the inherited product: the rule scores what this element
/// declares, and an ancestor's animation state is not that.
fn own_opacity(dom: &dyn Dom, el: ElId) -> f64 {
    let raw = dom.style(el, "opacity");
    if raw.is_empty() {
        return 1.0;
    }
    let v = parse_float(&raw);
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

re!(
    BLUR_FN_RE,
    format!(
        "{blur}{ws}*\\({ws}*([0-9.]+)({px}|{rem}|{em})?{ws}*\\)",
        blur = js::ci("blur"),
        px = js::ci("px"),
        rem = js::ci("rem"),
        em = js::ci("em"),
        ws = WS
    )
);

/// The widest `blur()` radius in a filter-shaped value, in px.
fn blur_radius_px(value: &str) -> f64 {
    let mut widest = 0.0f64;
    for m in BLUR_FN_RE.captures_iter(value) {
        let n = parse_float(m.get(1).map(|g| g.as_str()).unwrap_or(""));
        if !n.is_finite() {
            continue;
        }
        let unit = js::to_lower_case(m.get(2).map(|g| g.as_str()).unwrap_or("px"));
        let px = if unit == "rem" || unit == "em" {
            n * 16.0
        } else {
            n
        };
        if px > widest {
            widest = px;
        }
    }
    widest
}

/// The blur that reaches this element's own paint: its `filter` plus every
/// ancestor `filter`, because a blurred wrapper blurs the whole subtree it
/// renders, which is how a glow blob is usually softened. `backdrop-filter`
/// is deliberately not read here: it blurs what sits behind the element,
/// and the element's own background is painted on top of that, sharp.
fn ai_palette_blur_px(dom: &dyn Dom, el: ElId) -> f64 {
    let mut widest = 0.0f64;
    let mut cur = Some(el);
    while let Some(c) = cur {
        let px = blur_radius_px(&dom.style(c, "filter"));
        if px > widest {
            widest = px;
        }
        cur = dom.parent(c);
    }
    widest
}

/// True when a direct child paints over the whole box: the placeholder
/// gradient behind a `position: absolute; inset: 0; object-fit: cover`
/// image is never seen, so its stops are not the page's palette.
fn gradient_occluded_by_media_child(dom: &dyn Dom, el: ElId, rect: &Rect) -> bool {
    for child in dom.children(el) {
        if !OPAQUE_MEDIA_TAGS.contains(&tag_lower(dom, child).as_str()) {
            continue;
        }
        let Some(fit_host) = media_fit_host(dom, child) else {
            continue;
        };
        if !object_fit_covers(&dom.style(fit_host, "objectFit")) {
            continue;
        }
        if !media_child_paints(dom, child) {
            continue;
        }
        let Some(cr) = element_rect(dom, child) else {
            continue;
        };
        let slack = 1.0;
        if cr.left <= rect.left + slack
            && cr.top <= rect.top + slack
            && cr.right >= rect.right - slack
            && cr.bottom >= rect.bottom - slack
        {
            return true;
        }
    }
    false
}

/// The gradient half of the rule: a surface whose ramp is mostly violet or
/// cyan. Stops that paint nothing (too transparent, flat repeats of one
/// color) and surfaces nobody sees (hairlines, heavy blur, covered by an
/// image) never reach the hue test.
fn ai_palette_gradient_hit(
    dom: &dyn Dom,
    el: ElId,
    design_system: Option<&DesignSystemConfig>,
) -> Option<(TellHue, RuleHit)> {
    let bg_image = dom.style(el, "backgroundImage");
    let stops = parse_gradient_colors(Some(&bg_image));
    if stops.len() < 2 {
        return None;
    }
    // Stops that repeat one color exactly are a flat fill written as a
    // gradient. Alpha is part of "one color": a same-hue fade to transparent
    // is a real ramp, and it is how a glow blob is usually written.
    let first = stops[0];
    if stops.iter().all(|c| {
        c.r == first.r
            && c.g == first.g
            && c.b == first.b
            && c.alpha_or_one() == first.alpha_or_one()
    }) {
        return None;
    }
    let rect = element_rect(dom, el)?;
    if rect.width.min(rect.height) < AI_PALETTE_MIN_GRADIENT_SIDE
        || rect.width * rect.height < AI_PALETTE_MIN_GRADIENT_AREA
    {
        return None;
    }
    if ai_palette_blur_px(dom, el) >= AI_PALETTE_MAX_BLUR_PX {
        return None;
    }
    if gradient_occluded_by_media_child(dom, el, &rect) {
        return None;
    }
    let opacity = own_opacity(dom, el);
    let mut painted = 0usize;
    let mut in_band = 0usize;
    let mut tell: Option<TellHue> = None;
    for c in &stops {
        // A stop DESIGN.md declares was picked; it is not the default
        // palette and takes no part in the count.
        if is_declared_design_color(design_system, c) {
            continue;
        }
        if c.alpha_or_one() * opacity < AI_PALETTE_MIN_STOP_ALPHA {
            continue;
        }
        if !has_chroma(Some(c), Some(50.0)) {
            continue;
        }
        painted += 1;
        if let Some(band) = TellHue::of(get_hue(Some(c))) {
            in_band += 1;
            if tell.is_none() {
                tell = Some(band);
            }
        }
    }
    let tell = tell?;
    // One stop grazing a band edge inside an otherwise warm or brand ramp is
    // that ramp's accident, not a violet-to-cyan palette.
    if in_band * 2 < painted {
        return None;
    }
    Some((
        tell,
        RuleHit::new(
            "ai-color-palette",
            format!("{} gradient background", tell.label()),
        ),
    ))
}

/// The neon-text half: a run of glyphs the element paints itself, in band,
/// on a dark surface. SVG geometry, icon wrappers and spacer characters
/// inherit `color` without painting text, and one glyph is a texture.
fn ai_palette_text_hit(
    dom: &dyn Dom,
    el: ElId,
    design_system: Option<&DesignSystemConfig>,
) -> Option<(TellHue, RuleHit)> {
    if dom.namespace_uri(el) == SVG_NS {
        return None;
    }
    // The concatenated direct text, not the longest single node: a
    // typewriter or split-text hero puts every glyph in its own text node
    // and still paints the whole word.
    let text = direct_text(dom, el);
    if utf16_len(js::trim(&text)) < AI_PALETTE_MIN_TEXT_CHARS {
        return None;
    }
    let tc = parse_rgb_or_any(&dom.style(el, "color"))
        .filter(|c| !is_declared_design_color(design_system, c))?;
    if tc.alpha_or_one() * own_opacity(dom, el) < AI_PALETTE_MIN_STOP_ALPHA {
        return None;
    }
    if !has_chroma(Some(&tc), Some(80.0)) {
        return None;
    }
    let tell = TellHue::of(get_hue(Some(&tc)))?;
    let parent = dom.parent(el);
    let parent_bg_info = match parent {
        Some(p) => resolve_background_info(dom, p),
        None => BackgroundInfo {
            color: None,
            unresolved: false,
        },
    };
    let mut effective_bg = parent_bg_info.color;
    if effective_bg.is_none() && !parent_bg_info.unresolved {
        effective_bg = gradient_ancestor_average(dom, parent);
    }
    let bg = effective_bg?;
    if relative_luminance(&bg) >= 0.1 {
        return None;
    }
    Some((
        tell,
        RuleHit::new(
            "ai-color-palette",
            format!("{} neon text on dark background", tell.label()),
        ),
    ))
}

/// The element's own computed `opacity`, `1` when it does not resolve to a
/// number. Ancestor opacity is left out: one style read keeps the glow check
/// at constant cost per element.
fn element_opacity(dom: &dyn Dom, el: ElId) -> f64 {
    let value = js::parse_float(&dom.style(el, "opacity"));
    if value.is_nan() {
        1.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// True when the scan was given a DESIGN.md and that file declares this
/// color as one of the project's own.
///
/// `ai-color-palette` is a rule about the *unchosen* palette: the purple and
/// the cyan a model reaches for when nobody picked one. A color the author
/// wrote down in DESIGN.md was picked, so it is not that default whatever
/// its hue, and a site whose whole palette is its own documented tokens must
/// not trip the rule on every element that wears one. With no design system
/// there is nothing to consult and every color stays in scope, which is the
/// behavior every scan without a DESIGN.md keeps.
///
/// The tolerance is `browser_colors_close`, the same one the
/// `design-system-color` rule matches computed colors with, so a token the
/// design-system rule calls declared is declared here too.
fn is_declared_design_color(ds: Option<&DesignSystemConfig>, c: &Rgba) -> bool {
    let Some(ds) = ds else { return false };
    if !ds.has_colors {
        return false;
    }
    ds.allowed_colors
        .iter()
        .any(|allowed| browser_colors_close(c, allowed))
}

/// The two hues the AI palette is built out of. A page that uses one of them
/// has an accent; a page that uses both has the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TellHue {
    Cyan,
    Purple,
}

impl TellHue {
    /// The band a colour falls in, `None` outside both.
    fn of(hue: f64) -> Option<TellHue> {
        if (160.0..=200.0).contains(&hue) {
            Some(TellHue::Cyan)
        } else if (260.0..=310.0).contains(&hue) {
            Some(TellHue::Purple)
        } else {
            None
        }
    }
    fn label(self) -> &'static str {
        match self {
            TellHue::Cyan => "Cyan",
            TellHue::Purple => "Purple/violet",
        }
    }
}

/// What one element contributes to the AI-palette reading.
#[derive(Debug, Clone, Default)]
pub struct AiPaletteReading {
    /// Charged where they are found: a saturated cyan or purple *gradient* is
    /// the pattern by itself, whatever else the page does.
    pub hits: Vec<RuleHit>,
    /// Neon ink on a near-black ground, held until a second tell hue shows up
    /// somewhere on the page (REN-405).
    pub ink: Option<RuleHit>,
    /// The tell hues this element showed, gradient and ink alike.
    pub tells: Vec<TellHue>,
}

/// JS: checks.mjs#checkElementAIPaletteDOM(el)
///
/// One element's reading. The gradient half answers on its own; the ink half
/// is held for the page pass, because a single saturated hue on a dark ground
/// is how a great many ordinary systems draw their one accent: a teal
/// `#2fb8a6` on near-black lit 18 places on the bench's base, and every one of
/// them was the same deliberate accent (REN-405). Two different tell hues on
/// one page is the palette the rule is named for.
///
/// A colour the scan's DESIGN.md declares is skipped in both halves, and
/// each half keeps its own gates on what actually paints.
pub fn check_element_ai_palette_dom(
    dom: &dyn Dom,
    el: ElId,
    design_system: Option<&DesignSystemConfig>,
) -> AiPaletteReading {
    let mut reading = AiPaletteReading::default();
    // An element with no box paints nothing; see `ai_palette_is_visible` for
    // why the chain above it is read for the display switches only.
    if element_rect(dom, el).is_none() || !ai_palette_is_visible(dom, el) {
        return reading;
    }
    if let Some((tell, hit)) = ai_palette_gradient_hit(dom, el, design_system) {
        reading.tells.push(tell);
        reading.hits.push(hit);
    }
    if let Some((tell, hit)) = ai_palette_text_hit(dom, el, design_system) {
        reading.tells.push(tell);
        reading.ink = Some(hit);
    }
    reading
}

// ── radial spotlight ──────────────────────────────────────────────────────

re!(RADIAL_RE, js::ci("radial-gradient"));
re!(
    INLINE_BG_RE,
    format!(
        "{bg}(?:-{img})?{ws}*:{ws}*([^;]+)",
        bg = js::ci("background"),
        img = js::ci("image"),
        ws = WS
    )
);

/// JS: checks.mjs#elementGradientValue(style, el)
pub fn element_gradient_value(dom: &dyn Dom, el: ElId) -> String {
    let bg_image = {
        let v = dom.style(el, "backgroundImage");
        if !v.is_empty() && v != "none" {
            v
        } else {
            String::new()
        }
    };
    if RADIAL_RE.is_match(&bg_image) {
        return bg_image;
    }
    let bg = dom.style(el, "background");
    if RADIAL_RE.is_match(&bg) {
        return bg;
    }
    let raw_style = dom.attr(el, "style").unwrap_or_default();
    if let Some(m) = INLINE_BG_RE.captures(&raw_style) {
        let v = m.get(1).map(|g| g.as_str()).unwrap_or("");
        if RADIAL_RE.is_match(v) {
            return v.to_string();
        }
    }
    String::new()
}

/// JS: checks.mjs#spotlightLabel(el)
pub fn spotlight_label(dom: &dyn Dom, el: ElId) -> String {
    if let Some(name) = dom.attr(el, "data-name") {
        if !name.is_empty() {
            return name;
        }
    }
    if let Some(id) = dom.id_prop(el) {
        if !id.is_empty() {
            return id;
        }
    }
    if let Some(cls) = dom.class_name_prop(el) {
        let first = WS_RUN
            .split(js::trim(&cls))
            .next()
            .unwrap_or("")
            .to_string();
        if !first.is_empty() {
            return first;
        }
    }
    let t = dom.tag_name(el);
    if t.is_empty() {
        "section".to_string()
    } else {
        js::to_lower_case(&t)
    }
}

/// How many elements one scan may measure for glow-overlapping text. A page
/// with more elements than this has answered the question long before the cap.
const GLOW_TEXT_SCAN_LIMIT: usize = 5000;

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
}

/// The rectangles of every element's own text on the page, measured once per
/// scan and only when a glow first asks. In the browser each measurement is a
/// `Range` and a forced layout, so a page carrying several glow layers must not
/// pay the walk once per layer.
#[derive(Debug, Default)]
pub struct GlowTextRects(std::cell::OnceCell<Vec<Rect>>);

impl GlowTextRects {
    fn rects(&self, dom: &dyn Dom) -> &[Rect] {
        self.0.get_or_init(|| {
            let root = dom.body().or_else(|| dom.document_element());
            dom.query_all(root, "*")
                .unwrap_or_default()
                .into_iter()
                .take(GLOW_TEXT_SCAN_LIMIT)
                .filter_map(|other| dom.direct_text_rect(other))
                .filter(|tr| tr.all_finite() && tr.width > 0.0 && tr.height > 0.0)
                .collect()
        })
    }
}

/// Whether any element's own text paints over the glowing box. Text is what
/// makes a radial wash read as a spotlight; a glow with nothing over it is
/// surface treatment.
fn glow_behind_text(dom: &dyn Dom, rect: &Rect, text: &GlowTextRects) -> bool {
    if !rect.all_finite() || rect.width <= 0.0 || rect.height <= 0.0 {
        return false;
    }
    text.rects(dom).iter().any(|tr| rects_overlap(rect, tr))
}

/// The surface a glow paints on. The glow element's own image layers beneath
/// the glow and its background color come first, then each ancestor's images
/// and color, translucent paint composited over the first opaque surface.
/// `None` only when an image shows through or a color does not parse.
fn glow_backdrop(dom: &dyn Dom, el: ElId, gradient_value: &str) -> Option<Rgba> {
    let mut stack = measures::BackdropStack::default();
    let mut image = Some(measures::radial_spotlight_layers_beneath(gradient_value));
    let mut cur = Some(el);
    while let Some(c) = cur {
        let background_image = image
            .take()
            .unwrap_or_else(|| dom.style(c, "backgroundImage"));
        let raw = dom.style(c, "backgroundColor");
        let mut color = read_own_background_color(dom, c);
        if color.is_none() && js::trim(&raw).eq_ignore_ascii_case("currentcolor") {
            color = crate::color::parse_any_color(Some(&dom.style(c, "color")));
        }
        let declared = !crate::color::is_no_paint_color_value(Some(&raw));
        match stack.paint_element(Some(&background_image), color, declared) {
            measures::BackdropStep::Resolved(surface) => return Some(surface),
            measures::BackdropStep::Unreadable => return None,
            measures::BackdropStep::Continue => {}
        }
        cur = dom.parent(c);
    }
    Some(stack.finish())
}

/// JS: checks.mjs#checkElementRadialSpotlightDOM(el), measuring the page's
/// text afresh. A scan over many elements shares one [`GlowTextRects`]
/// through [`check_element_radial_spotlight_dom_with`].
pub fn check_element_radial_spotlight_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    check_element_radial_spotlight_dom_with(dom, el, &GlowTextRects::default())
}

/// The declaration test of `checkRadialSpotlight`, then the prominence gate,
/// reporting the stop that passed it.
pub fn check_element_radial_spotlight_dom_with(
    dom: &dyn Dom,
    el: ElId,
    text: &GlowTextRects,
) -> Vec<RuleHit> {
    let gradient_value = element_gradient_value(dom, el);
    if gradient_value.is_empty() {
        return Vec::new();
    }
    let stops = measures::radial_spotlight_stops(Some(&gradient_value));
    let rect = dom.rect(el);
    if stops.is_empty() || !measures::radial_spotlight_fits(rect.width, rect.height) {
        return Vec::new();
    }
    let prominence = measures::RadialGlowProminence {
        opacity: effective_opacity_dom(dom, el),
        backdrop: glow_backdrop(dom, el, &gradient_value),
    };
    let Some(stop) = measures::radial_glow_prominent_stop(&stops, &prominence, || {
        glow_behind_text(dom, &rect, text)
    }) else {
        return Vec::new();
    };
    let label = spotlight_label(dom, el);
    finding_hits(vec![measures::radial_spotlight_finding(
        &stop,
        rect.width,
        rect.height,
        Some(&label),
    )])
}

// ── oversized h1 / gpt border shadow ──────────────────────────────────────

/// JS: checks.mjs#checkElementOversizedH1DOM(el)
pub fn check_element_oversized_h1_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if tag != "h1" {
        return Vec::new();
    }
    let font_size = style_px(dom, el, "fontSize");
    let heading_text = collapse_ws(js::trim(&dom.text_content(el)));
    let rect = dom.rect(el);
    let vw = dom.inner_width();
    let vh = dom.inner_height();
    finding_hits(check_oversized_h1(&OversizedH1Input {
        tag: &tag,
        font_size,
        heading_text: &heading_text,
        rect: Some(measures::Rect {
            width: rect.width,
            height: rect.height,
        }),
        viewport_width: if num_truthy(vw) { vw } else { 0.0 },
        viewport_height: if num_truthy(vh) { vh } else { 0.0 },
    }))
}

/// The hairline-and-halo pair of one element, read off its computed style.
/// The halo is measured first: it is one string parse, where the hairlines
/// cost four style reads and two allocations, and a sibling row walk asks
/// this of every box it passes.
fn gpt_border_shadow_pair_dom(dom: &dyn Dom, el: ElId) -> Option<(f64, f64)> {
    let box_shadow = dom.style(el, "boxShadow");
    gpt_border_shadow_halo_blur_px(Some(&box_shadow))?;
    let style = ElStyle { dom, el };
    let widths = border_widths_from_style(&style);
    let colors: Vec<Option<String>> = border_colors_from_style(&style)
        .into_iter()
        .map(Some)
        .collect();
    gpt_thin_border_wide_shadow_pair(&GptBorderShadowInput {
        border_widths: &widths,
        border_colors: Some(&colors),
        box_shadow: Some(&box_shadow),
    })
}

/// Whether `el`, or a wrapper at most
/// [`measures::GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH`] levels above it, is
/// lifted out of the flow the way a popover is.
fn gpt_border_shadow_out_of_flow_dom(dom: &dyn Dom, el: ElId) -> bool {
    let mut current = Some(el);
    for _ in 0..=measures::GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH {
        let Some(node) = current else {
            break;
        };
        let position = js::to_lower_case(&dom.style(node, "position"));
        if position == "absolute" || position == "fixed" {
            return true;
        }
        current = dom.parent(node);
    }
    false
}

/// The laid-out tree a row walk reads in a browser scan: wrappers of one
/// kind share a tag, and two boxes are the same card when their rects are
/// comparable, which also keeps a box with no area, a closed one included,
/// out of every row. A box with area shows at rest unless it is out of the
/// flow and computed hidden, transparent along its ancestors, or parked past
/// the page's left or top edge or the viewport's right edge.
struct DomRowTree<'a> {
    dom: &'a dyn Dom,
}

impl DomRowTree<'_> {
    fn size(&self, el: ElId) -> measures::Rect {
        let r = self.dom.rect(el);
        measures::Rect {
            width: r.width,
            height: r.height,
        }
    }
}

impl GptBorderShadowRowTree for DomRowTree<'_> {
    type El = ElId;
    fn parent(&self, el: &ElId) -> Option<ElId> {
        self.dom.parent(*el)
    }
    fn previous_sibling(&self, el: &ElId) -> Option<ElId> {
        self.dom.previous_element_sibling(*el)
    }
    fn next_sibling(&self, el: &ElId) -> Option<ElId> {
        self.dom.next_element_sibling(*el)
    }
    fn first_child(&self, el: &ElId) -> Option<ElId> {
        self.dom.first_element_child(*el)
    }
    fn same_cell(&self, cell: &ElId, other: &ElId) -> bool {
        self.dom.tag_name(*cell) == self.dom.tag_name(*other)
    }
    fn same_card(&self, card: &ElId, other: &ElId) -> bool {
        gpt_border_shadow_sizes_match(&self.size(*card), &self.size(*other))
    }
    fn carries_pair(&self, el: &ElId) -> bool {
        gpt_border_shadow_pair_dom(self.dom, *el).is_some()
    }
    fn paints_at_rest(&self, el: &ElId) -> bool {
        // Only a box lifted out of the flow can be a popover waiting for its
        // trigger. An in-flow card sitting transparent or offset at scan time
        // is content staged for a scroll reveal, and a visitor sees it by
        // scrolling.
        if !gpt_border_shadow_out_of_flow_dom(self.dom, *el) {
            return true;
        }
        // Computed visibility is inherited, so the element's own value covers
        // a hidden ancestor.
        let visibility = js::to_lower_case(&self.dom.style(*el, "visibility"));
        if visibility == "hidden" || visibility == "collapse" {
            return false;
        }
        if effective_opacity_dom(self.dom, *el) <= 0.02 {
            return false;
        }
        let rect = self.dom.rect(*el);
        let viewport_width = self.dom.inner_width();
        let off_page = rect.right + self.dom.scroll_x() <= 0.0
            || rect.bottom + self.dom.scroll_y() <= 0.0
            || (viewport_width > 0.0 && rect.left >= viewport_width);
        !off_page
    }
}

/// JS: checks.mjs#checkElementGptBorderShadowDOM(el)
pub fn check_element_gpt_border_shadow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    // The row walk is worth paying for only once this element carries the
    // pair itself.
    let Some(pair) = gpt_border_shadow_pair_dom(dom, el) else {
        return Vec::new();
    };
    finding_hits(gpt_border_shadow_row_finding(
        pair,
        gpt_border_shadow_row_size(&DomRowTree { dom }, &el),
    ))
}

// ── clipped overflow container ────────────────────────────────────────────

// JS `\b` is ASCII (`(?-u:\b)`); `/i` folds ASCII only.
re!(
    DECOR_IDENT_RE,
    format!(
        "(?-u:\\b)({})(?-u:\\b)",
        [
            "art", "bg", "background", "badge", "blob", "crop", "decor", "dot", "glow", "grain",
            "image", "mask", "ornament", "overlay", "photo", "scrim", "shadow", "shine", "texture",
        ]
        .iter()
        .map(|w| js::ci(w))
        .collect::<Vec<_>>()
        .join("|")
    )
);
re!(CAROUSEL_ROLE_RE, r"(?-u:\b)(carousel|slider)(?-u:\b)");
re!(
    VIEWPORT_IDENT_RE,
    r"\b(carousel|comparison|compare|fisheye|flickity|marquee|owl|preview|scroller|slider|slideshow|splide|split|swiper|ticker|viewport)\b"
);
re!(DEMO_IDENT_RE, r"\b(demo-area|demo-stage|demo-viewport)\b");

/// JS: checks.mjs#positionedChildHasSubstantiveContent(child)
pub fn positioned_child_has_substantive_content(dom: &dyn Dom, child: ElId) -> bool {
    let text = collapse_ws(&dom.text_content(child));
    if !js::trim(&text).is_empty() {
        return true;
    }
    if matches_or_false(dom, child, POSITIONED_CHILD_INTERACTIVE_SELECTOR) {
        return true;
    }
    if let Ok(Some(_)) = dom.query_one(Some(child), POSITIONED_CHILD_INTERACTIVE_SELECTOR) {
        return true;
    }
    false
}

/// JS: checks.mjs#positionedChildIsDecorative(child)
pub fn positioned_child_is_decorative(dom: &dyn Dom, child: ElId) -> bool {
    if closest_or_none(dom, child, "[aria-hidden=\"true\"]").is_some() {
        return true;
    }
    let role = js::to_lower_case(&dom.attr(child, "role").unwrap_or_default());
    if role == "none" || role == "presentation" {
        return true;
    }
    let tag = tag_lower(dom, child);
    if matches!(tag.as_str(), "img" | "svg" | "canvas" | "video") {
        return true;
    }
    let ident = format!(
        "{} {}",
        dom.attr(child, "class").unwrap_or_default(),
        dom.attr(child, "id").unwrap_or_default()
    );
    if DECOR_IDENT_RE.is_match(&ident) && !positioned_child_has_substantive_content(dom, child) {
        return true;
    }
    false
}

/// A layer the clip would really trap, whatever else it looks like.
pub fn positioned_child_is_popover_layer(dom: &dyn Dom, child: ElId) -> bool {
    matches_or_false(dom, child, POPOVER_LAYER_SELECTOR)
        || matches!(dom.query_one(Some(child), POPOVER_LAYER_SELECTOR), Ok(Some(_)))
}

/// A positioned child that only paints: nothing to read, nothing to click,
/// and either no content of its own, only media, no pointer target, or
/// nothing visible at rest. Builders name these layers with hashed or
/// utility classes, which is why the word list above cannot find them.
pub fn positioned_child_is_ornament(dom: &dyn Dom, child: ElId) -> bool {
    if positioned_child_has_substantive_content(dom, child) {
        return false;
    }
    if dom.style(child, "pointerEvents") == "none" {
        return true;
    }
    // The child's own `opacity`, not the chain's: an ancestor that fades the
    // whole component fades the container too, and says nothing about this
    // layer. A value that does not parse is not a transparent layer.
    let opacity = parse_float(&dom.style(child, "opacity"));
    if opacity.is_finite() && opacity <= 0.05 {
        return true;
    }
    if dom.children(child).is_empty() {
        return true;
    }
    matches!(
        dom.query_one(Some(child), "img,picture,svg,video,canvas"),
        Ok(Some(_))
    )
}

/// JS: checks.mjs#clippingContainerIsIntentionalViewport(el)
pub fn clipping_container_is_intentional_viewport(dom: &dyn Dom, el: ElId) -> bool {
    let role_description =
        js::to_lower_case(&dom.attr(el, "aria-roledescription").unwrap_or_default());
    if CAROUSEL_ROLE_RE.is_match(&role_description) {
        return true;
    }
    if ident_names_viewport(dom, el) {
        return true;
    }
    // A marquee or a rail names the track that moves, not the window that
    // clips it, so the same words count on the immediate scrolling child.
    dom.children(el).iter().any(|&c| ident_names_viewport(dom, c))
}

fn ident_names_viewport(dom: &dyn Dom, el: ElId) -> bool {
    let ident = js::to_lower_case(&format!(
        "{} {}",
        dom.attr(el, "class").unwrap_or_default(),
        dom.attr(el, "id").unwrap_or_default()
    ));
    VIEWPORT_IDENT_RE.is_match(&ident) || DEMO_IDENT_RE.is_match(&ident)
}

/// An element with no principal box (`display: contents`) or no area clips
/// nothing, whatever its overflow says.
pub fn clipping_container_generates_no_box(dom: &dyn Dom, el: ElId) -> bool {
    let display = dom.style(el, "display");
    if display == "contents" || display == "none" {
        return true;
    }
    match element_rect(dom, el) {
        None => true,
        Some(rect) => rect.width <= 0.0 || rect.height <= 0.0,
    }
}

/// The box the whole document sits in. `overflow: hidden` there is the
/// standard guard against sideways scrolling, and nothing can be cut out of
/// a box that is the page.
pub fn clipping_container_is_page_shell(dom: &dyn Dom, el: ElId) -> bool {
    let Some(rect) = element_rect(dom, el) else {
        return false;
    };
    let viewport_width = dom.inner_width();
    if viewport_width <= 0.0 || rect.left > 1.0 || rect.width < viewport_width * 0.98 {
        return false;
    }
    let Some(root) = dom.document_element() else {
        return false;
    };
    let page = dom.rect(root);
    page.height > 0.0 && rect.top <= 1.0 && rect.height >= page.height * 0.98
}

/// `matrix(a, b, c, d, tx, ty)` / `matrix3d(...)` when the transform is
/// nothing but a translation; `None` when it also scales, rotates or skews.
fn transform_translation(transform: &str) -> Option<(f64, f64)> {
    let value = js::trim(transform);
    if value.is_empty() || value == "none" {
        return Some((0.0, 0.0));
    }
    let (kind, rest) = value.split_once('(')?;
    let nums: Vec<f64> = rest
        .trim_end_matches(')')
        .split(',')
        .map(|p| parse_float(js::trim(p)))
        .collect();
    let identity = |v: f64, want: f64| (v - want).abs() <= 0.001;
    match (js::trim(kind), nums.len()) {
        ("matrix", 6) => {
            let ok = identity(nums[0], 1.0)
                && identity(nums[1], 0.0)
                && identity(nums[2], 0.0)
                && identity(nums[3], 1.0);
            ok.then_some((nums[4], nums[5]))
        }
        ("matrix3d", 16) => {
            let linear = [0, 1, 2, 4, 5, 6, 8, 9, 10];
            let ok = linear
                .iter()
                .all(|&i| identity(nums[i], if i % 5 == 0 { 1.0 } else { 0.0 }));
            ok.then_some((nums[12], nums[13]))
        }
        _ => None,
    }
}

/// A masked reveal: the child is a copy no bigger than the box, parked
/// outside it by its own transform. Icon swaps, slide-ins and hover layers
/// all look like this, and the clip is what makes them work.
pub fn positioned_child_is_transform_offset_copy(
    dom: &dyn Dom,
    el: ElId,
    child: ElId,
    clip_x: bool,
    clip_y: bool,
) -> bool {
    let (Some(parent_rect), Some(child_rect)) = (element_rect(dom, el), element_rect(dom, child))
    else {
        return false;
    };
    let Some((tx, ty)) = transform_translation(&dom.style(child, "transform")) else {
        return false;
    };
    if tx == 0.0 && ty == 0.0 {
        return false;
    }
    let threshold = 2.0;
    if child_rect.width > parent_rect.width + threshold
        || child_rect.height > parent_rect.height + threshold
    {
        return false;
    }
    let rested = Rect::from_xywh(
        child_rect.x - tx,
        child_rect.y - ty,
        child_rect.width,
        child_rect.height,
    );
    let out_x = rested.left < parent_rect.left - threshold
        || rested.right > parent_rect.right + threshold;
    let out_y =
        rested.top < parent_rect.top - threshold || rested.bottom > parent_rect.bottom + threshold;
    !(clip_x && out_x) && !(clip_y && out_y)
}

/// JS: checks.mjs#elementRect(el)
pub fn element_rect(dom: &dyn Dom, el: ElId) -> Option<Rect> {
    let rect = dom.rect(el);
    if !rect.all_finite() {
        return None;
    }
    if rect.width <= 0.0 && rect.height <= 0.0 {
        return None;
    }
    Some(rect)
}

/// JS: checks.mjs#positionedChildEscapesClip(el, child, clipX, clipY)
pub fn positioned_child_escapes_clip(
    dom: &dyn Dom,
    el: ElId,
    child: ElId,
    clip_x: bool,
    clip_y: bool,
) -> Option<bool> {
    let parent_rect = element_rect(dom, el)?;
    let child_rect = element_rect(dom, child)?;
    let threshold = 2.0;
    Some(
        (clip_x
            && (child_rect.left < parent_rect.left - threshold
                || child_rect.right > parent_rect.right + threshold))
            || (clip_y
                && (child_rect.top < parent_rect.top - threshold
                    || child_rect.bottom > parent_rect.bottom + threshold)),
    )
}

/// The clipped axes of `el`, or `None` when it is not a clipping container
/// at all (it scrolls, its overflow is visible, or it has no box to clip
/// with).
fn clipped_axes(dom: &dyn Dom, el: ElId) -> Option<(bool, bool)> {
    let clips = |v: &str| v == "hidden" || v == "clip";
    let scrolls = |v: &str| v == "auto" || v == "scroll";
    let ox = dom.style(el, "overflowX");
    let oy = dom.style(el, "overflowY");
    let ov = dom.style(el, "overflow");
    let clip_x = clips(&ox) || clips(&ov);
    let clip_y = clips(&oy) || clips(&ov);
    if (!clip_x && !clip_y) || scrolls(&ox) || scrolls(&oy) || scrolls(&ov) {
        return None;
    }
    if clipping_container_generates_no_box(dom, el) {
        return None;
    }
    Some((clip_x, clip_y))
}

/// Whether `child` is cut by `el`'s clip in a way worth reporting. The
/// container-level exemptions are the caller's; this is the per-child half,
/// so an ancestor can ask the same question about the same child.
fn clip_traps_child(dom: &dyn Dom, el: ElId, child: ElId, clip_x: bool, clip_y: bool) -> bool {
    let escapes = positioned_child_escapes_clip(dom, el, child, clip_x, clip_y);
    if escapes == Some(false) {
        return false;
    }
    if escapes.is_none()
        && !measures::positioned_style_implies_escape_axis(
            &ElStyle { dom, el: child },
            clip_x,
            clip_y,
        )
    {
        return false;
    }
    !positioned_child_is_transform_offset_copy(dom, el, child, clip_x, clip_y)
        || positioned_child_is_popover_layer(dom, child)
}

/// Whether `el` may report a clipped child. The scan only visits the elements
/// in [`super::driver::element_is_scanned`], so a container outside that set
/// can never report anything and nothing may be handed to it: `body {
/// overflow: hidden }` is the common shape, and it is how a page stops
/// sideways scrolling rather than a component cutting a layer.
fn clip_container_can_own_finding(dom: &dyn Dom, el: ElId) -> bool {
    super::driver::element_is_scanned(dom, el)
        && !clipping_container_is_intentional_viewport(dom, el)
        && !clipping_container_is_page_shell(dom, el)
}

/// Nested clips repeat one decision about the same layer. The clip nearest
/// the child is the one that cuts it first and the one whose component the
/// layer belongs to, so an outer container defers to any clipping container
/// between it and the child that traps the same layer.
fn nearer_clip_traps_child(dom: &dyn Dom, el: ElId, child: ElId) -> bool {
    let mut current = dom.parent(child);
    while let Some(inner) = current {
        if inner == el {
            return false;
        }
        if let Some((clip_x, clip_y)) = clipped_axes(dom, inner) {
            if clip_container_can_own_finding(dom, inner)
                && clip_traps_child(dom, inner, child, clip_x, clip_y)
            {
                return true;
            }
        }
        current = dom.parent(inner);
    }
    false
}

/// JS: checks.mjs#checkClippedOverflow(el, style, getStyle)
pub fn check_clipped_overflow(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let Some((clip_x, clip_y)) = clipped_axes(dom, el) else {
        return Vec::new();
    };
    if !clip_container_can_own_finding(dom, el) {
        return Vec::new();
    }
    for child in dom.query_all(Some(el), "*").unwrap_or_default() {
        let pos = dom.style(child, "position");
        if pos != "absolute" && pos != "fixed" {
            continue;
        }
        if positioned_child_is_decorative(dom, child) {
            continue;
        }
        // Cheapest test first: most positioned children are inside the box.
        if !clip_traps_child(dom, el, child, clip_x, clip_y) {
            continue;
        }
        if positioned_child_is_ornament(dom, child) && !positioned_child_is_popover_layer(dom, child)
        {
            continue;
        }
        if nearer_clip_traps_child(dom, el, child) {
            continue;
        }
        return vec![RuleHit::new(
            "clipped-overflow-container",
            format!(
                "{} clips positioned {}",
                class_selector(dom, el),
                class_selector(dom, child)
            ),
        )];
    }
    Vec::new()
}

/// JS: checks.mjs#checkElementClippedOverflowDOM(el)
pub fn check_element_clipped_overflow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    check_clipped_overflow(dom, el)
}

// ── text overflow ─────────────────────────────────────────────────────────

re!(SCROLL_RE, r"(auto|scroll)");

fn is_scroll_region(dom: &dyn Dom, el: ElId) -> bool {
    SCROLL_RE.is_match(&dom.style(el, "overflowX")) || SCROLL_RE.is_match(&dom.style(el, "overflow"))
}

/// JS: checks.mjs#checkElementTextOverflowDOM(el)
pub fn check_element_text_overflow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    if TEXT_OVERFLOW_SKIP_TAGS.contains(&tag.as_str()) {
        return Vec::new();
    }
    if dom.namespace_uri(el) == "http://www.w3.org/2000/svg" {
        return Vec::new();
    }
    if !is_rendered_for_browser_rule(dom, el) {
        return Vec::new();
    }
    if !has_direct_text_longer_than(dom, el, 0) {
        return Vec::new();
    }
    let rect = dom.rect(el);
    let style = ElStyle { dom, el };
    if is_screen_reader_only_text_style(
        Some(&style),
        &SrOnlyMetrics {
            width: Some(rect.width),
            client_width: Some(dom.client_width(el)),
            height: Some(rect.height),
            client_height: Some(dom.client_height(el)),
        },
    ) {
        return Vec::new();
    }
    if is_scroll_region(dom, el) {
        return Vec::new();
    }
    let mut p = dom.parent(el);
    while let Some(pp) = p {
        if is_scroll_region(dom, pp) {
            return Vec::new();
        }
        p = dom.parent(pp);
    }
    let client_width = dom.client_width(el);
    let delta = dom.scroll_width(el) - client_width;
    if client_width > 0.0 && delta >= 16.0 {
        return vec![RuleHit::new(
            "text-overflow",
            format!(
                "{} overflows its box by {}px",
                class_selector(dom, el),
                round_str(delta)
            ),
        )];
    }
    if client_width == 0.0 && rect.width > 0.0 {
        let mut container = dom.parent(el);
        while let Some(c) = container {
            if dom.client_width(c) != 0.0 {
                break;
            }
            container = dom.parent(c);
        }
        let Some(container) = container else {
            return Vec::new();
        };
        let stop = dom.parent(container);
        let mut p = Some(el);
        while let Some(pp) = p {
            if Some(pp) == stop {
                break;
            }
            let t = dom.style(pp, "transform");
            if !t.is_empty() && t != "none" {
                return Vec::new();
            }
            p = dom.parent(pp);
        }
        let c_rect = dom.rect(container);
        let content_right =
            c_rect.left + dom.client_left(container) + dom.client_width(container);
        let spill = rect.right - content_right;
        if spill >= 16.0 {
            return vec![RuleHit::new(
                "text-overflow",
                format!(
                    "{} overflows its container by {}px",
                    class_selector(dom, el),
                    round_str(spill)
                ),
            )];
        }
    }
    Vec::new()
}

// ── blinking cursor ───────────────────────────────────────────────────────

re!(HIDDEN_RE, js::ci("hidden"));
re!(
    BLINK_NAME_RE,
    format!("{}|{}|{}", js::ci("blink"), js::ci("caret"), js::ci("cursor"))
);

/// JS: checks.mjs#keyframesToggleVisibilityDOM(name)
pub fn keyframes_toggle_visibility_dom(dom: &dyn Dom, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let Some(frames) = dom.keyframes(name) else {
        return false;
    };
    let mut toggles_out = false;
    for frame in &frames {
        for (prop, value) in &frame.decls {
            if prop == "opacity" {
                if pf0(value) <= 0.15 {
                    toggles_out = true;
                }
            } else if prop == "visibility" {
                if HIDDEN_RE.is_match(value) {
                    toggles_out = true;
                }
            } else if prop != "animation-timing-function" {
                return false;
            }
        }
    }
    toggles_out
}

/// JS: checks.mjs#checkElementBlinkingCursorDOM(el)
pub fn check_element_blinking_cursor_dom(dom: &dyn Dom, el: ElId) -> Vec<BrowserFinding> {
    let tag = tag_lower(dom, el);
    if matches!(
        tag.as_str(),
        "input" | "textarea" | "select" | "img" | "svg" | "script" | "style"
    ) {
        return Vec::new();
    }
    let iterations: Vec<String> = dom
        .style(el, "animationIterationCount")
        .split(',')
        .map(|s| js::trim(s).to_string())
        .collect();
    if !iterations.iter().any(|s| s == "infinite") {
        return Vec::new();
    }
    let names: Vec<String> = dom
        .style(el, "animationName")
        .split(',')
        .map(|s| js::trim(s).to_string())
        .filter(|n| !n.is_empty() && n != "none")
        .collect();
    if names.is_empty() {
        return Vec::new();
    }
    let blink_name = names
        .iter()
        .find(|n| BLINK_NAME_RE.is_match(n))
        .cloned()
        .or_else(|| {
            names
                .iter()
                .find(|n| keyframes_toggle_visibility_dom(dom, n))
                .cloned()
        });
    let Some(blink_name) = blink_name else {
        return Vec::new();
    };
    if dom.is_content_editable(el)
        || closest_or_none(
            dom,
            el,
            "[contenteditable=\"\"], [contenteditable=\"true\"], [role=\"textbox\"]",
        )
        .is_some()
    {
        return Vec::new();
    }
    let rect = dom.rect(el);
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Vec::new();
    }
    let scroll_y = dom.scroll_y();
    let page_top = rect.top + if num_truthy(scroll_y) { scroll_y } else { 0.0 };
    if page_top > CURSOR_FIRST_VIEWPORT_PX {
        return Vec::new();
    }
    let text = js::trim(&dom.text_content(el)).to_string();
    let glyph_cursor = utf16_len(&text) == 1 && CURSOR_GLYPH_RE.is_match(&text);
    let mut block_cursor = false;
    if !glyph_cursor {
        if !text.is_empty() || !dom.children(el).is_empty() {
            return Vec::new();
        }
        let bg = parse_any_color(Some(&dom.style(el, "backgroundColor")));
        let filled = bg.map_or(false, |b| b.alpha_or_one() > 0.2);
        let has_border_fill = ["Left", "Right", "Bottom"]
            .iter()
            .any(|side| style_px(dom, el, &format!("border{side}Width")) >= 1.0);
        if !filled && !has_border_fill {
            return Vec::new();
        }
        let vertical = rect.width >= 1.0
            && rect.width <= 24.0
            && rect.height >= 6.0
            && rect.height <= 48.0
            && rect.height >= rect.width;
        let underscore =
            rect.height >= 1.0 && rect.height <= 6.0 && rect.width >= 4.0 && rect.width <= 24.0;
        if !vertical && !underscore {
            return Vec::new();
        }
        let radius_px = style_px(dom, el, "borderRadius");
        if radius_px >= 0.4 * js::math_min(rect.width, rect.height) {
            return Vec::new();
        }
        block_cursor = true;
    }
    if !glyph_cursor && !block_cursor {
        return Vec::new();
    }
    let in_hero_region = page_top <= 900.0
        || closest_or_none(
            dom,
            el,
            "header, nav, [role=\"banner\"], [role=\"navigation\"]",
        )
        .is_some();
    vec![BrowserFinding {
        type_: "blinking-cursor".to_string(),
        detail: format!(
            "{} — {}x{}px blinking cursor (animation \"{}\") in the first viewport",
            class_selector(dom, el),
            round_str(rect.width),
            round_str(rect.height),
            blink_name
        ),
        severity: if in_hero_region {
            Some("warning".to_string())
        } else {
            None
        },
        ignore_value: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    /// One element, its own page-level dedupe state.
    fn colors(d: &FakeDom, el: ElId) -> Vec<RuleHit> {
        check_element_colors_dom(d, el, &mut SafeTagTextSeen::default())
    }

    fn page() -> (FakeDom, ElId) {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        for e in [html, body] {
            d.set_styles(
                e,
                &[
                    ("backgroundColor", "rgb(255, 255, 255)"),
                    ("backgroundImage", "none"),
                    ("opacity", "1"),
                    ("display", "block"),
                    ("visibility", "visible"),
                ],
            );
        }
        (d, body)
    }

    fn visible(d: &mut FakeDom, el: ElId) {
        d.set_styles(
            el,
            &[
                ("opacity", "1"),
                ("display", "block"),
                ("visibility", "visible"),
                ("backgroundImage", "none"),
            ],
        );
    }

    #[test]
    fn italic_serif_checks_visible_heading_text_including_inline_children() {
        let (mut d, body) = page();
        let h = d.add(Some(body), "h1");
        d.add_text(h, "Some places stay with ");
        d.set_styles(h, &[("fontStyle", "normal"), ("fontFamily", "Georgia, serif"), ("fontSize", "72px")]);
        let em = d.add(Some(h), "em");
        d.add_text(em, "you");
        d.set_styles(em, &[("fontStyle", "italic"), ("fontFamily", "Georgia, serif"), ("fontSize", "72px")]);
        let hits = check_element_italic_serif_dom(&d, h);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "italic-serif-display");
        // A second decorated word still produces one heading warning.
        let span = d.add(Some(h), "span");
        d.add_text(span, "forever");
        d.set_styles(span, &[("fontStyle", "italic"), ("fontFamily", "Georgia, serif"), ("fontSize", "72px")]);
        assert_eq!(check_element_italic_serif_dom(&d, h).len(), 1);
        d.set_style(span, "fontStyle", "normal");
        for (property, value) in [("display", "none"), ("visibility", "hidden"), ("opacity", "0"), ("fontSize", "24px"), ("fontFamily", "Arial, sans-serif"), ("fontStyle", "normal")] {
            let old = d.style(em, property);
            d.set_style(em, property, value);
            assert!(check_element_italic_serif_dom(&d, h).is_empty(), "{property}: {value}");
            d.set_style(em, property, &old);
        }
        d.set_style(h, "display", "none");
        assert!(check_element_italic_serif_dom(&d, h).is_empty());
    }

    #[test]
    fn italic_serif_uses_text_styles_not_an_overridden_parent() {
        let (mut d, body) = page();
        let h = d.add(Some(body), "h2");
        d.add_text(h, "  ");
        d.set_styles(h, &[("fontStyle", "italic"), ("fontFamily", "Georgia, serif"), ("fontSize", "72px")]);
        let span = d.add(Some(h), "span");
        d.add_text(span, "Roman headline");
        d.set_styles(span, &[("fontStyle", "normal"), ("fontFamily", "Georgia, serif"), ("fontSize", "72px")]);
        assert!(check_element_italic_serif_dom(&d, h).is_empty());
        d.set_style(span, "fontStyle", "italic");
        assert_eq!(check_element_italic_serif_dom(&d, h).len(), 1);
        assert!(check_element_italic_serif_dom(&d, span).is_empty());
    }

    #[test]
    fn side_tab_border_flags_and_active_class_exempts() {
        let mut d = FakeDom::new();
        let (_html, body) = d.with_page();
        let card = d.add(Some(body), "div");
        d.set_rect(card, 0.0, 0.0, 300.0, 100.0);
        d.set_styles(
            card,
            &[
                ("borderTopWidth", "4px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("borderLeftColor", "rgb(0, 0, 0)"),
                ("borderTopColor", "rgb(59, 130, 246)"),
                ("borderRightColor", "rgb(0, 0, 0)"),
                ("borderBottomColor", "rgb(0, 0, 0)"),
                ("borderRadius", "0px"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
            ],
        );
        let hits = check_element_borders_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "side-tab");
        assert_eq!(hits[0].snippet, "border-top: 4px");
        d.set_attr(card, "class", "card is-active");
        assert!(check_element_borders_dom(&d, card).is_empty());
    }

    #[test]
    fn pseudo_stripe_flags_left_stripe_and_skips_neutral() {
        let (mut d, body) = page();
        let card = d.add(Some(body), "div");
        visible(&mut d, card);
        d.set_attr(card, "class", "card feature");
        d.set_rect(card, 0.0, 0.0, 300.0, 120.0);
        d.set_styles(card, &[("borderRadius", "12px")]);
        for (p, v) in [
            ("content", "\"\""),
            ("position", "absolute"),
            ("opacity", "1"),
            ("display", "block"),
            ("width", "4px"),
            ("height", "120px"),
            ("left", "0px"),
            ("right", "296px"),
            ("top", "0px"),
            ("bottom", "0px"),
            ("backgroundColor", "rgb(59, 130, 246)"),
        ] {
            d.set_pseudo_style(card, "::before", p, v);
        }
        let hits = check_element_pseudo_stripe_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].snippet,
            "div.card.feature::before — absolute 4px pseudo-element stripe (left)"
        );
        d.set_pseudo_style(card, "::before", "backgroundColor", "rgb(120, 120, 120)");
        assert!(check_element_pseudo_stripe_dom(&d, card).is_empty());
    }

    #[test]
    fn pseudo_stripe_skips_a_square_host() {
        let (mut d, body) = page();
        let quote = d.add(Some(body), "div");
        visible(&mut d, quote);
        d.set_attr(quote, "class", "pullquote");
        d.set_rect(quote, 0.0, 0.0, 300.0, 120.0);
        d.set_styles(quote, &[("borderRadius", "0px")]);
        for (p, v) in [
            ("content", "\"\""),
            ("position", "absolute"),
            ("opacity", "1"),
            ("display", "block"),
            ("width", "4px"),
            ("height", "120px"),
            ("left", "0px"),
            ("right", "296px"),
            ("top", "0px"),
            ("bottom", "0px"),
            ("backgroundColor", "rgb(59, 130, 246)"),
        ] {
            d.set_pseudo_style(quote, "::before", p, v);
        }
        assert!(check_element_pseudo_stripe_dom(&d, quote).is_empty());
    }

    /// The side accent is the tell only on a rounded card, and the corners
    /// that decide it are the two the stripe does not touch.
    #[test]
    fn side_border_needs_a_radius_away_from_the_stripe() {
        let mut d = FakeDom::new();
        let (_html, body) = d.with_page();
        let card = d.add(Some(body), "div");
        d.set_rect(card, 0.0, 0.0, 300.0, 100.0);
        let with_radius = |d: &mut FakeDom, radius: &str| {
            d.set_styles(
                card,
                &[
                    ("borderTopWidth", "0px"),
                    ("borderRightWidth", "0px"),
                    ("borderBottomWidth", "0px"),
                    ("borderLeftWidth", "4px"),
                    ("borderTopColor", "rgb(0, 0, 0)"),
                    ("borderRightColor", "rgb(0, 0, 0)"),
                    ("borderBottomColor", "rgb(0, 0, 0)"),
                    ("borderLeftColor", "rgb(59, 130, 246)"),
                    ("borderRadius", radius),
                    ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ],
            );
        };

        with_radius(&mut d, "0px");
        assert!(check_element_borders_dom(&d, card).is_empty());

        with_radius(&mut d, "2px");
        assert!(check_element_borders_dom(&d, card).is_empty());

        with_radius(&mut d, "10px");
        let hits = check_element_borders_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "side-tab");
        assert_eq!(hits[0].snippet, "border-left: 4px + border-radius: 10px");

        // Rounded only along the left stripe: the card still reads square.
        with_radius(&mut d, "10px 0px 0px 10px");
        assert!(check_element_borders_dom(&d, card).is_empty());

        // Rounded away from the stripe, square where it runs: the tab shape.
        with_radius(&mut d, "0px 10px 10px 0px");
        let hits = check_element_borders_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "side-tab");
        assert_eq!(hits[0].snippet, "border-left: 4px");

        // A radius the engine cannot read is unknown, not square: a snapshot
        // missing the column, or a value no parser resolves, keeps the find.
        for unreadable in ["", "calc(0.5rem)", "var(--radius)"] {
            with_radius(&mut d, unreadable);
            let hits = check_element_borders_dom(&d, card);
            assert_eq!(hits.len(), 1, "radius {unreadable:?}");
            assert_eq!(hits[0].id, "side-tab");
        }
    }

    #[test]
    fn stripe_child_flags_left_edge_and_skips_neutral_text_and_tab_context() {
        let (mut d, body) = page();
        let host = d.add(Some(body), "div");
        visible(&mut d, host);
        d.set_attr(host, "class", "card");
        d.set_rect(host, 0.0, 0.0, 300.0, 100.0);
        let stripe = d.add(Some(host), "div");
        visible(&mut d, stripe);
        d.set_rect(stripe, 0.0, 0.0, 4.0, 100.0);
        d.set_styles(
            stripe,
            &[
                ("backgroundColor", "rgb(245, 158, 11)"),
                ("width", "4px"),
                ("height", "100px"),
            ],
        );
        let hits = check_element_stripe_child_dom(&d, stripe);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "side-tab");
        assert_eq!(hits[0].snippet, "div — 4px stripe child (left)");
        d.set_styles(stripe, &[("backgroundColor", "rgb(120, 120, 120)")]);
        assert!(check_element_stripe_child_dom(&d, stripe).is_empty());
        let stripe_text = d.add(Some(host), "div");
        visible(&mut d, stripe_text);
        d.set_rect(stripe_text, 4.0, 0.0, 4.0, 100.0);
        d.set_styles(
            stripe_text,
            &[("backgroundColor", "rgb(245, 158, 11)")],
        );
        d.add_text(stripe_text, "x");
        assert!(check_element_stripe_child_dom(&d, stripe_text).is_empty());
        d.set_styles(stripe, &[("backgroundColor", "rgb(245, 158, 11)")]);
        d.set_attr(host, "class", "card is-active");
        assert!(check_element_stripe_child_dom(&d, stripe).is_empty());
    }

    #[test]
    fn placeholder_low_contrast_flags() {
        let (mut d, body) = page();
        let input = d.add(Some(body), "input");
        visible(&mut d, input);
        d.set_attr(input, "placeholder", "Pale Placeholder On White Field");
        d.set_rect(input, 0.0, 0.0, 200.0, 40.0);
        d.set_styles(
            input,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("color", "rgb(0, 0, 0)"),
                ("fontSize", "16px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        d.set_pseudo_style(input, "::placeholder", "color", "rgb(187, 187, 187)");
        d.add_selector(input, ":placeholder-shown");
        let hits = colors(&d, input);
        assert!(
            hits.iter().any(|h| {
                h.id == "low-contrast"
                    && h.snippet
                        .contains("placeholder \"Pale Placeholder On White Field\"")
            }),
            "{hits:?}"
        );
    }

    #[test]
    fn placeholder_skips_when_not_shown() {
        let (mut d, body) = page();
        let input = d.add(Some(body), "input");
        visible(&mut d, input);
        d.set_attr(input, "placeholder", "Pale Placeholder On White Field");
        d.set_attr(input, "value", "");
        d.set_rect(input, 0.0, 0.0, 200.0, 40.0);
        d.set_styles(
            input,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("color", "rgb(0, 0, 0)"),
                ("fontSize", "16px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        d.set_pseudo_style(input, "::placeholder", "color", "rgb(187, 187, 187)");
        let hits = colors(&d, input);
        assert!(
            hits.iter().all(|h| h.id != "low-contrast"),
            "live filled field must not score a hidden placeholder, {hits:?}"
        );
    }

    #[test]
    fn colors_low_contrast_on_resolved_surface_and_pseudo_surface() {
        let (mut d, body) = page();
        let p = d.add(Some(body), "p");
        visible(&mut d, p);
        d.set_rect(p, 0.0, 0.0, 200.0, 40.0);
        d.add_text(p, "Body copy here");
        d.set_styles(
            p,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(200, 200, 200)"),
                ("fontSize", "16px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        let hits = colors(&d, p);
        assert!(hits.iter().any(|h| h.id == "low-contrast"), "{hits:?}");
        // hidden by opacity: nothing
        d.set_style(p, "opacity", "0");
        assert!(colors(&d, p).is_empty());
    }

    /// A link or span with its own washed-out text, inside a wrapper that
    /// carries none, on a white page.
    fn muted_text_in_wrapper(tag: &str, text: &str, color: &str) -> (FakeDom, ElId, ElId) {
        let (mut d, body) = page();
        let wrap = d.add(Some(body), "div");
        visible(&mut d, wrap);
        d.set_rect(wrap, 0.0, 0.0, 300.0, 40.0);
        d.set_styles(
            wrap,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let el = d.add(Some(wrap), tag);
        visible(&mut d, el);
        d.add_text(el, text);
        d.set_rect(el, 0.0, 0.0, 120.0, 20.0);
        d.set_styles(
            el,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", color),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        (d, wrap, el)
    }

    #[test]
    fn plain_link_text_is_scored_against_its_surface() {
        let (d, _wrap, a) = muted_text_in_wrapper("a", "Read more", "rgb(243, 123, 46)");
        let hits = colors(&d, a);
        assert!(
            hits.iter()
                .any(|h| h.id == "low-contrast" && h.snippet.contains("#f37b2e on #ffffff")),
            "{hits:?}"
        );
        // A span nested in a styled anchor is scored against the anchor's fill.
        let (mut d, _wrap, chip) = muted_text_in_wrapper("a", "", "rgb(234, 88, 12)");
        d.set_style(chip, "backgroundColor", "rgb(255, 247, 237)");
        let label = d.add(Some(chip), "span");
        visible(&mut d, label);
        d.add_text(label, "Backed by");
        d.set_rect(label, 0.0, 0.0, 100.0, 20.0);
        d.set_styles(
            label,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(234, 88, 12)"),
                ("fontSize", "14px"),
                ("fontWeight", "500"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        let hits = colors(&d, label);
        assert!(
            hits.iter()
                .any(|h| h.id == "low-contrast" && h.snippet.contains("#ea580c on #fff7ed")),
            "{hits:?}"
        );
    }

    #[test]
    fn safe_tag_text_keeps_its_old_exemptions() {
        // Readable colour: nothing.
        let (d, _wrap, a) = muted_text_in_wrapper("a", "Read more", "rgb(20, 60, 140)");
        assert!(colors(&d, a).is_empty());
        // Icon-font glyph, no reading load.
        let (d, _wrap, icon) = muted_text_in_wrapper("span", "\u{f09a}", "rgb(180, 180, 180)");
        assert!(colors(&d, icon).is_empty());
        // Screen-reader-only text.
        let (mut d, _wrap, sr) =
            muted_text_in_wrapper("span", "Opens a new tab", "rgb(180, 180, 180)");
        d.add_selector(sr, crate::checks::text_rules::SR_ONLY_SELECTOR);
        assert!(colors(&d, sr).is_empty());
        // An empty link has nothing to score.
        let (mut d, _wrap, empty) = muted_text_in_wrapper("a", "", "rgb(180, 180, 180)");
        d.set_rect(empty, 0.0, 0.0, 24.0, 24.0);
        assert!(colors(&d, empty).is_empty());
        // Parked beside the page: the readable copy of the slide is scored.
        let (mut d, _wrap, off) = muted_text_in_wrapper("span", "60%", "rgb(180, 180, 180)");
        d.set_rect(off, 1400.0, 20.0, 60.0, 20.0);
        assert!(colors(&d, off).is_empty());
        // The host heuristics stay behind the tag gate: a purple span heading
        // is still not an ai-color-palette hit.
        let (mut d, _wrap, purple) =
            muted_text_in_wrapper("span", "Ship faster", "rgb(168, 85, 247)");
        d.set_style(purple, "fontSize", "28px");
        let hits = colors(&d, purple);
        assert!(hits.iter().all(|h| h.id == "low-contrast"), "{hits:?}");
    }

    #[test]
    fn inherited_colour_is_reported_once() {
        let (mut d, body) = page();
        let p = d.add(Some(body), "p");
        visible(&mut d, p);
        d.add_text(p, "Terms of service ");
        d.set_rect(p, 0.0, 0.0, 300.0, 20.0);
        d.set_styles(
            p,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(170, 170, 170)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        let run = d.add(Some(p), "span");
        visible(&mut d, run);
        d.add_text(run, "and privacy");
        d.set_rect(run, 0.0, 0.0, 100.0, 20.0);
        d.set_styles(
            run,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(170, 170, 170)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        assert!(colors(&d, p).iter().any(|h| h.id == "low-contrast"));
        assert!(
            colors(&d, run).is_empty(),
            "the paragraph already carries this colour"
        );
        // Its own colour, and it is scored.
        d.set_style(run, "color", "rgb(200, 200, 200)");
        assert!(colors(&d, run).iter().any(|h| h.id == "low-contrast"));
    }

    #[test]
    fn an_ancestor_nothing_scores_does_not_silence_its_run() {
        // `<a><span>Read more</span> →</a>`: the anchor's own text is an
        // arrow, which the glyph exemption drops, so the span has to report.
        let (mut d, _wrap, a) = muted_text_in_wrapper("a", " \u{2192}", "rgb(148, 148, 148)");
        let label = d.add(Some(a), "span");
        visible(&mut d, label);
        d.add_text(label, "Read more about the service");
        d.set_rect(label, 0.0, 0.0, 180.0, 20.0);
        d.set_styles(
            label,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(148, 148, 148)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        assert!(colors(&d, a).is_empty(), "the arrow is not read");
        assert!(
            colors(&d, label)
                .iter()
                .any(|h| h.id == "low-contrast" && h.snippet.contains("#949494 on #ffffff")),
            "nothing above the span reports this colour"
        );
    }

    #[test]
    fn a_run_on_its_own_surface_is_scored_on_that_surface() {
        // White copy on a light section, repeated inside a green card: the
        // ancestor's verdict is a different one, so the run keeps its own.
        let (mut d, body) = page();
        let section = d.add(Some(body), "p");
        visible(&mut d, section);
        d.add_text(section, "Try the demo");
        d.set_rect(section, 0.0, 0.0, 400.0, 20.0);
        d.set_styles(
            section,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(255, 255, 255)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        let card = d.add(Some(section), "div");
        visible(&mut d, card);
        d.set_rect(card, 0.0, 0.0, 200.0, 40.0);
        d.set_styles(
            card,
            &[
                ("backgroundColor", "rgb(22, 163, 74)"),
                ("color", "rgb(255, 255, 255)"),
            ],
        );
        let run = d.add(Some(card), "span");
        visible(&mut d, run);
        d.add_text(run, "Book a slot");
        d.set_rect(run, 0.0, 0.0, 120.0, 20.0);
        d.set_styles(
            run,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(255, 255, 255)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        assert!(
            colors(&d, run)
                .iter()
                .any(|h| h.id == "low-contrast" && h.snippet.contains("#ffffff on #16a34a")),
            "the card's surface is not the section's"
        );
    }

    #[test]
    fn a_background_the_walk_read_as_the_text_colour_is_not_a_report() {
        // White label over a hero photo: the background walk sees through the
        // image to the page's own white and would report 1.0:1.
        let (d, _wrap, label) = muted_text_in_wrapper("span", "EN", "rgb(255, 255, 255)");
        assert!(colors(&d, label).is_empty());
    }

    #[test]
    fn a_disabled_control_is_not_scored() {
        let (mut d, _wrap, button) =
            muted_text_in_wrapper("button", "Generate", "rgb(176, 176, 176)");
        d.add_selector(button, "[disabled]");
        assert!(colors(&d, button).is_empty());
        let (mut d, _wrap, button) =
            muted_text_in_wrapper("button", "Generate", "rgb(176, 176, 176)");
        d.add_selector(button, "[aria-disabled=\"true\"]");
        assert!(colors(&d, button).is_empty());
    }

    /// A nav of `n` links, all in one washed-out colour on the page's white.
    fn nav_of_links(n: usize) -> (FakeDom, ElId, Vec<ElId>) {
        let (mut d, body) = page();
        let nav = d.add(Some(body), "nav");
        visible(&mut d, nav);
        d.set_rect(nav, 0.0, 0.0, 600.0, 40.0);
        d.set_styles(
            nav,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let mut links = Vec::new();
        for i in 0..n {
            let a = d.add(Some(nav), "a");
            visible(&mut d, a);
            d.add_text(a, &format!("Section {i}"));
            d.set_rect(a, (i as f64) * 70.0, 0.0, 60.0, 20.0);
            d.set_styles(
                a,
                &[
                    ("backgroundColor", "rgba(0, 0, 0, 0)"),
                    ("color", "rgb(136, 136, 136)"),
                    ("fontSize", "14px"),
                    ("fontWeight", "400"),
                    ("webkitBackgroundClip", "border-box"),
                ],
            );
            links.push(a);
        }
        (d, nav, links)
    }

    #[test]
    fn a_colour_the_glyphs_are_not_painted_in_is_not_scored() {
        // The gradient heading: a parent clips a gradient to the text and the
        // run fills its own glyphs with nothing, so `color` is a value that
        // renders nowhere and its ratio is a number about nothing.
        let (mut d, _wrap, run) = muted_text_in_wrapper("span", "Ship faster", "rgb(228, 233, 242)");
        assert!(
            !colors(&d, run).is_empty(),
            "control: the same colour scores while it is painted"
        );
        d.set_style(run, "webkitTextFillColor", "rgba(0, 0, 0, 0)");
        assert!(colors(&d, run).is_empty());
        // An opaque fill is the ordinary case and changes nothing.
        d.set_style(run, "webkitTextFillColor", "rgb(228, 233, 242)");
        assert!(!colors(&d, run).is_empty());
    }

    #[test]
    fn an_inline_ignore_waives_its_own_link_and_not_the_page() {
        // The waived link must not spend the page's one report of the pair:
        // an author silencing one link silences one link.
        let (mut d, _nav, links) = nav_of_links(3);
        d.set_attr(links[0], "data-impeccable-ignore", "low-contrast");
        let mut seen = SafeTagTextSeen::default();
        assert!(
            check_element_colors_dom(&d, links[0], &mut seen).is_empty(),
            "the ignored link reports nothing"
        );
        let hits = check_element_colors_dom(&d, links[1], &mut seen);
        assert!(
            hits.iter()
                .any(|h| h.id == "low-contrast" && h.snippet.contains("#888888 on #ffffff")),
            "the next link still carries the page's report, {hits:?}"
        );
        assert!(
            check_element_colors_dom(&d, links[2], &mut seen).is_empty(),
            "and only that one"
        );
    }

    /// An orange link standing in a positioned hero section at `top`, with an
    /// optional photo painted by an earlier sibling across the whole section.
    /// The page itself is white, so without the photo the link reports.
    fn hero_link(top: f64, with_photo: bool) -> (FakeDom, ElId, Option<ElId>, ElId) {
        let (mut d, body) = page();
        let hero = d.add(Some(body), "section");
        visible(&mut d, hero);
        d.set_rect(hero, 0.0, top, 1280.0, 500.0);
        d.set_styles(
            hero,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let photo = with_photo.then(|| {
            let img = d.add(Some(hero), "img");
            visible(&mut d, img);
            d.set_rect(img, 0.0, top, 1280.0, 500.0);
            img
        });
        let a = d.add(Some(hero), "a");
        visible(&mut d, a);
        d.add_text(a, "Read more");
        d.set_rect(a, 100.0, top + 200.0, 160.0, 20.0);
        d.set_styles(
            a,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(243, 123, 46)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        (d, hero, photo, a)
    }

    #[test]
    fn text_over_a_media_layer_is_left_to_the_visual_pass() {
        // A hero photo painted by a positioned sibling is nobody's ancestor,
        // so the background walk reads past it to the page's own white and
        // scores the text against a surface no reader sees.
        let (d, _hero, _photo, a) = hero_link(0.0, false);
        assert!(
            colors(&d, a)
                .iter()
                .any(|h| h.snippet.contains("#f37b2e on #ffffff")),
            "control: with nothing behind it, the link reports"
        );
        let (d, _hero, _photo, a) = hero_link(0.0, true);
        assert!(
            colors(&d, a).is_empty(),
            "the verdict against the page fill is a guess about a photograph"
        );
    }

    #[test]
    fn a_media_layer_below_the_fold_is_still_seen() {
        // The viewport is 800px tall and this hero starts at 1400px. No hit
        // test can be asked there, and none is needed: the photo's rect
        // covers the link's, and it paints first.
        let (d, _hero, _photo, a) = hero_link(1400.0, true);
        assert!(d.inner_height < 1600.0);
        assert!(colors(&d, a).is_empty(), "{:?}", colors(&d, a));
        // A photo that does not reach the text paints nothing under it.
        let (mut d, _hero, photo, a) = hero_link(1400.0, true);
        d.set_rect(photo.unwrap(), 0.0, 1400.0, 80.0, 80.0);
        assert!(
            colors(&d, a)
                .iter()
                .any(|h| h.snippet.contains("#f37b2e on #ffffff")),
            "{:?}",
            colors(&d, a)
        );
    }

    #[test]
    fn an_opaque_card_between_the_photo_and_the_text_is_the_surface() {
        // `#999` on a white card that sits on the hero photo. The card is
        // what the link is read against, and it is scored there.
        let (mut d, hero, _photo, a) = hero_link(0.0, true);
        let card = d.add(Some(hero), "div");
        visible(&mut d, card);
        d.set_rect(card, 100.0, 100.0, 400.0, 200.0);
        d.set_styles(
            card,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let link = d.add(Some(card), "a");
        visible(&mut d, link);
        d.add_text(link, "Read the full story");
        d.set_rect(link, 130.0, 150.0, 160.0, 20.0);
        d.set_styles(
            link,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(153, 153, 153)"),
                ("fontSize", "15px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        assert!(
            colors(&d, link)
                .iter()
                .any(|h| h.snippet.contains("#999999 on #ffffff")),
            "{:?}",
            colors(&d, link)
        );
        // The hero's own link, standing on the photo, stays quiet.
        assert!(colors(&d, a).is_empty());
    }

    /// A 4px inline SVG tile, and the same drawn as a remote file.
    const TILE_SVG: &str = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='4' height='4'%3E%3Crect width='1' height='1' fill='%23f7f7f7'/%3E%3C/svg%3E\")";
    const ICON_SVG: &str = "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10'%3E%3Cpath d='M0 0h10v10' fill='%23999'/%3E%3C/svg%3E\")";

    #[test]
    fn a_solid_section_with_a_texture_tile_is_a_surface() {
        let textured = |image: &str, size: &str, repeat: &str| {
            let (mut d, wrap, a) = muted_text_in_wrapper("a", "Read more", "rgb(157, 157, 157)");
            let shorthand = format!(
                "rgb(255, 255, 255) {image} {repeat} scroll 0% 0% / {size} padding-box border-box"
            );
            d.set_styles(
                wrap,
                &[
                    ("backgroundColor", "rgb(255, 255, 255)"),
                    ("backgroundImage", image),
                    ("backgroundSize", size),
                    ("background", &shorthand),
                ],
            );
            colors(&d, a)
        };
        let hits = textured(TILE_SVG, "auto", "repeat");
        assert!(
            hits.iter().any(|h| h.snippet.contains("#9d9d9d on #ffffff")),
            "{hits:?}"
        );
        let hits = textured("url(\"/noise.png\")", "24px 24px", "repeat");
        assert!(hits.iter().any(|h| h.id == "low-contrast"), "{hits:?}");
        // The shapes a photograph is drawn in are still a picture.
        assert!(textured(TILE_SVG, "cover", "repeat").is_empty());
        assert!(textured(TILE_SVG, "100% auto", "repeat").is_empty());
        assert!(textured(TILE_SVG, "1920px 1080px", "repeat").is_empty());
        assert!(textured("url(\"hero.jpg\")", "auto", "no-repeat").is_empty());
        // A remote file drawn at `auto` has no size the style states, so it
        // is not provably a tile: the green tab image on a near-white list
        // item that reported white label text at 1.1:1.
        assert!(textured("url(\"/tabs-active.png\")", "auto", "repeat").is_empty());
    }

    #[test]
    fn the_hit_test_fallback_stops_at_an_opaque_box() {
        use crate::browser::visual::{layer_under_text, LayerUnder};
        // A transparent document the climb cannot decide, with the layers
        // under the text reachable only by hit tests.
        let stacked = |under: &dyn Fn(ElId, ElId) -> Vec<ElId>| {
            let mut d = FakeDom::new();
            let (html, body) = d.with_page();
            for e in [html, body] {
                visible(&mut d, e);
                d.set_style(e, "backgroundColor", "rgba(0, 0, 0, 0)");
            }
            let wrap = d.add(Some(body), "div");
            visible(&mut d, wrap);
            d.set_rect(wrap, 0.0, 0.0, 300.0, 40.0);
            let a = d.add(Some(wrap), "a");
            visible(&mut d, a);
            d.add_text(a, "Read more");
            d.set_rect(a, 0.0, 0.0, 120.0, 20.0);
            let card = d.add(Some(body), "div");
            visible(&mut d, card);
            d.set_style(card, "backgroundColor", "rgb(255, 255, 255)");
            d.set_rect(card, 0.0, 0.0, 300.0, 40.0);
            let img = d.add(Some(body), "img");
            visible(&mut d, img);
            d.set_rect(img, 0.0, 0.0, 300.0, 40.0);
            for x in [60.0, 30.0, 90.0] {
                let mut stack = vec![a, wrap];
                stack.extend(under(card, img));
                d.set_point(x, 10.0, stack);
            }
            layer_under_text(&d, a)
        };
        assert_eq!(
            stacked(&|_card, img| vec![img]),
            LayerUnder::Picture,
            "a photo under the text"
        );
        assert!(
            matches!(
                stacked(&|card, img| vec![card, img]),
                LayerUnder::Detached(c) if c.r == 255.0 && c.g == 255.0 && c.b == 255.0
            ),
            "the white card covers the photo"
        );
    }

    /// A link with its own washed-out text at a given rect.
    fn muted_link(d: &mut FakeDom, parent: ElId, color: &str, rect: (f64, f64, f64, f64)) -> ElId {
        let a = d.add(Some(parent), "a");
        visible(d, a);
        d.add_text(a, "Read the full story");
        d.set_rect(a, rect.0, rect.1, rect.2, rect.3);
        d.set_styles(
            a,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", color),
                ("fontSize", "15px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        a
    }

    /// A box with no paint of its own.
    fn bare_box(d: &mut FakeDom, parent: ElId, tag: &str, rect: (f64, f64, f64, f64)) -> ElId {
        let el = d.add(Some(parent), tag);
        visible(d, el);
        d.set_rect(el, rect.0, rect.1, rect.2, rect.3);
        d.set_styles(
            el,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        el
    }

    #[test]
    fn a_photo_inside_zero_height_wrappers_is_under_the_text() {
        // `<div class="media"><div class="frame"><img></div></div>` beside the
        // content: both wrappers are zero-height, and the photo is positioned
        // over the whole hero from inside them.
        let hero = |with_photo: bool| {
            let (mut d, body) = page();
            let hero = bare_box(&mut d, body, "section", (0.0, 0.0, 1280.0, 500.0));
            d.set_style(hero, "position", "relative");
            let media = bare_box(&mut d, hero, "div", (0.0, 0.0, 1280.0, 0.0));
            let frame = bare_box(&mut d, media, "div", (0.0, 0.0, 1280.0, 0.0));
            if with_photo {
                let img = bare_box(&mut d, frame, "img", (0.0, 0.0, 1280.0, 500.0));
                d.set_style(img, "position", "absolute");
            }
            let content = bare_box(&mut d, hero, "div", (0.0, 0.0, 1280.0, 500.0));
            d.set_style(content, "position", "relative");
            let a = muted_link(&mut d, content, "rgb(245, 128, 48)", (100.0, 200.0, 420.0, 20.0));
            colors(&d, a)
        };
        assert!(hero(true).is_empty(), "{:?}", hero(true));
        assert!(
            hero(false)
                .iter()
                .any(|h| h.snippet.contains("#f58030 on #ffffff")),
            "control: with no photo the link reports, {:?}",
            hero(false)
        );
    }

    #[test]
    fn a_later_sibling_beneath_the_text_by_z_index_is_under_it() {
        // The photo comes after the content in the markup and is laid beneath
        // it by `z-index`, either its own negative one or the content's
        // positive one.
        let hero = |photo_z: &str, content_position: &str, content_z: &str| {
            let (mut d, body) = page();
            let hero = bare_box(&mut d, body, "section", (0.0, 0.0, 1280.0, 500.0));
            d.set_styles(hero, &[("position", "relative"), ("zIndex", "0")]);
            let content = bare_box(&mut d, hero, "div", (0.0, 0.0, 1280.0, 500.0));
            d.set_styles(content, &[("position", content_position), ("zIndex", content_z)]);
            let a = muted_link(&mut d, content, "rgb(243, 123, 46)", (100.0, 200.0, 420.0, 20.0));
            let img = bare_box(&mut d, hero, "img", (0.0, 0.0, 1280.0, 500.0));
            d.set_styles(img, &[("position", "absolute"), ("zIndex", photo_z)]);
            colors(&d, a)
        };
        assert!(hero("-1", "static", "auto").is_empty());
        assert!(hero("auto", "relative", "1").is_empty());
        // At the same layer the later photo paints over the text, so it is not
        // what the text is read against, and the link is scored on the page.
        assert!(
            hero("auto", "static", "auto")
                .iter()
                .any(|h| h.snippet.contains("#f37b2e on #ffffff")),
            "{:?}",
            hero("auto", "static", "auto")
        );
    }

    #[test]
    fn an_opaque_body_does_not_end_the_search_for_a_photo() {
        // The page paints white, and the photo under the link is somewhere the
        // geometric climb cannot see it. The hit-test stack answers, and stops
        // at the first opaque box.
        let stacked = |under: &dyn Fn(ElId, ElId, ElId) -> Vec<ElId>, top: f64| {
            let (mut d, body) = page();
            let wrap = bare_box(&mut d, body, "div", (0.0, top, 300.0, 40.0));
            let a = muted_link(&mut d, wrap, "rgb(243, 123, 46)", (0.0, top, 120.0, 20.0));
            let white = bare_box(&mut d, body, "div", (0.0, 0.0, 0.0, 0.0));
            d.set_style(white, "backgroundColor", "rgb(255, 255, 255)");
            let dark = bare_box(&mut d, body, "div", (0.0, 0.0, 0.0, 0.0));
            d.set_style(dark, "backgroundColor", "rgb(20, 20, 20)");
            let img = bare_box(&mut d, body, "img", (0.0, 0.0, 0.0, 0.0));
            for x in [60.0, 30.0, 90.0] {
                let mut stack = vec![a, wrap];
                stack.extend(under(white, dark, img));
                stack.extend([body]);
                d.set_point(x, top + 10.0, stack);
            }
            colors(&d, a)
        };
        assert!(stacked(&|_w, _d, img| vec![img], 0.0).is_empty(), "a photo");
        assert!(
            stacked(&|_w, dark, img| vec![dark, img], 0.0).is_empty(),
            "a dark panel the walk never read"
        );
        assert!(
            stacked(&|white, _d, img| vec![white, img], 0.0)
                .iter()
                .any(|h| h.snippet.contains("#f37b2e on #ffffff")),
            "a white panel is the surface the walk named"
        );
        assert!(
            stacked(&|_w, _d, img| vec![img], 1400.0)
                .iter()
                .any(|h| h.snippet.contains("#f37b2e on #ffffff")),
            "below the fold nothing can be asked, and the page's fill stands"
        );
    }

    #[test]
    fn an_icon_beside_the_text_is_not_a_picture() {
        let icon = |image: &str, size: &str, repeat: &str| {
            let (mut d, _wrap, a) =
                muted_text_in_wrapper("a", "Grey external link", "rgb(159, 159, 159)");
            let shorthand = format!(
                "rgba(0, 0, 0, 0) {image} {repeat} scroll 100% 50% / {size} padding-box border-box"
            );
            d.set_styles(
                a,
                &[
                    ("backgroundImage", image),
                    ("backgroundSize", size),
                    ("background", &shorthand),
                ],
            );
            colors(&d, a)
        };
        assert!(
            icon(ICON_SVG, "auto", "no-repeat")
                .iter()
                .any(|h| h.snippet.contains("#9f9f9f on #ffffff")),
            "{:?}",
            icon(ICON_SVG, "auto", "no-repeat")
        );
        assert!(!icon("url(\"/external.png\")", "12px 12px", "no-repeat").is_empty());
        // A picture, or a size nothing states, is still not scored.
        assert!(icon(ICON_SVG, "cover", "no-repeat").is_empty());
        assert!(icon("url(\"/external.png\")", "auto", "no-repeat").is_empty());
        assert!(icon("url(\"/photo.jpg\")", "640px 480px", "no-repeat").is_empty());
        assert!(icon(ICON_SVG, "auto", "repeat").is_empty());

        // An arrow bullet on the list item the link sits in.
        let (mut d, body) = page();
        let ul = bare_box(&mut d, body, "ul", (0.0, 0.0, 600.0, 24.0));
        let li = bare_box(&mut d, ul, "li", (0.0, 0.0, 600.0, 24.0));
        d.set_styles(
            li,
            &[
                ("backgroundImage", ICON_SVG),
                ("backgroundSize", "auto"),
                (
                    "background",
                    "rgba(0, 0, 0, 0) url(\"x\") no-repeat scroll 0% 50% / auto padding-box border-box",
                ),
            ],
        );
        let a = muted_link(&mut d, li, "rgb(158, 158, 158)", (14.0, 2.0, 300.0, 20.0));
        let hits = colors(&d, a);
        assert!(
            hits.iter().any(|h| h.snippet.contains("#9e9e9e on #ffffff")),
            "{hits:?}"
        );
    }

    #[test]
    fn a_section_laid_beneath_the_text_is_the_surface_it_names() {
        // A transparent header at `z-index: 1` over a later `display: contents`
        // section. The walk reads the white wrapper both sit in, and the
        // reader sees the section.
        let header = |section_fill: &str| {
            let (mut d, body) = page();
            let wrapper = bare_box(&mut d, body, "div", (0.0, 0.0, 1280.0, 4000.0));
            d.set_style(wrapper, "backgroundColor", "rgb(255, 255, 255)");
            let header = bare_box(&mut d, wrapper, "div", (0.0, 0.0, 1280.0, 100.0));
            d.set_styles(header, &[("position", "absolute"), ("zIndex", "1")]);
            let a = muted_link(&mut d, header, "rgb(206, 207, 208)", (266.0, 40.0, 79.0, 20.0));
            let contents = bare_box(&mut d, wrapper, "div", (0.0, 0.0, 0.0, 0.0));
            d.set_styles(contents, &[("display", "contents"), ("position", "relative")]);
            let section = bare_box(&mut d, contents, "section", (0.0, 0.0, 1280.0, 592.0));
            d.set_styles(
                section,
                &[("position", "relative"), ("backgroundColor", section_fill)],
            );
            colors(&d, a)
        };
        assert!(header("rgb(10, 16, 21)").is_empty(), "{:?}", header("rgb(10, 16, 21)"));
        assert!(
            header("rgb(255, 255, 255)")
                .iter()
                .any(|h| h.snippet.contains("#cecfd0 on #ffffff")),
            "a section in the colour the walk named is that surface, {:?}",
            header("rgb(255, 255, 255)")
        );
    }

    #[test]
    fn a_carousel_track_narrower_than_its_slides_is_looked_inside() {
        // A translucent counter pill over a slide photo. The track's own rect
        // sits beside the viewport; its slide covers the pill.
        let carousel = |with_photo: bool| {
            let (mut d, body) = page();
            let swiper = bare_box(&mut d, body, "div", (0.0, 106.0, 390.0, 358.0));
            d.set_styles(swiper, &[("position", "relative"), ("zIndex", "1")]);
            let track = bare_box(&mut d, swiper, "div", (-390.0, 106.0, 390.0, 358.0));
            d.set_styles(track, &[("position", "relative"), ("zIndex", "1")]);
            let slide = bare_box(&mut d, track, "div", (16.0, 106.0, 358.0, 358.0));
            if with_photo {
                bare_box(&mut d, slide, "img", (16.0, 106.0, 358.0, 358.0));
            }
            let pill = bare_box(&mut d, swiper, "div", (307.0, 424.0, 51.0, 24.0));
            d.set_styles(
                pill,
                &[
                    ("position", "absolute"),
                    ("zIndex", "1"),
                    ("backgroundColor", "rgba(0, 0, 0, 0.3)"),
                ],
            );
            let count = d.add(Some(pill), "span");
            visible(&mut d, count);
            d.add_text(count, "52");
            d.set_rect(count, 330.0, 429.0, 18.0, 13.0);
            d.set_styles(
                count,
                &[
                    ("backgroundColor", "rgba(0, 0, 0, 0)"),
                    ("color", "rgb(255, 255, 255)"),
                    ("fontSize", "11px"),
                    ("fontWeight", "500"),
                    ("webkitBackgroundClip", "border-box"),
                ],
            );
            colors(&d, count)
        };
        assert!(carousel(true).is_empty(), "{:?}", carousel(true));
        assert!(
            carousel(false)
                .iter()
                .any(|h| h.snippet.contains("#ffffff on #b3b3b3")),
            "{:?}",
            carousel(false)
        );
    }

    #[test]
    fn a_gradient_drawn_larger_than_its_box_shows_one_slice() {
        // An animated button sweeping a 200% gradient across itself: the stops
        // the walk scores are not all under the label at once.
        let button = |size: &str| {
            let (mut d, wrap, label) =
                muted_text_in_wrapper("span", "Install now", "rgb(255, 255, 255)");
            d.set_styles(
                wrap,
                &[
                    (
                        "backgroundImage",
                        "linear-gradient(135deg, rgb(244, 208, 63), rgb(32, 165, 58), rgb(251, 200, 212))",
                    ),
                    ("backgroundSize", size),
                ],
            );
            colors(&d, label)
        };
        assert!(button("200% 200%").is_empty(), "{:?}", button("200% 200%"));
        assert!(!button("auto").is_empty(), "control: the whole gradient is scored");
    }

    #[test]
    fn a_pseudo_element_under_the_text_is_read() {
        let hero = |pseudo: &[(&str, &str)]| {
            let (mut d, body) = page();
            let hero = bare_box(&mut d, body, "section", (0.0, 0.0, 1280.0, 400.0));
            d.set_style(hero, "position", "relative");
            for (prop, value) in pseudo {
                d.set_pseudo_style(hero, "::before", prop, value);
            }
            let content = bare_box(&mut d, hero, "div", (0.0, 0.0, 1280.0, 400.0));
            let a = muted_link(&mut d, content, "rgb(246, 129, 49)", (100.0, 150.0, 400.0, 20.0));
            colors(&d, a)
        };
        let stretched = |extra: (&'static str, &'static str)| {
            vec![
                ("content", "\"\""),
                ("position", "absolute"),
                ("display", "block"),
                ("width", "1280px"),
                ("height", "400px"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("backgroundImage", "none"),
                extra,
            ]
        };
        assert!(hero(&stretched(("backgroundImage", "url(\"hero.jpg\")"))).is_empty());
        assert!(hero(&stretched(("backgroundColor", "rgba(0, 0, 0, 0.45)"))).is_empty());
        assert!(hero(&stretched(("backgroundColor", "rgb(12, 20, 30)"))).is_empty());
        // An underline drawn by the pseudo is not a surface.
        let mut underline = stretched(("backgroundColor", "rgb(12, 20, 30)"));
        underline.push(("height", "2px"));
        assert!(
            hero(&underline)
                .iter()
                .any(|h| h.snippet.contains("#f68131 on #ffffff")),
            "{:?}",
            hero(&underline)
        );
    }

    #[test]
    fn a_run_inside_a_gradient_clipped_parent_is_not_scored() {
        // `<a class="gradlink"><span>Learn more</span></a>`: the parent clips
        // a gradient to the text, so the walk hands the span the gradient's
        // stops as its surface, which nobody reads it against.
        let (mut d, wrap, span) =
            muted_text_in_wrapper("span", "Learn more about it", "rgb(209, 213, 219)");
        d.set_styles(
            wrap,
            &[
                ("color", "rgb(209, 213, 219)"),
                (
                    "backgroundImage",
                    "linear-gradient(90deg, rgb(180, 83, 9), rgb(219, 39, 119))",
                ),
            ],
        );
        assert!(
            !colors(&d, span).is_empty(),
            "control: the gradient is scored while nothing clips it"
        );
        d.set_style(wrap, "webkitBackgroundClip", "text");
        assert!(colors(&d, span).is_empty(), "{:?}", colors(&d, span));
        // A painted box inside the clipped one is a real surface again.
        let (mut d, body) = page();
        let clipped = d.add(Some(body), "div");
        visible(&mut d, clipped);
        d.set_rect(clipped, 0.0, 0.0, 300.0, 40.0);
        d.set_styles(
            clipped,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("backgroundClip", "text"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let card = d.add(Some(clipped), "div");
        visible(&mut d, card);
        d.set_rect(card, 0.0, 0.0, 300.0, 40.0);
        d.set_styles(
            card,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("color", "rgb(17, 17, 17)"),
            ],
        );
        let run = d.add(Some(card), "span");
        visible(&mut d, run);
        d.add_text(run, "Learn more about it");
        d.set_rect(run, 0.0, 0.0, 120.0, 20.0);
        d.set_styles(
            run,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(160, 160, 160)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        assert!(
            colors(&d, run).iter().any(|h| h.id == "low-contrast"),
            "{:?}",
            colors(&d, run)
        );
    }

    #[test]
    fn a_raster_section_background_is_not_the_surface_the_walk_parsed() {
        // The walk answers with the first background colour it can parse. An
        // ancestor that paints an image over its own fill hands it a surface
        // that is covered up in the rendered page.
        let (mut d, wrap, a) = muted_text_in_wrapper("a", "Read more", "rgb(243, 123, 46)");
        d.set_styles(
            wrap,
            &[
                ("backgroundColor", "rgb(246, 247, 248)"),
                ("backgroundImage", "url(\"/hero.jpg\")"),
                ("backgroundSize", "cover"),
            ],
        );
        assert!(colors(&d, a).is_empty(), "{:?}", colors(&d, a));
        // Take the picture away and the same fill is a real surface again.
        d.set_style(wrap, "backgroundImage", "none");
        assert!(colors(&d, a)
            .iter()
            .any(|h| h.snippet.contains("#f37b2e on #f6f7f8")));
    }

    #[test]
    fn a_link_over_a_photo_does_not_spend_the_pages_report() {
        // Same colour twice: once over a photo, once on the page's own fill.
        // The suppressed one must not register the pair.
        let (mut d, hero, _photo, over) = hero_link(0.0, true);
        let body = d.parent(hero).unwrap();
        let plain = d.add(Some(body), "a");
        visible(&mut d, plain);
        d.add_text(plain, "Read more");
        d.set_rect(plain, 100.0, 700.0, 160.0, 20.0);
        d.set_styles(
            plain,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("color", "rgb(243, 123, 46)"),
                ("fontSize", "14px"),
                ("fontWeight", "400"),
                ("webkitBackgroundClip", "border-box"),
            ],
        );
        let mut seen = SafeTagTextSeen::default();
        assert!(check_element_colors_dom(&d, over, &mut seen).is_empty());
        assert!(
            check_element_colors_dom(&d, plain, &mut seen)
                .iter()
                .any(|h| h.id == "low-contrast"),
            "the readable copy of the colour still reports"
        );
    }

    #[test]
    fn a_duplicate_pair_costs_no_engine_work() {
        // The engine's verdict can be a layer walk. A hit the dedupe drops
        // anyway is not worth asking it about.
        let mut seen = SafeTagTextSeen::default();
        let mut calls = 0;
        for _ in 0..3 {
            let mut hits = vec![RuleHit::new(
                "low-contrast",
                "2.8:1 (need 4.5:1), text #999999 on #ffffff".to_string(),
            )];
            seen.keep_first(&mut hits, &mut |_h: &RuleHit| {
                calls += 1;
                true
            });
        }
        assert_eq!(calls, 1);
    }

    #[test]
    fn one_washed_out_colour_is_one_finding_per_page() {
        let (mut d, _nav, links) = nav_of_links(8);
        let mut seen = SafeTagTextSeen::default();
        let total: usize = links
            .iter()
            .map(|a| check_element_colors_dom(&d, *a, &mut seen).len())
            .sum();
        assert_eq!(total, 1, "one colour, one finding");
        // A second colour still reports once of its own.
        d.set_style(links[5], "color", "rgb(153, 153, 153)");
        let mut seen = SafeTagTextSeen::default();
        let total: usize = links
            .iter()
            .map(|a| check_element_colors_dom(&d, *a, &mut seen).len())
            .sum();
        assert_eq!(total, 2);
    }

    #[test]
    fn icon_tile_stack_flags() {
        let (mut d, body) = page();
        let card = d.add(Some(body), "div");
        let tile = d.add(Some(card), "div");
        let svg = d.add(Some(tile), "svg");
        let h3 = d.add(Some(card), "h3");
        d.add_text(h3, "Lightning Fast");
        d.set_rect(tile, 0.0, 0.0, 48.0, 48.0);
        d.set_rect(svg, 12.0, 12.0, 24.0, 24.0);
        d.set_rect(h3, 0.0, 60.0, 200.0, 24.0);
        d.set_styles(
            tile,
            &[
                ("backgroundColor", "rgb(59, 130, 246)"),
                ("backgroundImage", "none"),
                ("borderTopWidth", "0px"),
                ("borderRadius", "8px"),
            ],
        );
        let hits = check_element_icon_tile_dom(&d, h3);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "icon-tile-stack");
        assert!(hits[0].snippet.contains("\"Lightning Fast\""), "{}", hits[0].snippet);
    }

    #[test]
    fn glow_uses_parent_surface_and_ai_palette_reads_gradient() {
        let (mut d, body) = page();
        d.set_style(body, "backgroundColor", "rgb(0, 0, 0)");
        let card = d.add(Some(body), "div");
        visible(&mut d, card);
        d.set_rect(card, 0.0, 0.0, 320.0, 200.0);
        d.set_style(card, "boxShadow", "rgb(59, 130, 246) 0px 4px 20px 0px");
        d.set_style(card, "textShadow", "none");
        d.set_style(card, "backgroundColor", "rgba(0, 0, 0, 0)");
        let hits = check_element_glow_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "dark-glow");
        assert_eq!(hits[0].snippet, "Colored box-shadow glow (#3b82f6) on dark background");

        let hero = d.add(Some(body), "section");
        visible(&mut d, hero);
        d.set_rect(hero, 0.0, 0.0, 800.0, 400.0);
        d.set_style(
            hero,
            "backgroundImage",
            "linear-gradient(rgb(168, 85, 247), rgb(59, 130, 246))",
        );
        d.set_style(hero, "color", "rgb(0, 0, 0)");
        let reading = check_element_ai_palette_dom(&d, hero, None);
        assert_eq!(reading.hits.len(), 1);
        assert_eq!(reading.hits[0].snippet, "Purple/violet gradient background");
        assert!(reading.ink.is_none());
        assert_eq!(reading.tells, vec![TellHue::Purple]);
    }

    /// A DESIGN.md palette built out of the project's own oklch tokens is not
    /// the generic assistant default, however cyan or violet the tokens are.
    /// The verdigris-on-instrument pair here is the shape that fired 80 times
    /// on one site whose whole palette is documented.
    /// Everything one element reports with no design system: its gradient
    /// hit and its held ink, as the per-element rule read before the ink
    /// waited on the page pass.
    fn palette_hits(d: &FakeDom, el: ElId) -> Vec<RuleHit> {
        let reading = check_element_ai_palette_dom(d, el, None);
        reading.hits.into_iter().chain(reading.ink).collect()
    }

    fn design_system_with(colors: &[(f64, f64, f64)]) -> DesignSystemConfig {
        DesignSystemConfig {
            has_colors: true,
            allowed_colors: colors
                .iter()
                .map(|&(r, g, b)| Rgba { r, g, b, a: None })
                .collect(),
            ..DesignSystemConfig::default()
        }
    }

    #[test]
    fn ai_palette_skips_colors_the_design_system_declares() {
        let (mut d, body) = page();
        // oklch(24% 0 0) instrument face, oklch(70% 0.12 188) verdigris text.
        let panel = d.add(Some(body), "div");
        d.set_style(panel, "backgroundColor", "rgb(58, 58, 58)");
        d.set_rect(panel, 0.0, 0.0, 400.0, 80.0);
        let label = d.add(Some(panel), "span");
        d.add_text(label, "Live");
        d.set_rect(label, 24.0, 24.0, 60.0, 20.0);
        d.set_style(label, "color", "rgb(15, 182, 172)");

        // With no DESIGN.md the teal contributes ink to the page-wide reading.
        let reading = check_element_ai_palette_dom(&d, label, None);
        assert!(reading.hits.is_empty());
        assert_eq!(reading.ink.unwrap().snippet, "Cyan neon text on dark background");
        assert_eq!(reading.tells, vec![TellHue::Cyan]);

        // Declared in DESIGN.md, so it is the project's palette, not the default.
        let ds = design_system_with(&[(15.0, 182.0, 172.0)]);
        let declared = check_element_ai_palette_dom(&d, label, Some(&ds));
        assert!(declared.hits.is_empty());
        assert!(declared.ink.is_none());
        assert!(declared.tells.is_empty());

        // A design system that declares some other color leaves the rule alone.
        let other = design_system_with(&[(200.0, 40.0, 30.0)]);
        assert!(check_element_ai_palette_dom(&d, label, Some(&other)).ink.is_some());

        // `hasColors: false` is a DESIGN.md with no palette section: no allowlist
        // to consult, so the rule keeps its unconstrained behavior.
        let empty = DesignSystemConfig::default();
        assert!(check_element_ai_palette_dom(&d, label, Some(&empty)).ink.is_some());
    }

    #[test]
    fn ai_palette_gradient_skips_declared_stops_but_not_undeclared_ones() {
        let (mut d, body) = page();
        let hero = d.add(Some(body), "section");
        d.set_style(
            hero,
            "backgroundImage",
            "linear-gradient(rgb(168, 85, 247), rgb(59, 130, 246))",
        );
        d.set_style(hero, "color", "rgb(0, 0, 0)");
        d.set_rect(hero, 0.0, 0.0, 1280.0, 200.0);

        // The violet stop is a declared token, so this gradient is the project's.
        let ds = design_system_with(&[(168.0, 85.0, 247.0), (59.0, 130.0, 246.0)]);
        let declared = check_element_ai_palette_dom(&d, hero, Some(&ds));
        assert!(declared.hits.is_empty());
        assert!(declared.ink.is_none());
        assert!(declared.tells.is_empty());

        // Declaring only the blue stop leaves the violet one in scope.
        let partial = design_system_with(&[(59.0, 130.0, 246.0)]);
        let reading = check_element_ai_palette_dom(&d, hero, Some(&partial));
        assert_eq!(reading.hits.len(), 1);
        assert_eq!(reading.hits[0].snippet, "Purple/violet gradient background");
        assert!(reading.ink.is_none());
        assert_eq!(reading.tells, vec![TellHue::Purple]);
    }

    /// A surface carrying `gradient`, sized and positioned so only the
    /// gradient gates decide.
    fn gradient_surface(d: &mut FakeDom, parent: ElId, gradient: &str) -> ElId {
        let el = d.add(Some(parent), "div");
        visible(d, el);
        d.set_rect(el, 0.0, 0.0, 320.0, 180.0);
        d.set_styles(
            el,
            &[("backgroundImage", gradient), ("color", "rgb(0, 0, 0)")],
        );
        el
    }

    #[test]
    fn ai_palette_gradient_keeps_the_stock_ramps() {
        let (mut d, body) = page();
        // Cyan to indigo to violet on a CTA: two of three stops in band.
        let cta = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgb(130, 255, 247) 0%, rgb(71, 81, 255) 49%, rgb(133, 38, 254) 100%)",
        );
        d.set_rect(cta, 0.0, 0.0, 186.0, 70.0);
        let hits = palette_hits(&d, cta);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Cyan gradient background");

        // Orange to violet to teal full-bleed hero wash.
        let hero = gradient_surface(
            &mut d,
            body,
            "linear-gradient(135deg, rgb(255, 87, 36) 0%, rgb(192, 88, 243) 50%, rgb(42, 157, 144) 100%)",
        );
        d.set_rect(hero, 0.0, 0.0, 1280.0, 800.0);
        let hits = palette_hits(&d, hero);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");

        // A violet glow blob: one chromatic stop, the other fully transparent.
        let glow = gradient_surface(
            &mut d,
            body,
            "radial-gradient(50% 50%, rgba(133, 38, 254, 0.82) 0%, rgba(171, 171, 171, 0) 100%)",
        );
        d.set_rect(glow, 0.0, 0.0, 158.0, 158.0);
        let hits = palette_hits(&d, glow);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");

        // A 20% violet overlay still paints a visible lavender corner.
        let overlay = gradient_surface(
            &mut d,
            body,
            "linear-gradient(to right top, rgba(192, 88, 243, 0.2), rgba(255, 255, 255, 0.6), rgba(255, 87, 36, 0.25))",
        );
        d.set_rect(overlay, 0.0, 0.0, 1280.0, 800.0);
        let hits = palette_hits(&d, overlay);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");
    }

    #[test]
    fn ai_palette_gradient_skips_what_never_paints() {
        let (mut d, body) = page();

        // A 5% tint reads as flat near-white.
        let tint = gradient_surface(
            &mut d,
            body,
            "linear-gradient(to right bottom, rgba(63, 176, 224, 0.05) 0%, rgba(42, 157, 144, 0.05) 100%)",
        );
        assert!(palette_hits(&d, tint).is_empty());

        // A 120px blur is atmosphere, not a palette.
        let wash = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgb(130, 255, 247), rgb(255, 176, 5))",
        );
        d.set_style(wash, "filter", "blur(120px)");
        assert!(palette_hits(&d, wash).is_empty());

        // A 1px timeline rail is not a surface.
        let rail = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgba(63, 227, 223, 0.35), rgba(96, 165, 250, 0.14))",
        );
        d.set_rect(rail, 0.0, 0.0, 237.0, 1.0);
        assert!(palette_hits(&d, rail).is_empty());

        // One magenta stop grazing the band inside a warm story ring.
        let ring = gradient_surface(
            &mut d,
            body,
            "linear-gradient(rgb(213, 0, 194), rgb(255, 53, 60), rgb(255, 136, 0), rgb(255, 201, 0))",
        );
        d.set_rect(ring, 0.0, 0.0, 84.0, 84.0);
        assert!(palette_hits(&d, ring).is_empty());

        // Two identical stops, alpha included, are a flat fill written as a
        // gradient.
        let flat = gradient_surface(
            &mut d,
            body,
            "linear-gradient(270deg, rgb(77, 20, 140) 0%, rgb(77, 20, 140) 100%)",
        );
        assert!(palette_hits(&d, flat).is_empty());
        d.set_style(
            flat,
            "backgroundImage",
            "linear-gradient(270deg, rgba(77, 20, 140, 0.4) 0%, rgba(77, 20, 140, 0.4) 100%)",
        );
        assert!(palette_hits(&d, flat).is_empty());

        // display:none nav chrome with a zero box.
        let hidden = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgb(0, 159, 219), rgb(130, 255, 247))",
        );
        d.set_style(hidden, "display", "none");
        assert!(palette_hits(&d, hidden).is_empty());

        // A `visibility: hidden` ancestor hides the subtree for good.
        let shell = d.add(Some(body), "div");
        visible(&mut d, shell);
        d.set_rect(shell, 0.0, 0.0, 320.0, 180.0);
        d.set_style(shell, "visibility", "hidden");
        let offscreen = gradient_surface(
            &mut d,
            shell,
            "linear-gradient(90deg, rgb(0, 159, 219), rgb(130, 255, 247))",
        );
        assert!(palette_hits(&d, offscreen).is_empty());
    }

    #[test]
    fn ai_palette_gradient_reads_the_blur_of_the_whole_chain() {
        let (mut d, body) = page();
        // The blob idiom: the wrapper carries the blur, the child the ramp.
        let wrapper = d.add(Some(body), "div");
        visible(&mut d, wrapper);
        d.set_rect(wrapper, 0.0, 0.0, 400.0, 400.0);
        let blob = gradient_surface(
            &mut d,
            wrapper,
            "radial-gradient(rgba(133, 38, 254, 0.82), rgba(133, 38, 254, 0) 100%)",
        );
        assert_eq!(palette_hits(&d, blob).len(), 1);
        d.set_style(wrapper, "filter", "blur(120px)");
        assert!(palette_hits(&d, blob).is_empty());

        // `backdrop-filter` blurs what is behind the element; the element's
        // own background is painted on top of it, sharp.
        d.set_style(wrapper, "filter", "none");
        d.set_style(blob, "backdropFilter", "blur(120px)");
        assert_eq!(palette_hits(&d, blob).len(), 1);
    }

    #[test]
    fn ai_palette_gradient_keeps_a_same_color_alpha_fade() {
        let (mut d, body) = page();
        // The stock violet glow: one color fading out. Same r/g/b in every
        // stop, so only the alpha tells it apart from a flat fill.
        let glow = gradient_surface(
            &mut d,
            body,
            "radial-gradient(circle, rgba(168, 85, 247, 0.8) 0%, rgba(168, 85, 247, 0) 100%)",
        );
        let hits = palette_hits(&d, glow);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");
    }

    #[test]
    fn ai_palette_keeps_what_a_scroll_reveal_wrapper_holds() {
        let (mut d, body) = page();
        // A captured snapshot freezes the reveal at opacity 0; the visitor
        // sees the content the moment it scrolls in.
        let reveal = d.add(Some(body), "div");
        visible(&mut d, reveal);
        d.set_rect(reveal, 0.0, 0.0, 320.0, 180.0);
        d.set_style(reveal, "opacity", "0");
        let hero = gradient_surface(
            &mut d,
            reveal,
            "linear-gradient(90deg, rgb(168, 85, 247), rgb(130, 255, 247))",
        );
        assert_eq!(palette_hits(&d, hero).len(), 1);
    }

    #[test]
    fn ai_palette_gradient_skips_a_placeholder_under_its_image() {
        let (mut d, body) = page();
        let fill = gradient_surface(
            &mut d,
            body,
            "linear-gradient(150deg, rgb(168, 200, 232), rgb(167, 229, 211))",
        );
        d.set_rect(fill, 0.0, 0.0, 120.0, 213.0);
        assert_eq!(palette_hits(&d, fill).len(), 1);

        let img = d.add(Some(fill), "img");
        visible(&mut d, img);
        d.set_rect(img, 0.0, 0.0, 120.0, 213.0);
        d.set_style(img, "objectFit", "cover");
        assert!(palette_hits(&d, fill).is_empty());

        // A contained image letterboxes, so the gradient still shows.
        d.set_style(img, "objectFit", "contain");
        assert_eq!(palette_hits(&d, fill).len(), 1);

        // A hidden image covers nothing.
        d.set_style(img, "objectFit", "cover");
        d.set_style(img, "visibility", "hidden");
        assert_eq!(palette_hits(&d, fill).len(), 1);
        d.set_style(img, "visibility", "visible");
        assert!(palette_hits(&d, fill).is_empty());
    }

    #[test]
    fn ai_palette_reads_object_fit_through_a_picture() {
        let (mut d, body) = page();
        let fill = gradient_surface(
            &mut d,
            body,
            "linear-gradient(150deg, rgb(168, 85, 247), rgb(130, 255, 247))",
        );
        d.set_rect(fill, 0.0, 0.0, 120.0, 213.0);

        // `object-fit` is the image's property, never the wrapper's, so a
        // `<picture>` around a letterboxed image is not full coverage.
        let picture = d.add(Some(fill), "picture");
        visible(&mut d, picture);
        d.set_rect(picture, 0.0, 0.0, 120.0, 213.0);
        let inner = d.add(Some(picture), "img");
        visible(&mut d, inner);
        d.set_rect(inner, 0.0, 0.0, 120.0, 213.0);
        d.set_style(inner, "objectFit", "contain");
        assert_eq!(palette_hits(&d, fill).len(), 1);

        d.set_style(inner, "objectFit", "cover");
        assert!(palette_hits(&d, fill).is_empty());
    }

    /// A cyan run of text on a black page: only the element under test varies.
    fn neon_text_host(d: &mut FakeDom, body: ElId, tag: &str, text: &str) -> ElId {
        d.set_style(body, "backgroundColor", "rgb(0, 0, 0)");
        let el = d.add(Some(body), tag);
        visible(d, el);
        d.set_rect(el, 0.0, 0.0, 120.0, 30.0);
        d.set_styles(
            el,
            &[
                ("color", "rgb(130, 255, 247)"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
            ],
        );
        if !text.is_empty() {
            d.add_text(el, text);
        }
        el
    }

    #[test]
    fn ai_palette_neon_text_needs_painted_glyphs() {
        let (mut d, body) = page();
        let heading = neon_text_host(&mut d, body, "h3", "Download");
        let hits = palette_hits(&d, heading);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Cyan neon text on dark background");

        // An icon wrapper carries the color but paints no text.
        let wrapper = neon_text_host(&mut d, body, "div", "");
        assert!(palette_hits(&d, wrapper).is_empty());

        // SVG geometry inherits currentColor from the icon above it. Each
        // host is given real text so the namespace is the only thing that
        // can stop it: an <svg> reports once per shape otherwise.
        for tag in ["svg", "path", "circle", "line", "g"] {
            let shape = neon_text_host(&mut d, body, tag, "Download");
            assert!(
                palette_hits(&d, shape).is_empty(),
                "<{tag}> reported"
            );
        }

        // A single glyph in a binary-rain texture is not neon text.
        let bit = neon_text_host(&mut d, body, "span", "1");
        d.set_rect(bit, 0.0, 0.0, 6.6, 11.0);
        assert!(palette_hits(&d, bit).is_empty());

        // A whitespace-only span paints nothing either.
        let spacer = neon_text_host(&mut d, body, "span", " ");
        assert!(palette_hits(&d, spacer).is_empty());

        // A typewriter hero splits the word into one text node per glyph and
        // paints every one of them.
        let typed = neon_text_host(&mut d, body, "span", "");
        for glyph in ["I", "m", "a", "g", "e"] {
            d.add_text(typed, glyph);
        }
        let hits = palette_hits(&d, typed);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Cyan neon text on dark background");
    }

    #[test]
    fn glow_reads_element_opacity_and_size() {
        let (mut d, body) = page();
        d.set_style(body, "backgroundColor", "rgb(10, 10, 14)");

        // A blinking caret: 3x23px, half faded, with a halo bigger than it is.
        let caret = d.add(Some(body), "span");
        visible(&mut d, caret);
        d.set_style(caret, "opacity", "0.56");
        d.set_rect(caret, 120.0, 40.0, 3.0, 23.0);
        d.set_style(caret, "boxShadow", "rgba(155, 123, 232, 0.38) 0px 0px 9.9px 1.5px");
        d.set_style(caret, "textShadow", "none");
        assert!(check_element_glow_dom(&d, caret).is_empty());

        // The same halo on a button is the treatment the rule is for.
        let button = d.add(Some(body), "button");
        visible(&mut d, button);
        d.set_rect(button, 120.0, 80.0, 197.0, 40.0);
        d.set_style(button, "boxShadow", "rgba(0, 169, 255, 0.6) 0px 0px 24px 0px");
        d.set_style(button, "textShadow", "none");
        let hits = check_element_glow_dom(&d, button);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "Zero-offset box-shadow glow (#00a9ff)");
    }

    /// A heading whose own text paints over `rect`, so a glow behind it has
    /// something to be behind.
    fn heading_over(d: &mut FakeDom, parent: ElId, x: f64, y: f64) -> ElId {
        let h = d.add(Some(parent), "h2");
        d.add_text(h, "Headline over the glow");
        d.set_rect(h, x, y, 320.0, 48.0);
        d.el_mut(h).direct_text_rect = Some(Rect::from_xywh(x, y, 320.0, 48.0));
        h
    }

    #[test]
    fn radial_spotlight_and_oversized_h1() {
        let (mut d, body) = page();
        let sec = d.add(Some(body), "section");
        d.set_attr(sec, "class", "hero glow");
        d.set_style(
            sec,
            "backgroundImage",
            "radial-gradient(circle at 52% 38%, rgba(80, 111, 255, 0.26), transparent 44%)",
        );
        d.set_rect(sec, 0.0, 0.0, 800.0, 400.0);
        heading_over(&mut d, sec, 40.0, 120.0);
        let hits = check_element_radial_spotlight_dom(&d, sec);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "radial-spotlight-glow");
        assert!(hits[0].snippet.contains("\"hero\""), "{}", hits[0].snippet);
        assert!(hits[0].snippet.contains("800x400"), "{}", hits[0].snippet);

        let h1 = d.add(Some(body), "h1");
        d.add_text(h1, "A really long headline that dominates the whole viewport");
        d.set_style(h1, "fontSize", "96px");
        d.set_rect(h1, 0.0, 0.0, 1200.0, 300.0);
        let hits = check_element_oversized_h1_dom(&d, h1);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.starts_with("96px h1, 56 chars, 38vh"), "{}", hits[0].snippet);
    }

    /// A glow layer on a dark page, as the sites that carry them build it: an
    /// absolutely positioned div with one chromatic stop fading out.
    fn dark_page_with_glow(gradient: &str) -> (FakeDom, ElId, ElId) {
        let (mut d, body) = page();
        d.set_style(body, "backgroundColor", "rgb(4, 12, 19)");
        let section = d.add(Some(body), "section");
        d.set_style(section, "backgroundColor", "rgba(0, 0, 0, 0)");
        d.set_rect(section, 0.0, 0.0, 1280.0, 800.0);
        let glow = d.add(Some(section), "div");
        d.set_attr(glow, "class", "glow");
        d.set_styles(
            glow,
            &[
                ("position", "absolute"),
                ("opacity", "1"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("backgroundImage", gradient),
            ],
        );
        d.set_rect(glow, 0.0, 0.0, 803.0, 502.0);
        heading_over(&mut d, section, 60.0, 200.0);
        (d, section, glow)
    }

    #[test]
    fn radial_spotlight_needs_a_prominent_glow() {
        // Bright enough against the dark ground, and text sits on it.
        let (d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)",
        );
        let hits = check_element_radial_spotlight_dom(&d, glow);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].id, "radial-spotlight-glow");

        // The same hue at 0.10: a tonal shift in the ground, not a spotlight.
        let (d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(26, 189, 226, 0.10) 0%, transparent 70%)",
        );
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // Pastel mint on a white page: declared strong, barely visible.
        let (mut d, body) = page();
        let hero = d.add(Some(body), "section");
        d.set_styles(
            hero,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                (
                    "backgroundImage",
                    "radial-gradient(circle, rgba(63, 227, 223, 0.20) 0%, transparent 65%)",
                ),
                ("opacity", "1"),
            ],
        );
        d.set_rect(hero, 0.0, 0.0, 1280.0, 900.0);
        heading_over(&mut d, hero, 60.0, 200.0);
        assert!(check_element_radial_spotlight_dom(&d, hero).is_empty());

        // A bright stop the element's own opacity scales back to a wash.
        let (mut d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.38) 0%, transparent 70%)",
        );
        d.set_style(glow, "opacity", "0.35");
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // Nothing painted at all.
        let (mut d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)",
        );
        d.set_style(glow, "opacity", "0");
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // A bright glow with no copy over it is surface treatment.
        let (mut d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)",
        );
        d.set_rect(glow, 0.0, 2000.0, 803.0, 502.0);
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());
    }

    #[test]
    fn radial_spotlight_measures_a_hero_painted_with_a_gradient() {
        // The commonest way to build this pattern: a glow layer over a hero
        // whose own background is a gradient. The cascade cannot name one
        // color for it, so the mean of the gradient's stops is the surface.
        let bright = "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)";
        let (mut d, section, glow) = dark_page_with_glow(bright);
        d.set_style(
            section,
            "backgroundImage",
            "linear-gradient(180deg, rgb(11, 13, 19), rgb(20, 26, 43))",
        );
        assert_eq!(check_element_radial_spotlight_dom(&d, glow).len(), 1);

        // The same hero, the same glow at a wash's alpha: still silent.
        let (mut d, section, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(26, 189, 226, 0.10) 0%, transparent 70%)",
        );
        d.set_style(
            section,
            "backgroundImage",
            "linear-gradient(180deg, rgb(11, 13, 19), rgb(20, 26, 43))",
        );
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // A gradient hero pale enough to swallow the glow.
        let (mut d, section, glow) = dark_page_with_glow(bright);
        d.set_style(
            section,
            "backgroundImage",
            "linear-gradient(180deg, rgb(236, 244, 248), rgb(255, 255, 255))",
        );
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());
    }

    #[test]
    fn radial_spotlight_over_a_photograph_falls_back_to_alpha_and_copy() {
        // No cascade can say what a photo looks like under the glow, so the
        // contrast test drops out and the other two decide.
        let (mut d, section, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)",
        );
        d.set_style(section, "backgroundImage", "url(\"/hero.jpg\")");
        assert_eq!(check_element_radial_spotlight_dom(&d, glow).len(), 1);

        // A wash over the same photo is still a wash.
        let (mut d, section, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(26, 189, 226, 0.10) 0%, transparent 70%)",
        );
        d.set_style(section, "backgroundImage", "url(\"/hero.jpg\")");
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // And one with no copy over it is surface treatment, photo or not.
        let (mut d, section, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)",
        );
        d.set_style(section, "backgroundImage", "url(\"/hero.jpg\")");
        d.set_rect(glow, 0.0, 2000.0, 803.0, 502.0);
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());
    }

    #[test]
    fn radial_spotlight_does_not_depend_on_stop_order() {
        let bright_first = "radial-gradient(circle, rgba(120, 220, 255, 0.40) 0%, rgba(20, 20, 90, 0.30) 45%, transparent 75%)";
        let bright_second = "radial-gradient(circle, rgba(20, 20, 90, 0.30) 0%, rgba(120, 220, 255, 0.40) 45%, transparent 75%)";
        let (d, _, glow) = dark_page_with_glow(bright_first);
        let first = check_element_radial_spotlight_dom(&d, glow);
        let (d, _, glow) = dark_page_with_glow(bright_second);
        let second = check_element_radial_spotlight_dom(&d, glow);
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first[0].snippet, second[0].snippet);
        assert!(first[0].snippet.contains("#78dcff"), "{}", first[0].snippet);
    }

    #[test]
    fn radial_spotlight_measures_every_stop() {
        // A pale highlight core over a saturated ring: the ring is the glow,
        // and the finding names it.
        let (d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(255, 228, 186, 0.08) 0%, rgba(255, 90, 0, 0.40) 45%, transparent 75%)",
        );
        let hits = check_element_radial_spotlight_dom(&d, glow);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].snippet.contains("#ff5a00 a0.40"), "{}", hits[0].snippet);

        // The same hue at two alphas flags whichever stop is declared first,
        // and names the strong one both ways.
        for gradient in [
            "radial-gradient(circle, rgba(255, 90, 120, 0.28) 0%, rgba(255, 90, 120, 0.12) 45%, transparent 75%)",
            "radial-gradient(circle, rgba(255, 90, 120, 0.12) 0%, rgba(255, 90, 120, 0.28) 45%, transparent 75%)",
        ] {
            let (d, _, glow) = dark_page_with_glow(gradient);
            let hits = check_element_radial_spotlight_dom(&d, glow);
            assert_eq!(hits.len(), 1, "{gradient}");
            assert!(hits[0].snippet.contains("#ff5a78 a0.28"), "{}", hits[0].snippet);
        }
    }

    #[test]
    fn radial_spotlight_measures_through_a_translucent_layer_above_it() {
        // A white page whose hero carries a faint decorative fade: the fade is
        // composited over the page, not read as a wall that switches the
        // contrast test off, so a pastel glow in it stays silent.
        let (mut d, body) = page();
        let host = d.add(Some(body), "section");
        d.set_styles(
            host,
            &[
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                (
                    "backgroundImage",
                    "radial-gradient(circle at 50% 0%, rgba(0, 0, 0, 0.04), transparent 70%)",
                ),
                ("opacity", "1"),
            ],
        );
        d.set_rect(host, 0.0, 0.0, 900.0, 600.0);
        let glow = d.add(Some(host), "div");
        d.set_styles(
            glow,
            &[
                ("position", "absolute"),
                ("opacity", "1"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                (
                    "backgroundImage",
                    "radial-gradient(circle, rgba(63, 227, 223, 0.20) 0%, transparent 65%)",
                ),
            ],
        );
        d.set_rect(glow, 0.0, 0.0, 900.0, 600.0);
        heading_over(&mut d, host, 60.0, 200.0);
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // A pale translucent fade over a dark page lightens the surface enough
        // to swallow a glow that would flag on the bare dark ground.
        let bright = "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)";
        let (mut d, section, glow) = dark_page_with_glow(bright);
        d.set_style(
            section,
            "backgroundImage",
            "linear-gradient(180deg, rgba(255, 255, 255, 0.9), transparent)",
        );
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        // A dark fade leaves the ground dark, and the glow still flags.
        let (mut d, section, glow) = dark_page_with_glow(bright);
        d.set_style(
            section,
            "backgroundImage",
            "linear-gradient(180deg, rgba(20, 26, 43, 0.6), transparent)",
        );
        assert_eq!(check_element_radial_spotlight_dom(&d, glow).len(), 1);
    }

    #[test]
    fn radial_spotlight_reads_the_layers_beneath_it_in_its_own_value() {
        // The glow is the top layer of a panel painted with a pale gradient:
        // that gradient is the surface, not the dark page behind the panel.
        let (d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(63, 227, 223, 0.20) 0%, transparent 65%), linear-gradient(180deg, rgb(236, 244, 248), rgb(255, 255, 255))",
        );
        assert!(check_element_radial_spotlight_dom(&d, glow).is_empty());

        let (d, _, glow) = dark_page_with_glow(
            "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%), linear-gradient(180deg, rgb(11, 13, 19), rgb(20, 26, 43))",
        );
        assert_eq!(check_element_radial_spotlight_dom(&d, glow).len(), 1);
    }

    #[test]
    fn radial_spotlight_measures_the_page_text_once_per_scan() {
        let bright = "radial-gradient(circle, rgba(0, 209, 239, 0.16) 0%, transparent 70%)";
        let (mut d, section, glow) = dark_page_with_glow(bright);
        let wash = d.add(Some(section), "div");
        d.set_styles(
            wash,
            &[
                ("position", "absolute"),
                ("opacity", "1"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                (
                    "backgroundImage",
                    "radial-gradient(circle, rgba(26, 189, 226, 0.10) 0%, transparent 70%)",
                ),
            ],
        );
        d.set_rect(wash, 0.0, 0.0, 803.0, 502.0);
        let second = d.add(Some(section), "div");
        d.set_styles(
            second,
            &[
                ("position", "absolute"),
                ("opacity", "1"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("backgroundImage", bright),
            ],
        );
        d.set_rect(second, 0.0, 100.0, 803.0, 502.0);

        let text = GlowTextRects::default();
        // A wash never asks for the page's text.
        assert!(check_element_radial_spotlight_dom_with(&d, wash, &text).is_empty());
        assert!(text.0.get().is_none());
        // The first prominent glow measures it, the second reuses it.
        assert_eq!(check_element_radial_spotlight_dom_with(&d, glow, &text).len(), 1);
        let measured = text.0.get().expect("measured").as_ptr();
        assert_eq!(check_element_radial_spotlight_dom_with(&d, second, &text).len(), 1);
        assert_eq!(text.0.get().expect("kept").as_ptr(), measured);
    }

    #[test]
    fn clipped_overflow_and_text_overflow() {
        let (mut d, body) = page();
        let box_ = d.add(Some(body), "div");
        d.set_attr(box_, "class", "card");
        d.set_styles(box_, &[("overflow", "hidden"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(box_, 0.0, 0.0, 200.0, 100.0);
        let menu = d.add(Some(box_), "div");
        d.add_text(menu, "Menu item");
        d.set_style(menu, "position", "absolute");
        d.set_rect(menu, 0.0, 90.0, 200.0, 60.0);
        d.set_attr(menu, "class", "menu");
        let hits = check_element_clipped_overflow_dom(&d, box_);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "div.card clips positioned div.menu");
        d.set_attr(box_, "class", "carousel");
        assert!(check_element_clipped_overflow_dom(&d, box_).is_empty());

        let cell = d.add(Some(body), "div");
        visible(&mut d, cell);
        d.set_attr(cell, "class", "cell");
        d.add_text(cell, "averyveryverylongword");
        d.set_rect(cell, 0.0, 0.0, 100.0, 20.0);
        d.el_mut(cell).client_width = 100.0;
        d.el_mut(cell).client_height = 20.0;
        d.el_mut(cell).scroll_width = 140.0;
        d.set_styles(cell, &[("overflow", "visible"), ("overflowX", "visible"), ("overflowY", "visible"), ("position", "static"), ("fontSize", "16px"), ("width", "100px"), ("height", "20px")]);
        let hits = check_element_text_overflow_dom(&d, cell);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "div.cell overflows its box by 40px");
    }

    /// A clipping box with a real rect, the shape every case below shares.
    fn clipping_box(d: &mut FakeDom, parent: ElId, x: f64, y: f64, w: f64, h: f64) -> ElId {
        let el = d.add(Some(parent), "div");
        d.set_styles(
            el,
            &[
                ("overflow", "hidden"),
                ("overflowX", "hidden"),
                ("overflowY", "hidden"),
                ("display", "block"),
            ],
        );
        d.set_rect(el, x, y, w, h);
        el
    }

    fn positioned_child(d: &mut FakeDom, parent: ElId, x: f64, y: f64, w: f64, h: f64) -> ElId {
        let el = d.add(Some(parent), "div");
        d.set_styles(el, &[("position", "absolute"), ("opacity", "1")]);
        d.set_rect(el, x, y, w, h);
        el
    }

    #[test]
    fn clipped_overflow_exempts_masked_reveals_and_ornaments() {
        let (mut d, body) = page();
        // A same-size copy parked below the box by its own transform.
        let well = clipping_box(&mut d, body, 0.0, 0.0, 200.0, 100.0);
        let swap = positioned_child(&mut d, well, 0.0, 100.0, 200.0, 100.0);
        d.add_text(swap, "Saved");
        d.set_style(swap, "transform", "matrix(1, 0, 0, 1, 0, 100)");
        assert!(check_element_clipped_overflow_dom(&d, well).is_empty());
        // The same layer without the transform really is cut off.
        d.set_style(swap, "transform", "none");
        assert_eq!(check_element_clipped_overflow_dom(&d, well).len(), 1);
        // ... unless it is a menu, whatever parks it there.
        d.set_style(swap, "transform", "matrix(1, 0, 0, 1, 0, 100)");
        d.set_attr(swap, "role", "menu");
        d.add_selector(swap, "[role=\"menu\"]");
        assert_eq!(check_element_clipped_overflow_dom(&d, well).len(), 1);

        // Ornaments: no text, nothing to click, and no pointer target.
        let card = clipping_box(&mut d, body, 0.0, 200.0, 200.0, 100.0);
        let glow = positioned_child(&mut d, card, -20.0, 180.0, 240.0, 140.0);
        let glow_fill = d.add(Some(glow), "span");
        d.set_rect(glow_fill, -20.0, 180.0, 240.0, 140.0);
        d.set_style(glow, "pointerEvents", "none");
        assert!(check_element_clipped_overflow_dom(&d, card).is_empty());
        // ... or nothing visible at rest.
        d.set_style(glow, "pointerEvents", "auto");
        d.set_style(glow, "opacity", "0");
        assert!(check_element_clipped_overflow_dom(&d, card).is_empty());
        // ... or only an image inside a bled wrapper.
        d.set_style(glow, "opacity", "1");
        let photo = d.add(Some(glow), "img");
        d.set_rect(photo, -20.0, 180.0, 240.0, 140.0);
        assert!(check_element_clipped_overflow_dom(&d, card).is_empty());
        // Text in the same layer is a layer that needed to escape.
        d.add_text(glow, "Posted on the web");
        assert_eq!(check_element_clipped_overflow_dom(&d, card).len(), 1);
    }

    #[test]
    fn clipped_overflow_skips_boxless_page_and_nested_containers() {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 1280.0, 4000.0);
        for e in [html, body] {
            d.set_styles(e, &[("display", "block"), ("opacity", "1")]);
        }

        // `display: contents` generates no box, so it clips nothing.
        let shell = clipping_box(&mut d, body, 0.0, 0.0, 200.0, 100.0);
        d.set_style(shell, "display", "contents");
        let tip = positioned_child(&mut d, shell, 0.0, -40.0, 160.0, 30.0);
        d.add_text(tip, "Tooltip above the wrapper");
        assert!(check_element_clipped_overflow_dom(&d, shell).is_empty());
        d.set_style(shell, "display", "block");
        assert_eq!(check_element_clipped_overflow_dom(&d, shell).len(), 1);
        // Neither does a collapsed row.
        d.set_rect(shell, 0.0, 0.0, 200.0, 0.0);
        assert!(check_element_clipped_overflow_dom(&d, shell).is_empty());

        // The box the whole page sits in is layout containment.
        let page_shell = clipping_box(&mut d, body, 0.0, 0.0, 1280.0, 4000.0);
        let below = positioned_child(&mut d, page_shell, 0.0, 4200.0, 300.0, 40.0);
        d.add_text(below, "Content below the fold");
        assert!(check_element_clipped_overflow_dom(&d, page_shell).is_empty());

        // The exemption words count on the immediate scrolling child.
        let band = clipping_box(&mut d, body, 0.0, 0.0, 200.0, 40.0);
        let track = d.add(Some(band), "div");
        d.set_attr(track, "class", "marquee-track");
        d.set_rect(track, 0.0, 0.0, 800.0, 40.0);
        let item = positioned_child(&mut d, track, -200.0, 8.0, 200.0, 24.0);
        d.add_text(item, "Ticker copy");
        assert!(check_element_clipped_overflow_dom(&d, band).is_empty());
        d.set_attr(track, "class", "band-track");
        assert_eq!(check_element_clipped_overflow_dom(&d, band).len(), 1);

        // Nested clips repeat one decision: the clip nearest the layer owns
        // it, and the shell around it says nothing.
        let outer = clipping_box(&mut d, body, 0.0, 0.0, 200.0, 100.0);
        let inner = clipping_box(&mut d, outer, 0.0, 0.0, 180.0, 90.0);
        d.set_attr(outer, "class", "outer");
        d.set_attr(inner, "class", "inner");
        let menu = positioned_child(&mut d, inner, 0.0, -40.0, 160.0, 30.0);
        d.set_attr(menu, "class", "menu");
        d.add_text(menu, "Row actions");
        let hits = check_element_clipped_overflow_dom(&d, inner);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "div.inner clips positioned div.menu");
        assert!(check_element_clipped_overflow_dom(&d, outer).is_empty());

        // A second inner container is a second component with its own
        // finding, not one the shell absorbs.
        let inner_two = clipping_box(&mut d, outer, 0.0, 0.0, 180.0, 90.0);
        d.set_attr(inner_two, "class", "inner-two");
        let tip = positioned_child(&mut d, inner_two, 0.0, -50.0, 140.0, 26.0);
        d.set_attr(tip, "class", "tip");
        d.add_text(tip, "Delivered on Tuesday");
        let hits = check_element_clipped_overflow_dom(&d, inner_two);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "div.inner-two clips positioned div.tip");
        assert!(check_element_clipped_overflow_dom(&d, outer).is_empty());
    }

    #[test]
    fn clipped_overflow_keeps_findings_the_scan_never_visits_an_ancestor_for() {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 1280.0, 800.0);
        for e in [html, body] {
            d.set_styles(e, &[("display", "block"), ("opacity", "1")]);
        }
        // A centred column with `overflow: hidden` on `body`: a common guard
        // against sideways scrolling, and not page-shell shaped.
        d.set_rect(body, 240.0, 0.0, 800.0, 800.0);
        d.set_styles(
            body,
            &[("overflow", "hidden"), ("overflowX", "hidden"), ("overflowY", "hidden")],
        );

        let card = clipping_box(&mut d, body, 240.0, 0.0, 300.0, 120.0);
        d.set_attr(card, "class", "card");
        let tip = positioned_child(&mut d, card, 250.0, -30.0, 160.0, 30.0);
        d.set_attr(tip, "class", "tip");
        d.add_text(tip, "Free for the first month");

        // The tip escapes `body` as well, and `body` clips. But `body` is
        // never scanned, so it can never report this child: the card keeps
        // its own finding rather than handing it to nobody.
        assert!(!super::super::driver::element_is_scanned(&d, body));
        let hits = check_element_clipped_overflow_dom(&d, card);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "div.card clips positioned div.tip");
    }

    #[test]
    fn blinking_cursor_hero_promotion() {
        let (mut d, body) = page();
        let hero = d.add(Some(body), "header");
        let cur = d.add(Some(hero), "span");
        d.set_attr(cur, "class", "cursor");
        d.set_styles(
            cur,
            &[
                ("animationIterationCount", "infinite"),
                ("animationName", "blink"),
                ("backgroundColor", "rgb(0, 0, 0)"),
                ("borderRadius", "0px"),
                ("borderLeftWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
            ],
        );
        d.set_rect(cur, 100.0, 200.0, 2.0, 24.0);
        let hits = check_element_blinking_cursor_dom(&d, cur);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].severity.as_deref(), Some("warning"));
        assert_eq!(
            hits[0].detail,
            "span.cursor — 2x24px blinking cursor (animation \"blink\") in the first viewport"
        );
        // keyframes fallback: a fade name that toggles opacity
        d.set_style(cur, "animationName", "pulse-x");
        assert!(check_element_blinking_cursor_dom(&d, cur).is_empty());
        d.keyframes.insert(
            "pulse-x".into(),
            vec![crate::browser::dom::KeyframeFrame {
                decls: vec![("opacity".into(), "0".into())],
            }],
        );
        assert_eq!(check_element_blinking_cursor_dom(&d, cur).len(), 1);
    }

    /// One card carrying a hairline on every side plus `shadow`, sized `w`x`h`.
    fn hairline_card(d: &mut FakeDom, parent: ElId, w: f64, h: f64, shadow: &str) -> ElId {
        let card = d.add(Some(parent), "div");
        visible(d, card);
        d.set_rect(card, 0.0, 0.0, w, h);
        d.set_styles(
            card,
            &[
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgb(229, 231, 235)"),
                ("borderRightColor", "rgb(229, 231, 235)"),
                ("borderBottomColor", "rgb(229, 231, 235)"),
                ("borderLeftColor", "rgb(229, 231, 235)"),
                ("boxShadow", shadow),
            ],
        );
        card
    }

    /// A row of `n` identical cards under one parent; returns the first.
    fn hairline_row(d: &mut FakeDom, parent: ElId, n: usize, shadow: &str) -> ElId {
        let mut first = None;
        for _ in 0..n {
            let card = hairline_card(d, parent, 180.0, 140.0, shadow);
            first.get_or_insert(card);
        }
        first.expect("row")
    }

    #[test]
    fn gpt_border_shadow_needs_a_row_of_three() {
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let first = hairline_row(&mut d, row, 2, halo);
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());

        hairline_card(&mut d, row, 180.0, 140.0, halo);
        let hits = check_element_gpt_border_shadow_dom(&d, first);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "gpt-thin-border-wide-shadow");
        assert_eq!(
            hits[0].snippet,
            "1px border + 40px shadow blur, repeated across the row"
        );
    }

    #[test]
    fn gpt_border_shadow_ignores_tight_and_inset_shadows() {
        for shadow in [
            // a wide shadow, but a tight one
            "rgba(15, 23, 42, 0.18) 0px 0px 24px 0px",
            "rgba(15, 23, 42, 0.22) 0px 8px 24px 0px",
            // drawn inside the box
            "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px inset",
            "rgba(15, 23, 42, 0.18) 0px 8px 40px 0px inset",
        ] {
            let (mut d, body) = page();
            let row = d.add(Some(body), "div");
            let first = hairline_row(&mut d, row, 4, shadow);
            assert!(
                check_element_gpt_border_shadow_dom(&d, first).is_empty(),
                "{shadow} should not read as the repeated signature"
            );
        }
    }

    #[test]
    fn gpt_border_shadow_counts_a_row_lit_from_above() {
        // Every step of every mainstream elevation scale casts a y-offset, so
        // a repeated drop shadow is the population the rule is named for.
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let first = hairline_row(&mut d, row, 3, "rgba(15, 23, 42, 0.22) 0px 8px 40px 0px");
        let hits = check_element_gpt_border_shadow_dom(&d, first);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].snippet,
            "1px border + 40px shadow blur, repeated across the row"
        );
    }

    #[test]
    fn gpt_border_shadow_finds_row_mates_past_the_sibling_bound() {
        // A card sitting deep inside a long list still sees the boxes beside
        // it: the walk reads outward from the element, not the head of the
        // list.
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        for _ in 0..400 {
            let filler = d.add(Some(row), "div");
            visible(&mut d, filler);
            d.set_rect(filler, 0.0, 0.0, 180.0, 140.0);
        }
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let first = hairline_row(&mut d, row, 3, halo);
        assert_eq!(check_element_gpt_border_shadow_dom(&d, first).len(), 1);
    }

    #[test]
    fn gpt_border_shadow_row_needs_comparable_sizes() {
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let first = hairline_card(&mut d, row, 180.0, 140.0, halo);
        hairline_card(&mut d, row, 600.0, 90.0, halo);
        hairline_card(&mut d, row, 64.0, 400.0, halo);
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());

        // Within tolerance on both axes, the same three read as one row.
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let first = hairline_card(&mut d, row, 180.0, 140.0, halo);
        hairline_card(&mut d, row, 168.0, 132.0, halo);
        hairline_card(&mut d, row, 192.0, 148.0, halo);
        assert_eq!(check_element_gpt_border_shadow_dom(&d, first).len(), 1);
    }

    #[test]
    fn gpt_border_shadow_skips_a_row_that_paints_nothing() {
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let first = hairline_row(&mut d, row, 4, halo);
        for card in d.children(row) {
            d.set_rect(card, 0.0, 0.0, 0.0, 0.0);
        }
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());
    }

    /// Wraps a new hairline card in the tags of `wrappers`, outermost first,
    /// under `parent`; returns the card.
    fn wrapped_card(
        d: &mut FakeDom,
        parent: ElId,
        wrappers: &[&str],
        w: f64,
        h: f64,
        shadow: &str,
    ) -> ElId {
        let mut at = parent;
        for tag in wrappers {
            at = d.add(Some(at), tag);
            visible(d, at);
        }
        hairline_card(d, at, w, h, shadow)
    }

    #[test]
    fn gpt_border_shadow_counts_cards_wrapped_in_grid_items() {
        // The common generated grid wraps every card in its own link, so no
        // two cards are DOM siblings; the page still shows one row of three.
        let (mut d, body) = page();
        let grid = d.add(Some(body), "div");
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let cards: Vec<ElId> = (0..3)
            .map(|_| wrapped_card(&mut d, grid, &["a"], 180.0, 140.0, halo))
            .collect();
        for card in cards {
            assert_eq!(check_element_gpt_border_shadow_dom(&d, card).len(), 1);
        }

        // Two wrappers down, as in a list item holding a link.
        let (mut d, body) = page();
        let list = d.add(Some(body), "ul");
        let cards: Vec<ElId> = (0..3)
            .map(|_| wrapped_card(&mut d, list, &["li", "a"], 180.0, 140.0, halo))
            .collect();
        for card in cards {
            assert_eq!(check_element_gpt_border_shadow_dom(&d, card).len(), 1);
        }
    }

    #[test]
    fn gpt_border_shadow_counts_panels_repeated_one_per_article() {
        // Three articles down a page, each holding copy beside a panel, the
        // middle one flipped so the panel comes first: the panels sit at the
        // same depth but not at the same index, and they are one repetition.
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        let halo = "rgba(255, 255, 255, 0.04) 0px 1px 0px 0px inset, rgba(8, 33, 25, 0.6) 0px 30px 60px -40px";
        let mut panels = Vec::new();
        for flipped in [false, true, false] {
            let article = d.add(Some(stack), "article");
            visible(&mut d, article);
            if !flipped {
                let copy = d.add(Some(article), "p");
                visible(&mut d, copy);
            }
            let viz = d.add(Some(article), "div");
            visible(&mut d, viz);
            panels.push(hairline_card(&mut d, viz, 491.0, 265.0, halo));
            if flipped {
                let copy = d.add(Some(article), "p");
                visible(&mut d, copy);
            }
        }
        for panel in panels {
            let hits = check_element_gpt_border_shadow_dom(&d, panel);
            assert_eq!(hits.len(), 1);
            assert_eq!(
                hits[0].snippet,
                "1px border + 60px shadow blur, repeated across the row"
            );
        }
    }

    #[test]
    fn gpt_border_shadow_lone_panel_among_repeated_articles_stays_silent() {
        // A repeated layout is not enough: the other cells have to hold the
        // same card.
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let panel = wrapped_card(&mut d, stack, &["article", "div"], 491.0, 265.0, halo);
        for _ in 0..3 {
            let article = d.add(Some(stack), "article");
            visible(&mut d, article);
            let viz = d.add(Some(article), "div");
            visible(&mut d, viz);
            let plain = d.add(Some(viz), "div");
            visible(&mut d, plain);
            d.set_rect(plain, 0.0, 0.0, 491.0, 265.0);
        }
        assert!(check_element_gpt_border_shadow_dom(&d, panel).is_empty());
    }

    #[test]
    fn gpt_border_shadow_wrapped_row_needs_comparable_cards_and_cells() {
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";

        // Same wrappers, cards of very different sizes.
        let (mut d, body) = page();
        let grid = d.add(Some(body), "div");
        let first = wrapped_card(&mut d, grid, &["a"], 180.0, 140.0, halo);
        wrapped_card(&mut d, grid, &["a"], 600.0, 90.0, halo);
        wrapped_card(&mut d, grid, &["a"], 64.0, 400.0, halo);
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());

        // Comparable cards, but under wrappers of three different kinds.
        let (mut d, body) = page();
        let grid = d.add(Some(body), "div");
        let first = wrapped_card(&mut d, grid, &["a"], 180.0, 140.0, halo);
        wrapped_card(&mut d, grid, &["section"], 180.0, 140.0, halo);
        wrapped_card(&mut d, grid, &["aside"], 180.0, 140.0, halo);
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());
    }

    #[test]
    fn gpt_border_shadow_climbs_at_most_two_wrappers() {
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let (mut d, body) = page();
        let grid = d.add(Some(body), "div");
        let cards: Vec<ElId> = (0..3)
            .map(|_| wrapped_card(&mut d, grid, &["li", "a", "span"], 180.0, 140.0, halo))
            .collect();
        assert!(check_element_gpt_border_shadow_dom(&d, cards[0]).is_empty());
    }

    /// A nav bar of `n` items, each holding a trigger link beside a flyout
    /// sized like a card and wearing the pair; returns the flyouts.
    fn nav_with_flyouts(d: &mut FakeDom, body: ElId, n: usize) -> Vec<ElId> {
        let list = d.add(Some(body), "ul");
        (0..n)
            .map(|_| {
                let item = d.add(Some(list), "li");
                d.set_rect(item, 0.0, 0.0, 80.0, 20.0);
                let trigger = d.add(Some(item), "a");
                d.set_rect(trigger, 0.0, 0.0, 80.0, 20.0);
                let flyout =
                    hairline_card(d, item, 320.0, 200.0, "rgba(15, 23, 42, 0.18) 0px 20px 50px 0px");
                d.set_style(flyout, "position", "absolute");
                flyout
            })
            .collect()
    }

    #[test]
    fn gpt_border_shadow_hidden_flyouts_in_repeated_nav_items_stay_silent() {
        // Shown at rest, the three flyouts read as one row through their
        // wrappers, so the silence below comes from the visibility gate.
        let (mut d, body) = page();
        for flyout in nav_with_flyouts(&mut d, body, 3) {
            assert_eq!(check_element_gpt_border_shadow_dom(&d, flyout).len(), 1);
        }

        // Laid out ahead of their hover, they are popovers waiting for a
        // trigger, not a row of cards.
        type Hide = fn(&mut FakeDom, ElId);
        let hides: [(&str, Hide); 5] = [
            ("transparent", |d, f| {
                d.set_style(f, "opacity", "0");
            }),
            ("visibility hidden", |d, f| {
                d.set_style(f, "visibility", "hidden");
            }),
            ("inside a transparent nav item", |d, f| {
                let item = d.parent(f).expect("nav item");
                d.set_style(item, "opacity", "0");
            }),
            ("translated past the left edge", |d, f| {
                d.set_rect(f, -400.0, 40.0, 320.0, 200.0);
            }),
            ("parked past the viewport's right edge", |d, f| {
                d.set_rect(f, 1400.0, 40.0, 320.0, 200.0);
            }),
        ];
        for (name, hide) in hides {
            let (mut d, body) = page();
            let flyouts = nav_with_flyouts(&mut d, body, 3);
            for &flyout in &flyouts {
                hide(&mut d, flyout);
            }
            for &flyout in &flyouts {
                assert!(
                    check_element_gpt_border_shadow_dom(&d, flyout).is_empty(),
                    "flyouts {name} should stay silent"
                );
            }
        }
    }

    #[test]
    fn gpt_border_shadow_hidden_popover_is_not_a_row_mate() {
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let (mut d, body) = page();
        let row = d.add(Some(body), "div");
        let first = hairline_row(&mut d, row, 2, halo);
        let popover = hairline_card(&mut d, row, 180.0, 140.0, halo);
        d.set_style(popover, "position", "absolute");
        d.set_style(popover, "opacity", "0");
        assert!(check_element_gpt_border_shadow_dom(&d, first).is_empty());

        // A third card that shows makes the row, and the popover still
        // reports nothing.
        hairline_card(&mut d, row, 180.0, 140.0, halo);
        assert_eq!(check_element_gpt_border_shadow_dom(&d, first).len(), 1);
        assert!(check_element_gpt_border_shadow_dom(&d, popover).is_empty());
    }

    #[test]
    fn gpt_border_shadow_counts_a_row_staged_for_a_scroll_reveal() {
        // Before a scroll reveal runs, each article sits transparent and
        // offset in the flow. The panels inside are the row a visitor sees by
        // scrolling, and nothing about them waits for a trigger.
        let halo = "rgba(15, 23, 42, 0.18) 0px 0px 40px 0px";
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        let panels: Vec<ElId> = (0..3)
            .map(|i| {
                let panel = wrapped_card(&mut d, stack, &["article", "div"], 491.0, 265.0, halo);
                let article = d.parent(d.parent(panel).expect("viz")).expect("article");
                d.set_style(article, "opacity", "0");
                d.set_style(article, "transform", "matrix(1, 0, 0, 1, 0, 18)");
                d.set_rect(panel, 656.0, 1149.0 + 470.0 * i as f64, 491.0, 265.0);
                panel
            })
            .collect();
        for &panel in &panels {
            assert_eq!(check_element_gpt_border_shadow_dom(&d, panel).len(), 1);
        }

        // Cards staged one by one, each transparent or hidden in the flow.
        for (prop, value) in [("opacity", "0"), ("visibility", "hidden")] {
            let (mut d, body) = page();
            let row = d.add(Some(body), "div");
            let first = hairline_row(&mut d, row, 3, halo);
            for card in d.children(row) {
                d.set_style(card, prop, value);
            }
            assert_eq!(
                check_element_gpt_border_shadow_dom(&d, first).len(),
                1,
                "cards staged with {prop} {value}"
            );
        }
    }
}
