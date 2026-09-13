//! Section 5 per-element browser adapters (`checkElement*DOM`) from
//! `checks.mjs`, plus their DOM-facing helpers. See browser/mod.rs for the
//! full list this module owns.

#![allow(unused_imports)]

use super::background::{
    read_own_background_color, resolve_background_info, resolve_gradient_stops, BackgroundInfo,
};
use super::dom::{
    class_attr, class_attr_or_prop, closest_or_none, direct_text, has_direct_text_longer_than,
    matches_or_false, pf0, safe_id, style_px, tag_lower, Dom, ElId, ElStyle, Rect,
};
use super::BrowserFinding;
use crate::checks::measures::{
    self, border_colors_from_style, border_widths_from_style, check_gpt_thin_border_wide_shadow,
    check_oversized_h1, check_radial_spotlight, is_screen_reader_only_text_style,
    GptBorderShadowInput, OversizedH1Input, RadialSpotlightInput, SrOnlyMetrics,
};
use crate::checks::rules::{
    check_borders, check_colors, check_glow, check_hero_eyebrow, check_icon_tile,
    check_italic_serif, check_motion, check_placeholder_colors, is_emoji_only_text, BorderOpts,
    ColorOpts, GlowOpts, HeroEyebrowOpts, IconTileOpts, ItalicSerifOpts, MotionOpts, RuleHit,
    Sides, HEADING_TAGS,
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
        style_px(dom, el, "borderRadius"),
        &BorderOpts {
            badge_like,
            status_context: is_status_context_element(dom, el),
            tab_context: is_tab_context_element(dom, el),
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

/// JS: checks.mjs#checkElementColorsDOM(el)
pub fn check_element_colors_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
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
    let bg_info = resolve_background_info(dom, el);
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
        text_color: parse_rgb_or_any(&dom.style(el, "color")),
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
        bg_clip: Some(bg_clip),
        bg_image: Some(dom.style(el, "backgroundImage")),
        class_list: Some(class_attr(dom, el)),
        detector_is_browser: true,
    };
    let mut findings = check_colors(&color_opts);
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
    check_italic_serif(&ItalicSerifOpts {
        tag,
        font_style: Some(dom.style(el, "fontStyle")),
        font_family: Some(dom.style(el, "fontFamily")),
        font_size: style_px(dom, el, "fontSize"),
        heading_text: Some(dom.text_content(el)),
    })
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

/// The two hue bands the rule is about: the stock violet-to-cyan ramp.
const AI_PALETTE_VIOLET_BAND: (f64, f64) = (260.0, 310.0);
const AI_PALETTE_CYAN_BAND: (f64, f64) = (160.0, 200.0);
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

fn ai_palette_band(hue: f64) -> Option<&'static str> {
    if hue >= AI_PALETTE_VIOLET_BAND.0 && hue <= AI_PALETTE_VIOLET_BAND.1 {
        Some("Purple/violet")
    } else if hue >= AI_PALETTE_CYAN_BAND.0 && hue <= AI_PALETTE_CYAN_BAND.1 {
        Some("Cyan")
    } else {
        None
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
fn ai_palette_gradient_hit(dom: &dyn Dom, el: ElId) -> Option<RuleHit> {
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
    let mut label: Option<&'static str> = None;
    for c in &stops {
        if c.alpha_or_one() * opacity < AI_PALETTE_MIN_STOP_ALPHA {
            continue;
        }
        if !has_chroma(Some(c), Some(50.0)) {
            continue;
        }
        painted += 1;
        if let Some(band) = ai_palette_band(get_hue(Some(c))) {
            in_band += 1;
            if label.is_none() {
                label = Some(band);
            }
        }
    }
    let label = label?;
    // One stop grazing a band edge inside an otherwise warm or brand ramp is
    // that ramp's accident, not a violet-to-cyan palette.
    if in_band * 2 < painted {
        return None;
    }
    Some(RuleHit::new(
        "ai-color-palette",
        format!("{label} gradient background"),
    ))
}

/// The neon-text half: a run of glyphs the element paints itself, in band,
/// on a dark surface. SVG geometry, icon wrappers and spacer characters
/// inherit `color` without painting text, and one glyph is a texture.
fn ai_palette_text_hit(dom: &dyn Dom, el: ElId) -> Option<RuleHit> {
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
    let tc = parse_rgb_or_any(&dom.style(el, "color"))?;
    if tc.alpha_or_one() * own_opacity(dom, el) < AI_PALETTE_MIN_STOP_ALPHA {
        return None;
    }
    if !has_chroma(Some(&tc), Some(80.0)) {
        return None;
    }
    let label = ai_palette_band(get_hue(Some(&tc)))?;
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
    Some(RuleHit::new(
        "ai-color-palette",
        format!("{label} neon text on dark background"),
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

/// JS: checks.mjs#checkElementAIPaletteDOM(el)
pub fn check_element_ai_palette_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    // An element with no box paints nothing; see `ai_palette_is_visible` for
    // why the chain above it is read for the display switches only.
    if element_rect(dom, el).is_none() || !ai_palette_is_visible(dom, el) {
        return Vec::new();
    }
    let mut findings = Vec::new();
    findings.extend(ai_palette_gradient_hit(dom, el));
    findings.extend(ai_palette_text_hit(dom, el));
    findings
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

/// JS: checks.mjs#checkElementGptBorderShadowDOM(el)
pub fn check_element_gpt_border_shadow_dom(dom: &dyn Dom, el: ElId) -> Vec<RuleHit> {
    let style = ElStyle { dom, el };
    let widths = border_widths_from_style(&style);
    let colors: Vec<Option<String>> = border_colors_from_style(&style)
        .into_iter()
        .map(Some)
        .collect();
    let box_shadow = dom.style(el, "boxShadow");
    finding_hits(check_gpt_thin_border_wide_shadow(&GptBorderShadowInput {
        border_widths: &widths,
        border_colors: Some(&colors),
        box_shadow: Some(&box_shadow),
    }))
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
        let hits = check_element_colors_dom(&d, input);
        assert!(
            hits.iter().any(|h| {
                h.id == "low-contrast"
                    && h.snippet.contains("placeholder \"Pale Placeholder On White Field\"")
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
        let hits = check_element_colors_dom(&d, input);
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
        let hits = check_element_colors_dom(&d, p);
        assert!(hits.iter().any(|h| h.id == "low-contrast"), "{hits:?}");
        // hidden by opacity: nothing
        d.set_style(p, "opacity", "0");
        assert!(check_element_colors_dom(&d, p).is_empty());
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
        let hits = check_element_ai_palette_dom(&d, hero);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");
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
        let hits = check_element_ai_palette_dom(&d, cta);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Cyan gradient background");

        // Orange to violet to teal full-bleed hero wash.
        let hero = gradient_surface(
            &mut d,
            body,
            "linear-gradient(135deg, rgb(255, 87, 36) 0%, rgb(192, 88, 243) 50%, rgb(42, 157, 144) 100%)",
        );
        d.set_rect(hero, 0.0, 0.0, 1280.0, 800.0);
        let hits = check_element_ai_palette_dom(&d, hero);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");

        // A violet glow blob: one chromatic stop, the other fully transparent.
        let glow = gradient_surface(
            &mut d,
            body,
            "radial-gradient(50% 50%, rgba(133, 38, 254, 0.82) 0%, rgba(171, 171, 171, 0) 100%)",
        );
        d.set_rect(glow, 0.0, 0.0, 158.0, 158.0);
        let hits = check_element_ai_palette_dom(&d, glow);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Purple/violet gradient background");

        // A 20% violet overlay still paints a visible lavender corner.
        let overlay = gradient_surface(
            &mut d,
            body,
            "linear-gradient(to right top, rgba(192, 88, 243, 0.2), rgba(255, 255, 255, 0.6), rgba(255, 87, 36, 0.25))",
        );
        d.set_rect(overlay, 0.0, 0.0, 1280.0, 800.0);
        let hits = check_element_ai_palette_dom(&d, overlay);
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
        assert!(check_element_ai_palette_dom(&d, tint).is_empty());

        // A 120px blur is atmosphere, not a palette.
        let wash = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgb(130, 255, 247), rgb(255, 176, 5))",
        );
        d.set_style(wash, "filter", "blur(120px)");
        assert!(check_element_ai_palette_dom(&d, wash).is_empty());

        // A 1px timeline rail is not a surface.
        let rail = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgba(63, 227, 223, 0.35), rgba(96, 165, 250, 0.14))",
        );
        d.set_rect(rail, 0.0, 0.0, 237.0, 1.0);
        assert!(check_element_ai_palette_dom(&d, rail).is_empty());

        // One magenta stop grazing the band inside a warm story ring.
        let ring = gradient_surface(
            &mut d,
            body,
            "linear-gradient(rgb(213, 0, 194), rgb(255, 53, 60), rgb(255, 136, 0), rgb(255, 201, 0))",
        );
        d.set_rect(ring, 0.0, 0.0, 84.0, 84.0);
        assert!(check_element_ai_palette_dom(&d, ring).is_empty());

        // Two identical stops, alpha included, are a flat fill written as a
        // gradient.
        let flat = gradient_surface(
            &mut d,
            body,
            "linear-gradient(270deg, rgb(77, 20, 140) 0%, rgb(77, 20, 140) 100%)",
        );
        assert!(check_element_ai_palette_dom(&d, flat).is_empty());
        d.set_style(
            flat,
            "backgroundImage",
            "linear-gradient(270deg, rgba(77, 20, 140, 0.4) 0%, rgba(77, 20, 140, 0.4) 100%)",
        );
        assert!(check_element_ai_palette_dom(&d, flat).is_empty());

        // display:none nav chrome with a zero box.
        let hidden = gradient_surface(
            &mut d,
            body,
            "linear-gradient(90deg, rgb(0, 159, 219), rgb(130, 255, 247))",
        );
        d.set_style(hidden, "display", "none");
        assert!(check_element_ai_palette_dom(&d, hidden).is_empty());

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
        assert!(check_element_ai_palette_dom(&d, offscreen).is_empty());
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
        assert_eq!(check_element_ai_palette_dom(&d, blob).len(), 1);
        d.set_style(wrapper, "filter", "blur(120px)");
        assert!(check_element_ai_palette_dom(&d, blob).is_empty());

        // `backdrop-filter` blurs what is behind the element; the element's
        // own background is painted on top of it, sharp.
        d.set_style(wrapper, "filter", "none");
        d.set_style(blob, "backdropFilter", "blur(120px)");
        assert_eq!(check_element_ai_palette_dom(&d, blob).len(), 1);
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
        let hits = check_element_ai_palette_dom(&d, glow);
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
        assert_eq!(check_element_ai_palette_dom(&d, hero).len(), 1);
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
        assert_eq!(check_element_ai_palette_dom(&d, fill).len(), 1);

        let img = d.add(Some(fill), "img");
        visible(&mut d, img);
        d.set_rect(img, 0.0, 0.0, 120.0, 213.0);
        d.set_style(img, "objectFit", "cover");
        assert!(check_element_ai_palette_dom(&d, fill).is_empty());

        // A contained image letterboxes, so the gradient still shows.
        d.set_style(img, "objectFit", "contain");
        assert_eq!(check_element_ai_palette_dom(&d, fill).len(), 1);

        // A hidden image covers nothing.
        d.set_style(img, "objectFit", "cover");
        d.set_style(img, "visibility", "hidden");
        assert_eq!(check_element_ai_palette_dom(&d, fill).len(), 1);
        d.set_style(img, "visibility", "visible");
        assert!(check_element_ai_palette_dom(&d, fill).is_empty());
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
        assert_eq!(check_element_ai_palette_dom(&d, fill).len(), 1);

        d.set_style(inner, "objectFit", "cover");
        assert!(check_element_ai_palette_dom(&d, fill).is_empty());
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
        let hits = check_element_ai_palette_dom(&d, heading);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "Cyan neon text on dark background");

        // An icon wrapper carries the color but paints no text.
        let wrapper = neon_text_host(&mut d, body, "div", "");
        assert!(check_element_ai_palette_dom(&d, wrapper).is_empty());

        // SVG geometry inherits currentColor from the icon above it. Each
        // host is given real text so the namespace is the only thing that
        // can stop it: an <svg> reports once per shape otherwise.
        for tag in ["svg", "path", "circle", "line", "g"] {
            let shape = neon_text_host(&mut d, body, tag, "Download");
            assert!(
                check_element_ai_palette_dom(&d, shape).is_empty(),
                "<{tag}> reported"
            );
        }

        // A single glyph in a binary-rain texture is not neon text.
        let bit = neon_text_host(&mut d, body, "span", "1");
        d.set_rect(bit, 0.0, 0.0, 6.6, 11.0);
        assert!(check_element_ai_palette_dom(&d, bit).is_empty());

        // A whitespace-only span paints nothing either.
        let spacer = neon_text_host(&mut d, body, "span", " ");
        assert!(check_element_ai_palette_dom(&d, spacer).is_empty());

        // A typewriter hero splits the word into one text node per glyph and
        // paints every one of them.
        let typed = neon_text_host(&mut d, body, "span", "");
        for glyph in ["I", "m", "a", "g", "e"] {
            d.add_text(typed, glyph);
        }
        let hits = check_element_ai_palette_dom(&d, typed);
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
}
