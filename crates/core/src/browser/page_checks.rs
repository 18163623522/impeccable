//! Section 6 browser page-level checks from `checks.mjs`: `checkTypography`,
//! `isCardLikeDOM`, `checkLayout`, `checkHeadingRhythmDOM`,
//! `checkCreamPalette` (browser path), `measureHiddenTextDOM`,
//! `checkEdgeFlushCardsDOM`, `isLayeredElement`, `elementDirectText`,
//! `isPaintedForOcclusion`, `checkTextOcclusionDOM`,
//! `checkFirstViewportColumnOverflowDOM`.

#![allow(unused_imports)]
use super::dom::{
    ancestors_inclusive, class_attr, closest_or_none, direct_text, has_direct_text_longer_than, pf0,
    style_px, tag_lower, Dom, ElId, ElStyle, Rect,
};
use super::element_checks::{class_selector, effective_opacity_dom, is_rendered_for_browser_rule};
use super::{BrowserFinding, ElFinding};
use crate::checks::measures::{
    cream_from_class_list, is_cream_color, is_opaque_decorated_box,
    is_screen_reader_only_text_style, SrOnlyMetrics, StyleMap,
};
use crate::checks::rules::{
    check_flat_type_hierarchy_samples, is_card_like_from_props, type_hierarchy_role, RuleHit,
    TypeSample, TYPE_HIERARCHY_SELECTOR,
};
use crate::color::parse_any_color;
use crate::constants::{is_brand_font_on_own_domain, CSS_GENERIC_FONTS, OVERUSED_FONTS, SAFE_TAGS};
use crate::js::{self, math_max, math_min, math_round, number_to_string, parse_float, to_fixed};
use crate::js_ext_a::num_truthy;
use crate::js_ext_b::{slice_utf16_prefix, utf16_len};
use once_cell::sync::Lazy;
use regex::Regex;

/// The hidden-text measurement result type is shared.
pub use impeccable_foundation::browser::HiddenTextMeasure;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

/// JS `\b` (ASCII word boundary).
const B: &str = r"(?-u:\b)";
const D: &str = "[0-9]";

re!(WS_RE, format!("{}+", js::WS));
re!(QUOTE_EDGE_START, r#"^['"]"#);
re!(QUOTE_EDGE_END, r#"['"]$"#);
re!(SHADOW_CLASS_RE, format!(r"{B}shadow(?:-sm|-md|-lg|-xl|-2xl)?{B}"));
re!(BORDER_CLASS_RE, format!(r"{B}border{B}"));
re!(ROUNDED_CLASS_RE, format!(r"{B}rounded(?:-sm|-md|-lg|-xl|-2xl|-full)?{B}"));
re!(BG_CLASS_RE, format!(r"{B}bg-(?:white|gray-{D}+|slate-{D}+){B}"));
re!(
    POPOVER_CLASS_RE,
    format!(
        "{B}(?:{}){B}",
        ["dropdown", "popover", "tooltip", "menu", "modal", "dialog"]
            .iter()
            .map(|w| js::ci(w))
            .collect::<Vec<_>>()
            .join("|")
    )
);
re!(SCROLL_RE, r"(auto|scroll)");
re!(HIDDEN_VIS_RE, r"^(hidden|collapse)$");
re!(
    MARQUEE_IDENT_RE,
    format!(
        "{B}({}){B}",
        ["marquee", "ticker", "scroller", "carousel", "conveyor"]
            .iter()
            .map(|w| js::ci(w))
            .collect::<Vec<_>>()
            .join("|")
    )
);
re!(MARQUEE_ANIM_RE, r"marquee|ticker|scroll");
re!(GRADIENT_URL_RE, format!("({}|{})\\(", js::ci("gradient"), js::ci("url")));
re!(MULTI_COL_RE, r"(^|inline-)(grid|flex)$");

/// JS `s.replace(/\s+/g, ' ')`.
fn collapse_ws(s: &str) -> String {
    WS_RE.replace_all(s, " ").into_owned()
}

/// JS `f.trim().replace(/^['"]|['"]$/g, '')`: one leading and one trailing
/// quote removed (the `g` flag on an anchored alternation).
fn strip_edge_quotes(s: &str) -> String {
    let t = QUOTE_EDGE_START.replace(s, "");
    QUOTE_EDGE_END.replace(&t, "").into_owned()
}

/// JS `[...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim().length > 0)`.
fn has_visible_direct_text(dom: &dyn Dom, el: ElId) -> bool {
    has_direct_text_longer_than(dom, el, 0)
}

const IMPECCABLE_OWN: &str =
    ".impeccable-overlay, .impeccable-label, .impeccable-banner, .impeccable-tooltip";

/// JS: checks.mjs#checkTypography()
pub fn check_typography(dom: &dyn Dom) -> Vec<BrowserFinding> {
    let mut findings = Vec::new();

    let mut font_usage: Vec<(String, f64)> = Vec::new();
    let mut total_text_elements = 0.0f64;
    for el in dom
        .query_all(
            None,
            "p, h1, h2, h3, h4, h5, h6, li, td, th, dd, blockquote, figcaption, a, button, label, span",
        )
        .unwrap_or_default()
    {
        if closest_or_none(dom, el, IMPECCABLE_OWN).is_some() {
            continue;
        }
        if !has_visible_direct_text(dom, el) {
            continue;
        }
        let ff = dom.style(el, "fontFamily");
        if ff.is_empty() {
            continue;
        }
        let stack: Vec<String> = ff
            .split(',')
            .map(|f| js::to_lower_case(&strip_edge_quotes(js::trim(f))))
            .collect();
        // JS-PARITY: checks.mjs#checkTypography uses primaryFontFace(ff) whose
        // default skip is CSS_GENERIC_FONTS, so a system stack keeps its system
        // face as primary (fix #678).
        let Some(primary) = stack
            .iter()
            .find(|f| !f.is_empty() && !CSS_GENERIC_FONTS.contains(&f.as_str()))
        else {
            continue;
        };
        if let Some(slot) = font_usage.iter_mut().find(|(k, _)| k == primary) {
            slot.1 += 1.0;
        } else {
            font_usage.push((primary.clone(), 1.0));
        }
        total_text_elements += 1.0;
    }

    if total_text_elements >= 20.0 {
        // Report the actual primary face: the uniquely most-used family. The
        // old 15% threshold labeled secondary faces as primary, e.g. an 82/18
        // split (#709). `Array.prototype.sort` is stable, so ties keep
        // first-seen order and the tie test compares the top two counts.
        let hostname = dom.hostname();
        let mut ranked: Vec<&(String, f64)> = font_usage.iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((font, count)) = ranked.first().map(|(f, c)| (f, *c)) {
            let tied = ranked.get(1).map(|r| r.1) == Some(count);
            if !tied {
                let share = count / total_text_elements;
                if OVERUSED_FONTS.contains(&font.as_str())
                    && !is_brand_font_on_own_domain(font, Some(&hostname))
                {
                    findings.push(BrowserFinding::new(
                        "overused-font",
                        format!(
                            "Primary font: {} ({}% of text)",
                            font,
                            number_to_string(math_round(share * 100.0))
                        ),
                    ));
                }
            }
        }
    }

    for hit in check_flat_type_hierarchy_from_dom(dom, Some(TYPE_HIERARCHY_SKIP_SELECTOR)) {
        findings.push(BrowserFinding::new(&hit.id, hit.snippet));
    }

    findings
}

/// The overlay chrome `checkTypography` hands `checkFlatTypeHierarchyFromDoc`
/// as its `skipElement` selector.
pub const TYPE_HIERARCHY_SKIP_SELECTOR: &str =
    ".impeccable-overlay, .impeccable-label, .impeccable-banner, .impeccable-tooltip, [id^=\"impeccable-live-\"]";

/// JS: checks.mjs#isRenderedTypeElement over a live DOM.
fn is_rendered_type_element(dom: &dyn Dom, el: ElId) -> bool {
    for current in ancestors_inclusive(dom, el) {
        if dom.hidden_prop(current) || dom.attr(current, "hidden").is_some() {
            return false;
        }
        let display = js::to_lower_case(&dom.style(current, "display"));
        let visibility = js::to_lower_case(&dom.style(current, "visibility"));
        let content_visibility = js::to_lower_case(&dom.style(current, "contentVisibility"));
        if display == "none"
            || visibility == "hidden"
            || visibility == "collapse"
            || content_visibility == "hidden"
        {
            return false;
        }
        let opacity = parse_float(&dom.style(current, "opacity"));
        if opacity.is_finite() && opacity <= 0.01 {
            return false;
        }
    }
    true
}

/// JS: checks.mjs#checkFlatTypeHierarchyFromDoc over a live DOM.
pub fn check_flat_type_hierarchy_from_dom(
    dom: &dyn Dom,
    skip_selector: Option<&str>,
) -> Vec<RuleHit> {
    let mut samples: Vec<TypeSample> = Vec::new();
    for el in dom
        .query_all(None, TYPE_HIERARCHY_SELECTOR)
        .unwrap_or_default()
    {
        if let Some(sel) = skip_selector {
            if closest_or_none(dom, el, sel).is_some() {
                continue;
            }
        }
        if js::trim(&dom.text_content(el)).is_empty() || !is_rendered_type_element(dom, el) {
            continue;
        }
        let font_size = parse_float(&dom.style(el, "fontSize"));
        if !font_size.is_finite() || font_size < 8.0 || font_size >= 200.0 {
            continue;
        }
        samples.push(TypeSample {
            role: type_hierarchy_role(&tag_lower(dom, el)),
            size: font_size,
        });
    }
    check_flat_type_hierarchy_samples(&samples)
}

/// JS: checks.mjs#isCardLikeDOM(el)
pub fn is_card_like_dom(dom: &dyn Dom, el: ElId) -> bool {
    let tag = tag_lower(dom, el);
    if SAFE_TAGS.contains(&tag.as_str())
        || matches!(
            tag.as_str(),
            "input" | "select" | "textarea" | "img" | "video" | "canvas" | "picture"
        )
    {
        return false;
    }
    let cls = class_attr(dom, el);
    let box_shadow = dom.style(el, "boxShadow");
    let has_shadow = (!box_shadow.is_empty() && box_shadow != "none") || SHADOW_CLASS_RE.is_match(&cls);
    let has_border = BORDER_CLASS_RE.is_match(&cls);
    let has_radius = parse_float(&dom.style(el, "borderRadius")) > 0.0 || ROUNDED_CLASS_RE.is_match(&cls);
    let bg = dom.style(el, "backgroundColor");
    let has_bg = (!bg.is_empty() && bg != "rgba(0, 0, 0, 0)") || BG_CLASS_RE.is_match(&cls);
    is_card_like_from_props(has_shadow, has_border, has_radius, has_bg)
}

/// JS: checks.mjs#checkLayout() — `{ type, detail, el }`.
pub fn check_layout(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut findings = Vec::new();
    let mut flagged: Vec<ElId> = Vec::new();

    for el in dom.query_all(None, "*").unwrap_or_default() {
        if !is_card_like_dom(dom, el) || flagged.contains(&el) {
            continue;
        }
        let cls = class_attr(dom, el);
        let pos = dom.style(el, "position");
        if pos == "absolute" || pos == "fixed" {
            continue;
        }
        if POPOVER_CLASS_RE.is_match(&cls) {
            continue;
        }
        if utf16_len(js::trim(&dom.text_content(el))) < 10 {
            continue;
        }
        let rect = dom.rect(el);
        if rect.width < 50.0 || rect.height < 30.0 {
            continue;
        }
        let mut parent = dom.parent(el);
        while let Some(p) = parent {
            if is_card_like_dom(dom, p) {
                flagged.push(el);
                break;
            }
            parent = dom.parent(p);
        }
    }

    for &el in &flagged {
        let is_ancestor = flagged
            .iter()
            .any(|&other| other != el && dom.contains(el, other));
        if !is_ancestor {
            findings.push(ElFinding {
                el: Some(el),
                finding: BrowserFinding::new("nested-cards", "Card inside card"),
            });
        }
    }
    findings
}

/// `heading-rhythm`: an element that takes part in normal flow and paints a
/// box of its own.
fn rhythm_visible_flow(dom: &dyn Dom, el: ElId) -> bool {
    let display = dom.style(el, "display");
    let visibility = dom.style(el, "visibility");
    if display == "none" || visibility == "hidden" {
        return false;
    }
    let op = dom.style(el, "opacity");
    let op = if op.is_empty() { "1".to_string() } else { op };
    if parse_float(&op) <= 0.05 {
        return false;
    }
    let pos = dom.style(el, "position");
    if pos == "absolute" || pos == "fixed" || pos == "sticky" {
        return false;
    }
    let r = dom.rect(el);
    r.width >= 1.0 && r.height >= 1.0
}

/// `display: contents` generates no box: its children lay out as children
/// of its parent, so the walks look through it.
fn rhythm_is_contents(dom: &dyn Dom, el: ElId) -> bool {
    dom.style(el, "display") == "contents"
}

fn rhythm_overlaps_x(sr: &Rect, rect: &Rect) -> bool {
    math_min(sr.right, rect.right) - math_max(sr.left, rect.left) >= 8.0
}

/// A box that paints an edge on `side` ("Top" or "Bottom"): a background,
/// a border on that side, or a shadow.
fn rhythm_paints_edge(dom: &dyn Dom, el: ElId, side: &str) -> bool {
    if rhythm_is_contents(dom, el) {
        return false;
    }
    if let Some(bg) = parse_any_color(Some(&dom.style(el, "backgroundColor"))) {
        if bg.alpha_or_one() > 0.05 {
            return true;
        }
    }
    if style_px(dom, el, &format!("border{side}Width")) > 0.0 {
        return true;
    }
    crate::checks::measures::box_shadow_paints(&dom.style(el, "boxShadow"))
}

/// The flow box `s` presents to a walk: `s` itself, or for a
/// `display: contents` element the nearest of its children. `pick` tests a
/// candidate's rect; `from_end` walks the children last-first.
fn rhythm_flow_box(
    dom: &dyn Dom,
    s: ElId,
    rect: &Rect,
    from_end: bool,
    pick: &dyn Fn(&Rect) -> bool,
) -> Option<ElId> {
    if rhythm_is_contents(dom, s) {
        let mut kids = dom.children(s);
        if from_end {
            kids.reverse();
        }
        // A spacer child is space, not the wrapper's block: the walk goes on
        // to the content beside it.
        return kids.into_iter().find_map(|k| {
            rhythm_flow_box(dom, k, rect, from_end, pick).filter(|&b| !rhythm_is_spacer(dom, b))
        });
    }
    if !rhythm_visible_flow(dom, s) {
        return None;
    }
    let sr = dom.rect(s);
    (pick(&sr) && rhythm_overlaps_x(&sr, rect)).then_some(s)
}

const RHYTHM_MEDIA_TAGS: &[&str] = &["img", "picture", "video", "canvas", "svg", "iframe"];

/// A box that draws a border on a side other than its bottom: a frame, not a
/// rule.
fn rhythm_frames(dom: &dyn Dom, el: ElId) -> bool {
    ["borderTopWidth", "borderLeftWidth", "borderRightWidth"]
        .iter()
        .any(|side| style_px(dom, el, side) > 0.0)
}

/// The block measured above a heading already separates it from the heading:
/// a rule (an `hr` or a line a few pixels tall), a bottom border drawn alone
/// on the block or on the descendants that form its bottom edge, or a picture
/// that forms that edge. A heading tight under a photo is that photo's
/// caption, and a heading tight under a rule starts the section the rule
/// opens; neither reads as a caption for the content above. A box bordered on
/// its other sides too (a code block, a panel, a table cell) is a framed block
/// of content, and the heading tight under it still reads as its caption.
fn rhythm_block_separates(dom: &dyn Dom, el: ElId) -> bool {
    let er = dom.rect(el);
    if tag_lower(dom, el) == "hr" || er.height <= 4.0 {
        return true;
    }
    // A block with no words that holds a picture is a picture: a photo frame,
    // an icon badge.
    if js::trim(&dom.text_content(el)).is_empty()
        && !dom
            .query_all(Some(el), "img, picture, video, canvas, svg, iframe")
            .unwrap_or_default()
            .is_empty()
    {
        return true;
    }
    let mut cur = Some(el);
    let mut framed = false;
    for _ in 0..8 {
        let Some(c) = cur else { break };
        let cr = dom.rect(c);
        // Inside a frame, the frame is the edge a reader sees; a line drawn
        // within it is part of the framed content.
        framed = framed || rhythm_frames(dom, c);
        // A rule runs across the block; a bordered button or chip inside it
        // does not.
        if !framed && style_px(dom, c, "borderBottomWidth") > 0.0 && cr.width >= er.width * 0.9 {
            return true;
        }
        let tag = tag_lower(dom, c);
        if RHYTHM_MEDIA_TAGS.contains(&tag.as_str()) && cr.width >= er.width * 0.5 {
            return true;
        }
        // A heading stacked under another heading (a name over a title, a
        // title over a subtitle) is one titling group, not a caption for
        // content above.
        if matches!(tag.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
            return true;
        }
        cur = dom.children(c).into_iter().rev().find(|&k| {
            if dom.style(k, "display") == "none" {
                return false;
            }
            let kr = dom.rect(k);
            kr.width >= 1.0 && kr.height >= 1.0 && kr.bottom >= er.bottom - 2.0
        });
    }
    false
}

fn rhythm_rendered_children(dom: &dyn Dom, el: ElId) -> Vec<ElId> {
    dom.children(el)
        .into_iter()
        .filter(|&k| {
            if dom.style(k, "display") == "none" {
                return false;
            }
            let pos = dom.style(k, "position");
            if pos == "absolute" || pos == "fixed" {
                return false;
            }
            let r = dom.rect(k);
            r.width >= 1.0 && r.height >= 1.0
        })
        .collect()
}

/// An empty box that only holds space open: no text, no picture, nothing
/// laid out inside it, nothing painted. It is part of the gap, not a block.
/// Form controls draw what they hold (a value, a placeholder) without DOM
/// text, so an empty one is content, not space.
const RHYTHM_CONTROL_TAGS: &[&str] = &["input", "textarea", "select", "button", "meter", "progress"];

fn rhythm_is_spacer(dom: &dyn Dom, el: ElId) -> bool {
    let tag = tag_lower(dom, el);
    !rhythm_paints_edge(dom, el, "Bottom")
        && !RHYTHM_MEDIA_TAGS.contains(&tag.as_str())
        && !RHYTHM_CONTROL_TAGS.contains(&tag.as_str())
        && rhythm_rendered_children(dom, el).is_empty()
        && js::trim(&dom.text_content(el)).is_empty()
}

/// Where a block's content ends, as a reader sees it: a box that paints its
/// bottom edge ends at that edge; otherwise its bottom padding is space, and
/// so is whatever runs past the lowest child it lays out.
fn rhythm_content_bottom(dom: &dyn Dom, el: ElId) -> f64 {
    // An inline run can draw its line box past the block that holds it; the
    // block's own bottom is as far as its content reaches.
    math_min(rhythm_lowest_content(dom, el), dom.rect(el).bottom)
}

fn rhythm_lowest_content(dom: &dyn Dom, el: ElId) -> f64 {
    let mut cur = el;
    for _ in 0..8 {
        let r = dom.rect(cur);
        if rhythm_paints_edge(dom, cur, "Bottom")
            || RHYTHM_MEDIA_TAGS.contains(&tag_lower(dom, cur).as_str())
        {
            return r.bottom;
        }
        let lowest = rhythm_rendered_children(dom, cur)
            .into_iter()
            .filter(|&k| !rhythm_is_spacer(dom, k))
            .max_by(|&a, &b| {
                dom.rect(a)
                    .bottom
                    .partial_cmp(&dom.rect(b).bottom)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        match lowest {
            Some(k) => cur = k,
            None => return r.bottom - math_max(0.0, style_px(dom, cur, "paddingBottom")),
        }
    }
    dom.rect(cur).bottom
}

fn rhythm_font_size(dom: &dyn Dom, el: ElId) -> f64 {
    let n = parse_float(&dom.style(el, "fontSize"));
    if num_truthy(n) {
        n
    } else {
        16.0
    }
}

/// `el` and its descendants in document order, at most `limit` of them.
fn rhythm_subtree(dom: &dyn Dom, el: ElId, limit: usize) -> Vec<ElId> {
    let mut out = Vec::new();
    let mut stack = vec![el];
    while let Some(c) = stack.pop() {
        if out.len() >= limit {
            break;
        }
        out.push(c);
        let mut kids = dom.children(c);
        kids.reverse();
        stack.extend(kids);
    }
    out
}

fn rhythm_has_words(dom: &dyn Dom, el: ElId) -> bool {
    dom.direct_text_nodes(el).iter().any(|t| !js::trim(t).is_empty())
}

/// The size of the text a block sets: that of the first element in it that
/// holds words, or the block's own size when none does.
fn rhythm_text_size(dom: &dyn Dom, el: ElId) -> f64 {
    let first = rhythm_subtree(dom, el, 60)
        .into_iter()
        .find(|&e| rhythm_has_words(dom, e))
        .unwrap_or(el);
    rhythm_font_size(dom, first)
}

/// A short line above a heading that reads as the heading's label: set
/// smaller than the body text (`text_size`), in capitals, tracked out, or
/// as a chip that paints its own small box. A line set like the body copy is
/// content of its own (a date, a byline, a closing sentence), not a label.
fn rhythm_reads_as_eyebrow(dom: &dyn Dom, line: ElId, heading: ElId, text_size: f64) -> bool {
    let heading_size = rhythm_font_size(dom, heading);
    let span = math_max(dom.rect(heading).width, dom.rect(line).width);
    for e in rhythm_subtree(dom, line, 40) {
        let er = dom.rect(e);
        if rhythm_paints_edge(dom, e, "Bottom")
            && er.width >= 1.0
            && er.width < span * 0.6
            && !js::trim(&dom.text_content(e)).is_empty()
        {
            return true;
        }
        if !rhythm_has_words(dom, e) {
            continue;
        }
        let size = rhythm_font_size(dom, e);
        if size > heading_size {
            continue;
        }
        if size < text_size * 0.92 || dom.style(e, "textTransform") == "uppercase" {
            return true;
        }
        let tracking = parse_float(&dom.style(e, "letterSpacing"));
        if tracking.is_finite() && tracking >= 0.5 {
            return true;
        }
        let text = dom.direct_text_nodes(e).concat();
        let cased: Vec<char> = text
            .chars()
            .filter(|c| c.is_uppercase() || c.is_lowercase())
            .collect();
        if cased.len() >= 3 && cased.iter().all(|c| c.is_uppercase()) {
            return true;
        }
    }
    false
}

fn rhythm_painted_background(dom: &dyn Dom, el: ElId) -> Option<crate::color::Rgba> {
    parse_any_color(Some(&dom.style(el, "backgroundColor"))).filter(|c| c.alpha_or_one() > 0.05)
}

/// A box that shows where it ends: a bottom border, a shadow, or a background
/// band that differs from the backdrop behind what follows it.
fn rhythm_draws_bottom_edge(dom: &dyn Dom, el: ElId) -> bool {
    if rhythm_is_contents(dom, el) {
        return false;
    }
    if style_px(dom, el, "borderBottomWidth") > 0.0 {
        return true;
    }
    if crate::checks::measures::box_shadow_paints(&dom.style(el, "boxShadow")) {
        return true;
    }
    let Some(band) = rhythm_painted_background(dom, el) else { return false };
    let mut backdrop = crate::color::Rgba::new(255.0, 255.0, 255.0, 1.0);
    let mut cur = dom.parent(el);
    while let Some(c) = cur {
        if let Some(bg) = rhythm_painted_background(dom, c) {
            backdrop = bg;
            break;
        }
        cur = dom.parent(c);
    }
    (band.r - backdrop.r).abs() > 2.0
        || (band.g - backdrop.g).abs() > 2.0
        || (band.b - backdrop.b).abs() > 2.0
        || (band.alpha_or_one() - backdrop.alpha_or_one()).abs() > 0.02
}

/// The outline of a box's rendered structure: tags only, a few levels deep.
fn rhythm_shape(dom: &dyn Dom, el: ElId, depth: usize, budget: &mut usize, out: &mut String) {
    out.push_str(&tag_lower(dom, el));
    if depth == 0 {
        return;
    }
    let kids = rhythm_rendered_children(dom, el);
    if kids.is_empty() {
        return;
    }
    out.push('(');
    for k in kids {
        if *budget == 0 {
            out.push('+');
            break;
        }
        *budget -= 1;
        rhythm_shape(dom, k, depth - 1, budget, out);
        out.push(' ');
    }
    out.push(')');
}

fn rhythm_shape_of(dom: &dyn Dom, el: ElId) -> String {
    let mut out = String::new();
    let mut budget = 24;
    rhythm_shape(dom, el, 3, &mut budget, &mut out);
    out
}

/// How much two outlines share: the Dice coefficient of their tag multisets.
fn rhythm_shape_similarity(a: &str, b: &str) -> f64 {
    let tokens = |s: &str| -> Vec<String> {
        s.split(|c: char| c == '(' || c == ')' || c == ' ')
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect()
    };
    let ta = tokens(a);
    let mut tb = tokens(b);
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    let total = ta.len() + tb.len();
    let mut shared = 0usize;
    for t in &ta {
        if let Some(i) = tb.iter().position(|u| u == t) {
            tb.swap_remove(i);
            shared += 1;
        }
    }
    2.0 * shared as f64 / total as f64
}

/// The way down from `row` to `h`, heading first: each step's tag, class, and
/// index among its parent's children.
fn rhythm_heading_path(dom: &dyn Dom, row: ElId, h: ElId) -> Vec<(String, String, usize)> {
    let mut path = Vec::new();
    let mut cur = h;
    while cur != row {
        let Some(p) = dom.parent(cur) else { break };
        let idx = dom.children(p).iter().position(|&k| k == cur).unwrap_or(0);
        path.push((tag_lower(dom, cur), class_attr(dom, cur), idx));
        cur = p;
    }
    path
}

/// `s` holds a heading of `h`'s level where `row` holds `h`: at the same child
/// path (the same tags at the same indices), or under the same chain of tags
/// and classes.
fn rhythm_holds_heading_alike(dom: &dyn Dom, s: ElId, path: &[(String, String, usize)]) -> bool {
    let Some((heading_tag, _, _)) = path.first() else { return false };
    let mut cur = Some(s);
    for (tag, _, idx) in path.iter().rev() {
        cur = cur
            .and_then(|c| dom.children(c).get(*idx).copied())
            .filter(|&k| &tag_lower(dom, k) == tag);
    }
    if cur.is_some() {
        return true;
    }
    rhythm_subtree(dom, s, 400).into_iter().skip(1).any(|e| {
        &tag_lower(dom, e) == heading_tag
            && rhythm_heading_path(dom, s, e)
                .iter()
                .map(|(t, c, _)| (t, c))
                .eq(path.iter().map(|(t, c, _)| (t, c)))
    })
}

/// Outlines at least this alike are one repeated component: an accordion row
/// next to the open row, a card without the label its neighbour carries.
const RHYTHM_REPEAT_SIMILARITY: f64 = 0.7;

/// A box that is one of a run of like boxes: an accordion row, a list item, a
/// card in a grid. The box laid out next to it on either side has the same
/// tag, holds a heading of the same level in the same place (the same child
/// path, or the same chain of classes down to it), and has a similar outline.
/// A layout wrapper that shares a generic class with its neighbour but holds
/// other content (a heading alone in a `.row`, then a `.row` of feature
/// columns) repeats nothing, and the heading is measured past it.
fn rhythm_repeats(dom: &dyn Dom, el: ElId, h: ElId) -> bool {
    let tag = tag_lower(dom, el);
    let shape = rhythm_shape_of(dom, el);
    let path = rhythm_heading_path(dom, el, h);
    let lays_out = |s: ElId| -> bool {
        let pos = dom.style(s, "position");
        let r = dom.rect(s);
        dom.style(s, "display") != "none"
            && pos != "absolute"
            && pos != "fixed"
            && r.width >= 1.0
            && r.height >= 1.0
            && !rhythm_is_spacer(dom, s)
    };
    let alike = |s: ElId| -> bool {
        tag_lower(dom, s) == tag
            && rhythm_holds_heading_alike(dom, s, &path)
            && rhythm_shape_similarity(&shape, &rhythm_shape_of(dom, s)) >= RHYTHM_REPEAT_SIMILARITY
    };
    let mut prev = dom.previous_element_sibling(el);
    while let Some(s) = prev {
        if lays_out(s) {
            if alike(s) {
                return true;
            }
            break;
        }
        prev = dom.previous_element_sibling(s);
    }
    let mut next = dom.next_element_sibling(el);
    while let Some(s) = next {
        if lays_out(s) {
            return alike(s);
        }
        next = dom.next_element_sibling(s);
    }
    false
}

/// JS: checks.mjs#checkHeadingRhythmDOM()
pub fn check_heading_rhythm_dom(dom: &dyn Dom) -> Vec<ElFinding> {
    const MIN_VIOLATIONS: usize = 2;
    const CARD_EXEMPT_HEIGHT: f64 = 200.0;
    const MAX_BELOW_PX: f64 = 160.0;
    const MIN_DEFICIT_PX: f64 = 12.0;
    let body = dom.body();

    let is_visible_flow = |el: ElId| rhythm_visible_flow(dom, el);
    let overlaps_x = rhythm_overlaps_x;
    let has_own_top_boundary = |el: ElId| rhythm_paints_edge(dom, el, "Top");
    // The nearest previous sibling that lays out a box, looking through
    // `display: contents` and past elements that render nothing.
    let previous_box = |n: ElId| -> Option<ElId> {
        let mut sib = dom.previous_element_sibling(n);
        while let Some(s) = sib {
            if rhythm_is_contents(dom, s) {
                if let Some(inner) = dom.children(s).into_iter().rev().find(|&k| {
                    let r = dom.rect(k);
                    r.width >= 1.0 && r.height >= 1.0
                }) {
                    return Some(inner);
                }
            } else {
                let r = dom.rect(s);
                if dom.style(s, "display") != "none" && r.width >= 1.0 && r.height >= 1.0 {
                    return Some(s);
                }
            }
            sib = dom.previous_element_sibling(s);
        }
        None
    };
    // The eyebrow fold. A label above the heading is part of the heading's
    // cluster whether it is the heading's own sibling or a sibling of a
    // wrapper that starts where the cluster starts: per-text wrappers
    // (`<div><p>Label</p></div><div><h2>…</h2></div>`) are how site builders
    // emit an eyebrow, and missing them measured the gap to the heading's
    // own label. Only a line that reads as a label folds in: `text_size` is
    // the size of the body text it is set against.
    let cluster_top = |h: ElId, rect: &Rect, text_size: f64| -> (ElId, f64) {
        let mut top_el = h;
        let mut top = rect.top;
        let mut cursor = h;
        let mut folded = 0;
        while folded < 3 {
            let Some(sib) = previous_box(cursor) else {
                let Some(p) = dom.parent(cursor) else { break };
                if Some(p) == body {
                    break;
                }
                let starts_with_cluster = rhythm_is_contents(dom, p)
                    || ((dom.rect(p).top - top).abs() <= 1.0 && !has_own_top_boundary(p));
                if !starts_with_cluster {
                    break;
                }
                cursor = p;
                continue;
            };
            // An empty spacer is gap, as it is to `edge_above`: step over it
            // so a label above it still folds in.
            if rhythm_is_spacer(dom, sib) {
                cursor = sib;
                continue;
            }
            if !is_visible_flow(sib) {
                break;
            }
            let sr = dom.rect(sib);
            if !overlaps_x(&sr, rect) {
                break;
            }
            let gap = top - sr.bottom;
            if gap < 0.0 || gap >= 28.0 || sr.height > 60.0 {
                break;
            }
            let text = js::trim(&dom.text_content(sib)).to_string();
            let text_len = utf16_len(&text);
            // A label has words. A rule, a spacer or an icon above the heading
            // is a block of its own, and the walk above judges it as one.
            if text_len == 0 {
                break;
            }
            // A heading above is a title of its own, never this heading's label.
            if matches!(tag_lower(dom, sib).as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                break;
            }
            if text_len > 80 || !rhythm_reads_as_eyebrow(dom, sib, h, text_size) {
                break;
            }
            top_el = sib;
            top = sr.top;
            cursor = sib;
            folded += 1;
        }
        (top_el, top)
    };
    // The block above, and where its content ends. The gap a reader sees
    // runs from that content to the cluster's own first line: empty spacer
    // boxes are part of the gap, so is the bottom padding of a block that
    // paints no edge, and so is top padding on the cluster's first box.
    let edge_above = |start_el: ElId, top: f64, rect: &Rect| -> Option<(f64, ElId)> {
        let pick = |sr: &Rect| sr.bottom <= top + 2.0;
        let inset = if rhythm_paints_edge(dom, start_el, "Top") {
            0.0
        } else {
            math_max(0.0, style_px(dom, start_el, "paddingTop"))
        };
        let mut node = Some(start_el);
        while let Some(n) = node {
            if Some(n) == body {
                break;
            }
            // The nearest block above, not the first in source order: flex and
            // grid `order` can lay siblings out in another sequence.
            let mut nearest: Option<(f64, ElId)> = None;
            let mut sib = dom.previous_element_sibling(n);
            while let Some(s) = sib {
                if let Some(b) = rhythm_flow_box(dom, s, rect, true, &pick) {
                    if !rhythm_is_spacer(dom, b) {
                        let bottom = rhythm_content_bottom(dom, b);
                        if nearest.map_or(true, |(nb, _)| bottom > nb) {
                            nearest = Some((bottom, b));
                        }
                    }
                }
                sib = dom.previous_element_sibling(s);
            }
            if let Some((bottom, b)) = nearest {
                return Some((bottom - inset, b));
            }
            let parent = dom.parent(n);
            let Some(p) = parent else { return None };
            if Some(p) == body {
                return None;
            }
            if has_own_top_boundary(p) {
                return None;
            }
            node = Some(p);
        }
        None
    };
    // The content the heading introduces: the nearest block below it, with
    // empty spacer boxes counted as space. When the heading is the last thing
    // in its box, the walk leaves the box, and the box's bottom padding and
    // margin are space below: a padded section header, a title row stretched
    // by a button or an icon beside the heading. The exception is a box that
    // ends visibly (a bottom border, a shadow, a band of its own color) or is
    // one of a run of like boxes (accordion rows, list items, cards in a
    // grid). Then the heading ends its box, what follows belongs to the next
    // box, and there is nothing below to measure.
    let edge_below = |h: ElId, rect: &Rect| -> Option<(f64, ElId)> {
        let pick = |sr: &Rect| sr.top >= rect.bottom - 2.0;
        let mut node = Some(h);
        while let Some(n) = node {
            if Some(n) == body {
                break;
            }
            let mut nearest: Option<(f64, ElId)> = None;
            let mut sib = dom.next_element_sibling(n);
            while let Some(s) = sib {
                if let Some(b) = rhythm_flow_box(dom, s, rect, false, &pick) {
                    if !rhythm_is_spacer(dom, b) {
                        let t = dom.rect(b).top;
                        if nearest.map_or(true, |(nt, _)| t < nt) {
                            nearest = Some((t, b));
                        }
                    }
                }
                sib = dom.next_element_sibling(s);
            }
            if nearest.is_some() {
                return nearest;
            }
            let p = dom.parent(n)?;
            if Some(p) != body
                && !rhythm_is_contents(dom, p)
                && (rhythm_draws_bottom_edge(dom, p) || rhythm_repeats(dom, p, h))
            {
                return None;
            }
            node = Some(p);
        }
        None
    };
    let inside_small_card = |h: ElId| -> bool {
        let mut cur = dom.parent(h);
        while let Some(c) = cur {
            if Some(c) == body {
                break;
            }
            if is_card_like_dom(dom, c) {
                let cr = dom.rect(c);
                if cr.height < CARD_EXEMPT_HEIGHT {
                    return true;
                }
            }
            cur = dom.parent(c);
        }
        false
    };

    struct Cand {
        el: ElId,
        tag: String,
        text: String,
        above: f64,
        below: f64,
    }
    let mut candidates: Vec<Cand> = Vec::new();
    for h in dom.query_all(None, "h2, h3, h4").unwrap_or_default() {
        if !is_visible_flow(h) {
            continue;
        }
        let text = collapse_ws(js::trim(&dom.text_content(h)));
        if utf16_len(&text) < 3 {
            continue;
        }
        let rect = dom.rect(h);
        // A heading that draws its own top rule or band is separated from
        // whatever sits above it by that edge.
        if has_own_top_boundary(h) {
            continue;
        }
        let Some((below_top, below_el)) = edge_below(h, &rect) else { continue };
        // The body text a label is set against: the page's own text size, or
        // the text the heading introduces when that is set larger.
        let text_size = math_max(
            body.map_or(16.0, |b| rhythm_font_size(dom, b)),
            rhythm_text_size(dom, below_el),
        );
        let (top_el, top) = cluster_top(h, &rect, text_size);
        let Some((above_bottom, above_el)) = edge_above(top_el, top, &rect) else { continue };
        if inside_small_card(h) {
            continue;
        }
        let above = math_max(0.0, top - above_bottom);
        let below = math_max(0.0, below_top - rect.bottom);
        if below < 6.0 || below > MAX_BELOW_PX {
            continue;
        }
        if above < below * 0.75
            && below - above >= MIN_DEFICIT_PX
            && !rhythm_block_separates(dom, above_el)
        {
            candidates.push(Cand {
                el: h,
                tag: tag_lower(dom, h),
                text: slice_utf16_prefix(&text, 60),
                above,
                below,
            });
        }
    }

    if candidates.len() < MIN_VIOLATIONS {
        return Vec::new();
    }
    let n = candidates.len();
    candidates
        .into_iter()
        .map(|c| ElFinding {
            el: Some(c.el),
            finding: BrowserFinding::new(
                "heading-rhythm",
                format!(
                    "{} \"{}\" has {}px above vs {}px below — it reads as bound to the block above ({} headings on page)",
                    c.tag,
                    c.text,
                    number_to_string(math_round(c.above)),
                    number_to_string(math_round(c.below)),
                    n
                ),
            ),
        })
        .collect()
}

/// JS: checks.mjs#checkCreamPalette(document) (browser path)
pub fn check_cream_palette(dom: &dyn Dom) -> Vec<RuleHit> {
    let mut findings = Vec::new();
    let Some(body) = dom.body() else { return findings };
    let html = dom.document_element();

    let mut bg = super::background::read_own_background_color(dom, body);
    if bg.is_none() || bg.map_or(false, |c| c.a == Some(0.0)) {
        if let Some(h) = html {
            bg = super::background::read_own_background_color(dom, h);
        }
    }
    if is_cream_color(bg.as_ref()) {
        let c = bg.unwrap();
        findings.push(RuleHit::new(
            "cream-palette",
            format!(
                "cream/beige page background rgb({}, {}, {})",
                number_to_string(c.r),
                number_to_string(c.g),
                number_to_string(c.b)
            ),
        ));
        return findings;
    }

    for el in [Some(body), html] {
        let cls = el.and_then(|e| dom.attr(e, "class"));
        // JS `el && el.getAttribute ? el.getAttribute('class') : ''` then
        // creamFromClassList(null) → null.
        if let Some(tok) = cream_from_class_list(cls.as_deref()) {
            findings.push(RuleHit::new(
                "cream-palette",
                format!("cream/beige page background (Tailwind {})", tok),
            ));
            break;
        }
    }
    findings
}

const HIDDEN_TEXT_EXCLUDE_TAGS: &[&str] = &[
    "script", "style", "noscript", "template", "title", "head", "meta", "link", "option",
    "optgroup", "select", "datalist", "dialog",
];

#[derive(Clone, Copy, PartialEq)]
enum HiddenState {
    Visible,
    Invisible,
    Excluded,
}

/// JS: checks.mjs#measureHiddenTextDOM()
pub fn measure_hidden_text_dom(dom: &dyn Dom) -> HiddenTextMeasure {
    let root = dom.document_element();
    let mut cache: std::collections::HashMap<ElId, HiddenState> = std::collections::HashMap::new();

    fn state_of(
        dom: &dyn Dom,
        root: Option<ElId>,
        cache: &mut std::collections::HashMap<ElId, HiddenState>,
        el: Option<ElId>,
    ) -> HiddenState {
        let Some(el) = el else { return HiddenState::Visible };
        if Some(el) == root {
            return HiddenState::Visible;
        }
        if let Some(s) = cache.get(&el) {
            return *s;
        }
        let tag = tag_lower(dom, el);
        let state = if HIDDEN_TEXT_EXCLUDE_TAGS.contains(&tag.as_str()) {
            HiddenState::Excluded
        } else {
            let parent_state = state_of(dom, root, cache, dom.parent(el));
            if parent_state == HiddenState::Excluded {
                HiddenState::Excluded
            } else {
                let display = dom.style(el, "display");
                let cv = js::to_lower_case(&dom.style(el, "contentVisibility"));
                if display == "none"
                    || dom.hidden_prop(el)
                    || dom.attr(el, "aria-hidden").as_deref() == Some("true")
                    || cv == "hidden"
                {
                    HiddenState::Excluded
                } else if parent_state == HiddenState::Invisible
                    || pf0(&dom.style(el, "opacity")) <= 0.02
                    || HIDDEN_VIS_RE.is_match(&dom.style(el, "visibility"))
                {
                    HiddenState::Invisible
                } else {
                    HiddenState::Visible
                }
            }
        };
        cache.insert(el, state);
        state
    }

    let mut total_chars = 0.0f64;
    let mut hidden_chars = 0.0f64;
    let mut hidden_samples: Vec<String> = Vec::new();
    for el in dom.query_all(None, "body *").unwrap_or_default() {
        let mut len = 0usize;
        for t in dom.direct_text_nodes(el) {
            len += utf16_len(js::trim(&collapse_ws(&t)));
        }
        if len == 0 {
            continue;
        }
        let state = state_of(dom, root, &mut cache, Some(el));
        if state == HiddenState::Excluded {
            continue;
        }
        total_chars += len as f64;
        if state == HiddenState::Invisible {
            hidden_chars += len as f64;
            if hidden_samples.len() < 3 {
                let text = slice_utf16_prefix(js::trim(&collapse_ws(&dom.text_content(el))), 40);
                if !text.is_empty() {
                    hidden_samples.push(text);
                }
            }
        }
    }
    HiddenTextMeasure {
        total_chars,
        hidden_chars,
        hidden_samples,
    }
}

/// JS `isScroller(s)` from checkEdgeFlushCardsDOM.
fn is_scroller(dom: &dyn Dom, el: ElId) -> bool {
    SCROLL_RE.is_match(&dom.style(el, "overflowX")) || SCROLL_RE.is_match(&dom.style(el, "overflow"))
}

/// JS: checks.mjs#checkEdgeFlushCardsDOM()
pub fn check_edge_flush_cards_dom(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut findings = Vec::new();
    let vh = {
        let h = dom.inner_height();
        if num_truthy(h) {
            h
        } else {
            800.0
        }
    };
    let scroll_y = {
        let y = dom.scroll_y();
        if num_truthy(y) {
            y
        } else {
            0.0
        }
    };

    for scroller in dom.query_all(None, "*").unwrap_or_default() {
        if !is_scroller(dom, scroller) {
            continue;
        }
        if dom.scroll_width(scroller) <= dom.client_width(scroller) + 8.0 {
            continue;
        }
        if dom.scroll_left(scroller) > 4.0 {
            continue;
        }
        let sc_rect = dom.rect(scroller);
        if sc_rect.width < 120.0 || sc_rect.height < 60.0 {
            continue;
        }
        if sc_rect.top + scroll_y > 2.0 * vh {
            continue;
        }
        let content_left = sc_rect.left + dom.client_left(scroller);
        let content_right = content_left + dom.client_width(scroller);

        struct Flush {
            card: ElId,
            edge: &'static str,
            gap: f64,
        }
        let mut flush: Vec<Flush> = Vec::new();
        for card in dom.query_all(Some(scroller), "*").unwrap_or_default() {
            if !is_rendered_for_browser_rule(dom, card) {
                continue;
            }
            let mut owner = dom.parent(card);
            while let Some(o) = owner {
                if o == scroller || is_scroller(dom, o) {
                    break;
                }
                owner = dom.parent(o);
            }
            if owner != Some(scroller) {
                continue;
            }
            let rect = dom.rect(card);
            if rect.width < 80.0 || rect.height < 40.0 {
                continue;
            }
            let bg = parse_any_color(Some(&dom.style(card, "backgroundColor")));
            let has_bg = bg.map_or(false, |c| c.alpha_or_one() > 0.5);
            let border_sides = ["Top", "Right", "Bottom", "Left"]
                .iter()
                .filter(|s| style_px(dom, card, &format!("border{s}Width")) > 0.0)
                .count();
            if !has_bg && border_sides < 2 {
                continue;
            }
            let left_gutter = rect.left - content_left;
            let right_gap = content_right - rect.right;
            let flush_right = left_gutter >= 6.0 && right_gap < 8.0 && right_gap > -24.0;
            let flush_left = right_gap >= 6.0 && left_gutter < 8.0 && left_gutter > -24.0;
            if !flush_right && !flush_left {
                continue;
            }
            flush.push(Flush {
                card,
                edge: if flush_right { "right" } else { "left" },
                gap: math_round(if flush_right { right_gap } else { left_gutter }),
            });
        }
        if flush.is_empty() {
            continue;
        }
        let mut worst = &flush[0];
        for f in &flush[1..] {
            if f.gap < worst.gap {
                worst = f;
            }
        }
        findings.push(ElFinding {
            el: Some(scroller),
            finding: BrowserFinding::new(
                "edge-flush-cards",
                format!(
                    "{} card{} flush against the {} edge of {} at rest ({}px gap, e.g. {})",
                    flush.len(),
                    if flush.len() == 1 { "" } else { "s" },
                    worst.edge,
                    class_selector(dom, scroller),
                    number_to_string(worst.gap),
                    class_selector(dom, worst.card)
                ),
            ),
        });
    }
    findings
}

/// JS: checks.mjs#isLayeredElement(el)
pub fn is_layered_element(dom: &dyn Dom, el: ElId) -> bool {
    let body = dom.body();
    let mut cur = Some(el);
    while let Some(c) = cur {
        if Some(c) == body {
            break;
        }
        let pos = dom.style(c, "position");
        let pos = if pos.is_empty() { "static".to_string() } else { pos };
        if pos == "absolute" || pos == "fixed" || pos == "sticky" {
            return true;
        }
        cur = dom.parent(c);
    }
    false
}

/// JS: checks.mjs#elementDirectText(el)
pub fn element_direct_text(dom: &dyn Dom, el: ElId) -> String {
    js::trim(&direct_text(dom, el)).to_string()
}

/// JS: checks.mjs#isPaintedForOcclusion(el)
pub fn is_painted_for_occlusion(dom: &dyn Dom, el: ElId) -> bool {
    // Closed <details> content is hidden through ::details-content, which no
    // element's computed style reports, so the browser's own verdict comes first.
    if dom.check_visibility(el) == Some(false) {
        return false;
    }
    let mut cur = Some(el);
    while let Some(c) = cur {
        let visibility = js::to_lower_case(&dom.style(c, "visibility"));
        if dom.style(c, "display") == "none" || visibility == "hidden" || visibility == "collapse" {
            return false;
        }
        if pf0(&dom.style(c, "opacity")) <= 0.05 {
            return false;
        }
        if js::to_lower_case(&dom.style(c, "contentVisibility")) == "hidden" {
            return false;
        }
        cur = dom.parent(c);
    }
    true
}

const OCCLUSION_TEXT_SKIP_TAGS: &[&str] = &["script", "style", "noscript", "template", "title"];

/// JS: checks.mjs#checkTextOcclusionDOM()
pub fn check_text_occlusion_dom(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut findings = Vec::new();
    let mut seen_victims: Vec<ElId> = Vec::new();
    let vw = {
        let w = dom.inner_width();
        if num_truthy(w) {
            w
        } else {
            1280.0
        }
    };
    let vh = {
        let h = dom.inner_height();
        if num_truthy(h) {
            h
        } else {
            800.0
        }
    };
    let body = dom.body();

    let is_floated = |el: ElId| -> bool {
        let f = {
            let a = dom.style(el, "cssFloat");
            if !a.is_empty() {
                a
            } else {
                let b = dom.style(el, "float");
                if !b.is_empty() {
                    b
                } else {
                    "none".to_string()
                }
            }
        };
        let f = js::to_lower_case(&f);
        f == "left" || f == "right"
    };
    let is_marqueeish = |el: ElId| -> bool {
        if dom.tag_name(el) == "MARQUEE" {
            return true;
        }
        let ident = format!(
            "{} {}",
            dom.attr(el, "class").unwrap_or_default(),
            dom.attr(el, "id").unwrap_or_default()
        );
        if MARQUEE_IDENT_RE.is_match(&ident) {
            return true;
        }
        let anim = js::to_lower_case(&dom.style(el, "animationName"));
        MARQUEE_ANIM_RE.is_match(&anim)
    };
    let is_pinned_overlay = |el: ElId| -> bool {
        let mut cur = Some(el);
        while let Some(c) = cur {
            if Some(c) == body {
                break;
            }
            let pos = dom.style(c, "position");
            let pos = if pos.is_empty() { "static".to_string() } else { pos };
            if pos == "fixed" || pos == "sticky" {
                return true;
            }
            cur = dom.parent(c);
        }
        false
    };

    // JS `paintedRect(el, rect)`: the part of an element that is actually
    // painted, after every scrolling or clipping ancestor has had its say.
    // getBoundingClientRect reports where a box would be if nothing cut it
    // off; the elementFromPoint probe must only sample coordinates the text
    // is painted at (sticky footers under scroll regions otherwise read as
    // burying the clipped-away half). Border box on purpose: it errs toward
    // probing.
    let painted_rect = |el: ElId, rect: &Rect| -> Option<Rect> {
        let mut left = rect.left;
        let mut top = rect.top;
        let mut right = rect.right;
        let mut bottom = rect.bottom;
        let doc_el = dom.document_element();
        let mut cur = dom.parent(el);
        while let Some(c) = cur {
            if Some(c) == doc_el {
                break;
            }
            let ov = |k: &str| {
                let v = dom.style(c, k);
                if v.is_empty() {
                    "visible".to_string()
                } else {
                    v
                }
            };
            let clips_x = ov("overflowX") != "visible";
            let clips_y = ov("overflowY") != "visible";
            if !clips_x && !clips_y {
                cur = dom.parent(c);
                continue;
            }
            let b = dom.rect(c);
            if clips_x {
                left = js::math_max(left, b.left);
                right = js::math_min(right, b.right);
            }
            if clips_y {
                top = js::math_max(top, b.top);
                bottom = js::math_min(bottom, b.bottom);
            }
            if right - left < 1.0 || bottom - top < 1.0 {
                return None;
            }
            cur = dom.parent(c);
        }
        Some(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
            top,
            right,
            bottom,
            left,
        })
    };

    struct TextEl {
        el: ElId,
        rect: Rect,
        text: String,
    }
    let mut text_els: Vec<TextEl> = Vec::new();
    for el in dom.query_all(None, "body *").unwrap_or_default() {
        let tag = tag_lower(dom, el);
        if OCCLUSION_TEXT_SKIP_TAGS.contains(&tag.as_str()) {
            continue;
        }
        let in_svg = closest_or_none(dom, el, "svg").is_some();
        if in_svg && tag != "text" {
            continue;
        }
        let text = if in_svg {
            js::trim(&dom.text_content(el)).to_string()
        } else {
            element_direct_text(dom, el)
        };
        if utf16_len(&text) < 2 {
            continue;
        }
        if !is_painted_for_occlusion(dom, el) {
            continue;
        }
        if effective_opacity_dom(dom, el) <= 0.02 {
            continue;
        }
        let full = dom.rect(el);
        if full.width < 6.0 || full.height < 6.0 {
            continue;
        }
        // Probe only where the text is on screen. A run clipped down to a
        // sliver is dropped rather than sampled.
        let Some(rect) = painted_rect(el, &full) else {
            continue;
        };
        if rect.width < 6.0 || rect.height < 6.0 {
            continue;
        }
        if rect.bottom <= 0.0 || rect.top >= vh {
            continue;
        }
        text_els.push(TextEl { el, rect, text });
    }

    for victim in &text_els {
        let el = victim.el;
        let rect = &victim.rect;
        let text = &victim.text;
        if seen_victims.contains(&el) {
            continue;
        }
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
            continue;
        }

        let cols = math_max(6.0, math_min(30.0, math_round(rect.width / 12.0)));
        let rows = math_max(1.0, math_min(4.0, math_round(rect.height / 14.0)));
        let mut total = 0usize;
        let mut occluded = 0usize;
        let mut occluder_el: Option<ElId> = None;
        let mut occluder_kind = "";
        let mut i = 0.0;
        while i < cols {
            let x = rect.left + rect.width * ((i + 0.5) / cols);
            i += 1.0;
            if x < 1.0 || x > vw - 1.0 {
                continue;
            }
            let mut j = 0.0;
            while j < rows {
                let y = rect.top + rect.height * ((j + 0.5) / rows);
                j += 1.0;
                if y < 1.0 || y > vh - 1.0 {
                    continue;
                }
                total += 1;
                let Some(top) = dom.element_from_point(x, y) else { continue };
                if top == el || dom.contains(el, top) || dom.contains(top, el) {
                    continue;
                }
                if is_floated(top) || is_marqueeish(top) || is_pinned_overlay(top) {
                    continue;
                }
                if effective_opacity_dom(dom, top) <= 0.02 {
                    continue;
                }
                let top_tag = tag_lower(dom, top);
                if matches!(top_tag.as_str(), "img" | "video" | "canvas" | "picture") {
                    continue;
                }
                let top_has_text = !element_direct_text(dom, top).is_empty()
                    || closest_or_none(dom, top, "svg").is_some();
                let top_style = ElStyle { dom, el: top };
                if is_opaque_decorated_box(Some(&top_style)) {
                    occluded += 1;
                    if occluder_el.is_none() {
                        occluder_el = Some(top);
                        occluder_kind = "box";
                    }
                } else if top_has_text {
                    occluded += 1;
                    if occluder_el.is_none() {
                        occluder_el = Some(top);
                        occluder_kind = "text";
                    }
                }
            }
        }
        let Some(occ) = occluder_el else { continue };
        if total == 0 {
            continue;
        }
        let occ_frac = occluded as f64 / total as f64;
        if occ_frac < (if occluder_kind == "text" { 0.45 } else { 0.3 }) {
            continue;
        }

        if occluder_kind == "text" {
            let victim_svg = closest_or_none(dom, el, "svg");
            let occ_svg = closest_or_none(dom, occ, "svg");
            if victim_svg.is_some() && occ_svg.is_some() && victim_svg == occ_svg {
                continue;
            }
            if !is_layered_element(dom, el) && !is_layered_element(dom, occ) {
                continue;
            }
        }
        seen_victims.push(el);
        findings.push(ElFinding {
            el: Some(el),
            finding: BrowserFinding::new(
                "text-occlusion",
                format!(
                    "{} \"{}\" is {}% covered by {} ({})",
                    class_selector(dom, el),
                    slice_utf16_prefix(text, 24),
                    number_to_string(math_round(occ_frac * 100.0)),
                    if occluder_kind == "text" { "overlapping text" } else { "an opaque element" },
                    class_selector(dom, occ)
                ),
            ),
        });
    }

    // (ii) Headline overhanging an opaque card.
    struct Card {
        el: ElId,
        rect: Rect,
    }
    let mut cards: Vec<Card> = Vec::new();
    for el in dom.query_all(None, "body *").unwrap_or_default() {
        if closest_or_none(dom, el, "svg").is_some() {
            continue;
        }
        if !is_painted_for_occlusion(dom, el) {
            continue;
        }
        let bg = parse_any_color(Some(&dom.style(el, "backgroundColor")));
        let bg_img = dom.style(el, "backgroundImage");
        let Some(bg) = bg else { continue };
        if bg.alpha_or_one() <= 0.7 {
            continue;
        }
        if !bg_img.is_empty() && bg_img != "none" && GRADIENT_URL_RE.is_match(&bg_img) {
            continue;
        }
        let has_border = ["Top", "Right", "Bottom", "Left"]
            .iter()
            .any(|s| style_px(dom, el, &format!("border{s}Width")) > 0.0);
        let bs = dom.style(el, "boxShadow");
        let has_shadow = !bs.is_empty() && bs != "none";
        if !has_border && !has_shadow {
            continue;
        }
        if is_pinned_overlay(el) {
            continue;
        }
        let cr = dom.rect(el);
        if cr.width < 100.0 || cr.width > 0.8 * vw || cr.height < 60.0 {
            continue;
        }
        cards.push(Card { el, rect: cr });
    }
    for victim in &text_els {
        let el = victim.el;
        let rect = &victim.rect;
        let text = &victim.text;
        if seen_victims.contains(&el) {
            continue;
        }
        let font_size = {
            let n = parse_float(&dom.style(el, "fontSize"));
            if num_truthy(n) {
                n
            } else {
                16.0
            }
        };
        if font_size < 40.0 {
            continue;
        }
        let mut line_height = parse_float(&dom.style(el, "lineHeight"));
        if !line_height.is_finite() {
            line_height = font_size * 1.2;
        }
        let center_x = rect.left + rect.width / 2.0;
        for card in &cards {
            if card.el == el || dom.contains(el, card.el) || dom.contains(card.el, el) {
                continue;
            }
            let ix = math_max(0.0, math_min(rect.right, card.rect.right) - math_max(rect.left, card.rect.left));
            let iy = math_max(0.0, math_min(rect.bottom, card.rect.bottom) - math_max(rect.top, card.rect.top));
            if ix < 8.0 || iy < 0.5 * line_height {
                continue;
            }
            if center_x >= card.rect.left && center_x <= card.rect.right {
                continue;
            }
            if ix > 0.5 * rect.width {
                continue;
            }
            seen_victims.push(el);
            findings.push(ElFinding {
                el: Some(el),
                finding: BrowserFinding::new(
                    "text-occlusion",
                    format!(
                        "{} \"{}\" overhangs {} by {}px — the headline and the card collide",
                        class_selector(dom, el),
                        slice_utf16_prefix(text, 24),
                        class_selector(dom, card.el),
                        number_to_string(math_round(ix))
                    ),
                ),
            });
            break;
        }
    }

    // (iii) Inline padding leak.
    for el in dom.query_all(None, "body *").unwrap_or_default() {
        if closest_or_none(dom, el, "svg").is_some() {
            continue;
        }
        if !is_painted_for_occlusion(dom, el) {
            continue;
        }
        if dom.style(el, "display") != "inline" {
            continue;
        }
        let Some(bg) = parse_any_color(Some(&dom.style(el, "backgroundColor"))) else { continue };
        if bg.alpha_or_one() <= 0.6 {
            continue;
        }
        let pad_top = style_px(dom, el, "paddingTop");
        let pad_bottom = style_px(dom, el, "paddingBottom");
        if pad_top + pad_bottom < 24.0 {
            continue;
        }
        let rect = dom.rect(el);
        if rect.width < 12.0 || rect.height < 24.0 {
            continue;
        }
        let font_size = {
            let n = parse_float(&dom.style(el, "fontSize"));
            if num_truthy(n) {
                n
            } else {
                16.0
            }
        };
        let mut line_height = parse_float(&dom.style(el, "lineHeight"));
        if !line_height.is_finite() {
            line_height = font_size * 1.4;
        }
        if rect.height < 2.2 * line_height {
            continue;
        }
        if seen_victims.contains(&el) {
            continue;
        }
        let mut overlaps: Option<ElId> = None;
        let siblings = match dom.parent(el) {
            Some(p) => dom.children(p),
            None => Vec::new(),
        };
        for other in siblings {
            if other == el || dom.contains(el, other) || dom.contains(other, el) {
                continue;
            }
            if dom.style(other, "display") == "none" {
                continue;
            }
            let o_rect = dom.rect(other);
            let ix = math_max(0.0, math_min(rect.right, o_rect.right) - math_max(rect.left, o_rect.left));
            let iy = math_max(0.0, math_min(rect.bottom, o_rect.bottom) - math_max(rect.top, o_rect.top));
            if ix > 4.0 && iy > 4.0 && !js::trim(&dom.text_content(other)).is_empty() {
                overlaps = Some(other);
                break;
            }
        }
        seen_victims.push(el);
        findings.push(ElFinding {
            el: Some(el),
            finding: BrowserFinding::new(
                "text-occlusion",
                format!(
                    "{} is an inline element whose opaque fill leaks {}px past its line{}",
                    class_selector(dom, el),
                    number_to_string(math_round(rect.height)),
                    match overlaps {
                        Some(o) => format!(" onto {}", class_selector(dom, o)),
                        None => String::new(),
                    }
                ),
            ),
        });
    }

    findings
}

/// JS: checks.mjs#checkFirstViewportColumnOverflowDOM()
pub fn check_first_viewport_column_overflow_dom(dom: &dyn Dom) -> Vec<ElFinding> {
    let mut findings = Vec::new();
    let vw = {
        let w = dom.inner_width();
        if num_truthy(w) {
            w
        } else {
            1280.0
        }
    };
    let vh = {
        let h = dom.inner_height();
        if num_truthy(h) {
            h
        } else {
            800.0
        }
    };
    let scroll_y = {
        let y = dom.scroll_y();
        if num_truthy(y) {
            y
        } else {
            0.0
        }
    };

    for el in dom.query_all(None, "body *").unwrap_or_default() {
        if !MULTI_COL_RE.is_match(&dom.style(el, "display")) {
            continue;
        }
        let rect = dom.rect(el);
        if rect.width < 0.5 * vw {
            continue;
        }
        let page_top = rect.top + scroll_y;
        let page_bottom = page_top + rect.height;
        if page_top >= vh * 0.9 || page_bottom <= vh {
            continue;
        }

        struct Col {
            top: f64,
            content_h: f64,
        }
        let mut cols: Vec<Col> = Vec::new();
        for child in dom.children(el) {
            if dom.style(child, "display") == "none" {
                continue;
            }
            let pos = dom.style(child, "position");
            if pos == "absolute" || pos == "fixed" {
                continue;
            }
            let cr = dom.rect(child);
            let w_share = cr.width / rect.width;
            if w_share < 0.25 || w_share > 0.9 {
                continue;
            }
            if cr.height < 40.0 {
                continue;
            }
            let mut content_bottom = cr.top;
            for d in dom.query_all(Some(child), "*").unwrap_or_default() {
                let dpos = dom.style(d, "position");
                if dpos == "absolute" || dpos == "fixed" {
                    continue;
                }
                if dom.style(d, "display") == "none" || dom.style(d, "visibility") == "hidden" {
                    continue;
                }
                let dr = dom.rect(d);
                if dr.width > 0.0 && dr.height > 0.0 {
                    content_bottom = math_max(content_bottom, dr.bottom);
                }
            }
            cols.push(Col {
                top: cr.top,
                content_h: content_bottom - cr.top,
            });
        }
        if cols.len() < 2 {
            continue;
        }
        cols.sort_by(|a, b| {
            b.content_h
                .partial_cmp(&a.content_h)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let tall = &cols[0];
        let shortest = &cols[cols.len() - 1];
        if (tall.top - shortest.top).abs() > 0.25 * vh {
            continue;
        }
        if tall.content_h <= vh * 1.4 {
            continue;
        }
        if shortest.content_h > vh {
            continue;
        }

        findings.push(ElFinding {
            el: Some(el),
            finding: BrowserFinding::new(
                "first-viewport-column-overflow",
                format!(
                    "{} opens the page with one column running {}% of the viewport tall while a sibling fits in {}% — the fold falls deep inside the section",
                    class_selector(dom, el),
                    number_to_string(math_round(tall.content_h / vh * 100.0)),
                    number_to_string(math_round(shortest.content_h / vh * 100.0))
                ),
            ),
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    /// FakeDom matches selectors by exact string; `body *` (every descendant
    /// of body) has to be declared per element.
    fn mark_body_descendants(d: &mut FakeDom) {
        let body = d.body.unwrap();
        // a real body paints
        d.set_styles(body, &[("display", "block"), ("opacity", "1"), ("visibility", "visible")]);
        let html = d.document_element.unwrap();
        d.set_styles(html, &[("display", "block"), ("opacity", "1"), ("visibility", "visible")]);
        let n = d.els.len() as ElId;
        for id in 1..n {
            if id != body && d.contains(body, id) {
                d.add_selector(id, "body *");
            }
        }
    }

    fn card_styles(d: &mut FakeDom, el: ElId, bg: &str) {
        d.set_styles(
            el,
            &[
                ("boxShadow", "rgba(0, 0, 0, 0.1) 0px 2px 4px 0px"),
                ("borderRadius", "8px"),
                ("backgroundColor", bg),
                ("position", "static"),
                ("display", "block"),
                ("visibility", "visible"),
                ("opacity", "1"),
            ],
        );
    }

    #[test]
    fn typography_overused_and_flat() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        for i in 0..20 {
            let p = d.add(Some(body), "p");
            d.add_text(p, "text");
            d.set_style(p, "fontFamily", "\"Inter\", sans-serif");
            d.set_style(p, "fontSize", if i % 2 == 0 { "16px" } else { "18px" });
        }
        let s = d.add(Some(body), "span");
        d.add_text(s, "x");
        d.set_style(s, "fontFamily", "Georgia");
        d.set_style(s, "fontSize", "24px");
        let f = check_typography(&d);
        // One `body` role is under TYPE_HIERARCHY_MIN_ROLES, so the flat-type
        // rule abstains and only the font finding stands (#702).
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].type_, "overused-font");
        assert_eq!(f[0].detail, "Primary font: inter (95% of text)");
    }

    #[test]
    fn flat_type_hierarchy_roles() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        for (tag, size) in [("p", "16px"), ("p", "16px"), ("h2", "17px"), ("h1", "18px")] {
            let el = d.add(Some(body), tag);
            d.add_text(el, "text");
            d.set_style(el, "fontSize", size);
        }
        let f = check_typography(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].type_, "flat-type-hierarchy");
        assert_eq!(
            f[0].detail,
            "Role sizes: body 16px, h2 17px, h1 18px (largest adjacent step 1.06:1; target 1.25:1)"
        );

        // A clear step at any adjacent pair clears the rule.
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        for (tag, size) in [("p", "16px"), ("h2", "24px"), ("h1", "40px")] {
            let el = d.add(Some(body), tag);
            d.add_text(el, "text");
            d.set_style(el, "fontSize", size);
        }
        assert!(check_typography(&d).is_empty());

        // A hidden ancestor takes its text out of the sample.
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        for (tag, size) in [("p", "16px"), ("p", "16px"), ("h2", "17px")] {
            let el = d.add(Some(body), tag);
            d.add_text(el, "text");
            d.set_style(el, "fontSize", size);
        }
        let wrap = d.add(Some(body), "div");
        d.set_style(wrap, "display", "none");
        let h1 = d.add(Some(wrap), "h1");
        d.add_text(h1, "text");
        d.set_style(h1, "fontSize", "18px");
        assert!(check_typography(&d).is_empty());
    }

    #[test]
    fn nested_cards() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let outer = d.add(Some(body), "div");
        card_styles(&mut d, outer, "rgb(255, 255, 255)");
        d.set_rect(outer, 0.0, 0.0, 400.0, 300.0);
        let inner = d.add(Some(outer), "div");
        card_styles(&mut d, inner, "rgb(250, 250, 250)");
        d.set_rect(inner, 10.0, 10.0, 200.0, 100.0);
        d.add_text(inner, "Some card body text");
        d.add_text(outer, "Outer text longer than ten");
        let f = check_layout(&d);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].el, Some(inner));
        assert_eq!(f[0].finding.detail, "Card inside card");
        d.set_style(inner, "position", "absolute");
        assert!(check_layout(&d).is_empty());
    }

    #[test]
    fn heading_rhythm_two_violations() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let sec = d.add(Some(body), "section");
        d.set_styles(sec, &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("borderTopWidth", "0px"), ("boxShadow", "none")]);
        d.set_rect(sec, 0.0, 0.0, 800.0, 1000.0);
        let mut y = 0.0;
        for i in 0..2 {
            let p0 = d.add(Some(sec), "p");
            d.add_text(p0, "Intro paragraph text that runs well past forty characters");
            d.set_styles(p0, &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static"), ("fontSize", "20px")]);
            d.set_rect(p0, 0.0, y, 800.0, 20.0);
            y += 20.0 + 8.0; // 8px above the heading
            let h = d.add(Some(sec), "h2");
            d.add_text(h, &format!("Heading number {i}"));
            d.set_styles(h, &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static"), ("fontSize", "24px")]);
            d.set_rect(h, 0.0, y, 800.0, 30.0);
            y += 30.0 + 40.0; // 40px below
            let p1 = d.add(Some(sec), "p");
            d.add_text(p1, "Body paragraph");
            d.set_styles(p1, &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static"), ("fontSize", "16px")]);
            d.set_rect(p1, 0.0, y, 800.0, 20.0);
            y += 60.0;
        }
        let f = check_heading_rhythm_dom(&d);
        assert_eq!(f.len(), 2, "{f:?}");
        assert_eq!(
            f[0].finding.detail,
            "h2 \"Heading number 0\" has 8px above vs 40px below — it reads as bound to the block above (2 headings on page)"
        );
    }

    /// A `display: contents` wrapper whose first child is a spacer: the
    /// block below the heading is the content after the spacer.
    #[test]
    fn heading_rhythm_reads_past_a_spacer_in_a_contents_wrapper() {
        let shown = [("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static")];
        let build = |spacer_tag: &str| {
            let mut d = FakeDom::new();
            let (_h, body) = d.with_page();
            let sec = d.add(Some(body), "section");
            d.set_styles(sec, &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("position", "static"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("borderTopWidth", "0px"), ("boxShadow", "none")]);
            d.set_rect(sec, 0.0, 0.0, 800.0, 2000.0);
            let mut y = 0.0;
            for i in 0..2 {
                let p0 = d.add(Some(sec), "p");
                d.add_text(p0, "Intro paragraph text that runs well past forty characters");
                d.set_styles(p0, &shown);
                d.set_style(p0, "fontSize", "20px");
                d.set_rect(p0, 0.0, y, 800.0, 20.0);
                y += 28.0;
                let h = d.add(Some(sec), "h2");
                d.add_text(h, &format!("Heading number {i}"));
                d.set_styles(h, &shown);
                d.set_style(h, "fontSize", "24px");
                d.set_rect(h, 0.0, y, 800.0, 30.0);
                y += 30.0;
                let wrap = d.add(Some(sec), "div");
                d.set_style(wrap, "display", "contents");
                let spacer = d.add(Some(wrap), spacer_tag);
                d.set_styles(spacer, &shown);
                d.set_rect(spacer, 0.0, y, 800.0, 16.0);
                y += 40.0;
                let p1 = d.add(Some(wrap), "p");
                d.add_text(p1, "Body paragraph");
                d.set_styles(p1, &shown);
                d.set_style(p1, "fontSize", "16px");
                d.set_rect(p1, 0.0, y, 800.0, 20.0);
                y += 400.0;
            }
            d
        };
        let f = check_heading_rhythm_dom(&build("div"));
        assert_eq!(f.len(), 2, "{f:?}");
        assert_eq!(
            f[0].finding.detail,
            "h2 \"Heading number 0\" has 8px above vs 40px below — it reads as bound to the block above (2 headings on page)"
        );
        // A borderless input in the spacer's place is the block below: it
        // draws its placeholder, which is not DOM text.
        let f = check_heading_rhythm_dom(&build("input"));
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn hidden_text_measure() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let vis = d.add(Some(body), "p");
        d.add_text(vis, "  visible   text ");
        d.set_styles(vis, &[("display", "block"), ("opacity", "1"), ("visibility", "visible")]);
        let hid = d.add(Some(body), "div");
        d.set_styles(hid, &[("display", "block"), ("opacity", "0"), ("visibility", "visible")]);
        let inner = d.add(Some(hid), "span");
        d.add_text(inner, "hidden words here");
        d.set_styles(inner, &[("display", "inline"), ("opacity", "1"), ("visibility", "visible")]);
        let scr = d.add(Some(body), "script");
        d.add_text(scr, "var x = 1;");
        mark_body_descendants(&mut d);
        let m = measure_hidden_text_dom(&d);
        assert_eq!(m.total_chars, 12.0 + 17.0);
        assert_eq!(m.hidden_chars, 17.0);
        assert_eq!(m.hidden_samples, vec!["hidden words here".to_string()]);
    }

    #[test]
    fn edge_flush_cards() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let sc = d.add(Some(body), "div");
        d.set_attr(sc, "class", "rail");
        d.set_styles(sc, &[("overflowX", "auto"), ("overflow", "auto")]);
        d.set_rect(sc, 0.0, 100.0, 600.0, 200.0);
        {
            let e = d.el_mut(sc);
            e.client_width = 600.0;
            e.scroll_width = 1200.0;
            e.scroll_left = 0.0;
            e.client_left = 0.0;
        }
        let card = d.add(Some(sc), "article");
        d.set_attr(card, "class", "card");
        d.set_styles(card, &[("overflowX", "visible"), ("overflow", "visible"), ("backgroundColor", "rgb(255, 255, 255)"), ("borderTopWidth", "0px"), ("borderRightWidth", "0px"), ("borderBottomWidth", "0px"), ("borderLeftWidth", "0px")]);
        d.set_rect(card, 24.0, 110.0, 574.0, 150.0); // right edge at 598 → gap 2
        let f = check_edge_flush_cards_dom(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].el, Some(sc));
        assert_eq!(
            f[0].finding.detail,
            format!(
                "1 card flush against the right edge of {} at rest (2px gap, e.g. {})",
                class_selector(&d, sc),
                class_selector(&d, card)
            )
        );
        d.set_rect(card, 24.0, 110.0, 540.0, 150.0);
        assert!(check_edge_flush_cards_dom(&d).is_empty());
    }

    #[test]
    fn text_occlusion_box_and_inline_leak() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let base = &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("contentVisibility", "visible"), ("position", "static"), ("cssFloat", "none"), ("animationName", "none")][..];
        let txt = d.add(Some(body), "p");
        d.set_attr(txt, "class", "victim");
        d.add_text(txt, "Readable headline");
        d.set_styles(txt, base);
        d.set_styles(txt, &[("fontSize", "16px"), ("overflow", "visible"), ("overflowX", "visible"), ("overflowY", "visible"), ("clip", "auto"), ("clipPath", "none")]);
        d.set_rect(txt, 100.0, 100.0, 240.0, 28.0);
        let boxel = d.add(Some(body), "div");
        d.set_attr(boxel, "class", "cover");
        d.set_styles(boxel, base);
        d.set_styles(boxel, &[("position", "absolute"), ("backgroundColor", "rgb(20, 20, 20)")]);
        d.set_rect(boxel, 100.0, 100.0, 240.0, 28.0);
        // FakeDom's elementsFromPoint returns the last element in document
        // order whose rect contains the point → the box.
        mark_body_descendants(&mut d);
        let f = check_text_occlusion_dom(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].el, Some(txt));
        assert_eq!(
            f[0].finding.detail,
            format!(
                "{} \"Readable headline\" is 100% covered by an opaque element ({})",
                class_selector(&d, txt),
                class_selector(&d, boxel)
            )
        );

        // inline leak
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let wrap = d.add(Some(body), "div");
        d.set_styles(wrap, base);
        d.set_rect(wrap, 0.0, 0.0, 400.0, 200.0);
        let leak = d.add(Some(wrap), "span");
        d.set_attr(leak, "class", "marker");
        d.set_styles(leak, base);
        d.set_styles(leak, &[("display", "inline"), ("backgroundColor", "rgb(255, 0, 0)"), ("paddingTop", "20px"), ("paddingBottom", "20px"), ("fontSize", "16px"), ("lineHeight", "20px")]);
        d.set_rect(leak, 10.0, 10.0, 40.0, 60.0);
        let sib = d.add(Some(wrap), "p");
        d.add_text(sib, "neighbour");
        d.set_attr(sib, "class", "next");
        d.set_styles(sib, base);
        d.set_rect(sib, 0.0, 40.0, 400.0, 20.0);
        mark_body_descendants(&mut d);
        let f = check_text_occlusion_dom(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(
            f[0].finding.detail,
            format!(
                "{} is an inline element whose opaque fill leaks 60px past its line onto {}",
                class_selector(&d, leak),
                class_selector(&d, sib)
            )
        );
    }

    /// A link in a closed <details> keeps its rect but paints nothing; the
    /// hero text behind it must not read as covering it.
    #[test]
    fn text_occlusion_skips_content_the_browser_reports_unpainted() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let base = &[("display", "block"), ("visibility", "visible"), ("opacity", "1"), ("contentVisibility", "visible"), ("position", "static"), ("cssFloat", "none"), ("animationName", "none")][..];
        let details = d.add(Some(body), "details");
        d.set_styles(details, base);
        let link = d.add(Some(details), "a");
        d.add_text(link, "Documentation");
        d.set_styles(link, base);
        d.set_style(link, "position", "absolute");
        d.set_rect(link, 100.0, 100.0, 240.0, 28.0);
        let h1 = d.add(Some(body), "h1");
        d.add_text(h1, "Hero headline");
        d.set_styles(h1, base);
        d.set_rect(h1, 100.0, 100.0, 240.0, 28.0);
        mark_body_descendants(&mut d);

        d.el_mut(link).check_visibility = Some(false);
        assert!(check_text_occlusion_dom(&d).is_empty());

        d.el_mut(link).check_visibility = Some(true);
        let f = check_text_occlusion_dom(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].el, Some(link));
    }

    #[test]
    fn first_viewport_column_overflow() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let grid = d.add(Some(body), "section");
        d.set_attr(grid, "class", "hero");
        d.set_styles(grid, &[("display", "grid")]);
        d.set_rect(grid, 0.0, 0.0, 1280.0, 1400.0);
        let a = d.add(Some(grid), "div");
        d.set_styles(a, &[("display", "block"), ("position", "static")]);
        d.set_rect(a, 0.0, 0.0, 640.0, 1400.0);
        let a_in = d.add(Some(a), "p");
        d.set_styles(a_in, &[("display", "block"), ("position", "static"), ("visibility", "visible")]);
        d.set_rect(a_in, 0.0, 0.0, 600.0, 1300.0);
        let b = d.add(Some(grid), "div");
        d.set_styles(b, &[("display", "block"), ("position", "static")]);
        d.set_rect(b, 640.0, 0.0, 640.0, 1400.0);
        let b_in = d.add(Some(b), "p");
        d.set_styles(b_in, &[("display", "block"), ("position", "static"), ("visibility", "visible")]);
        d.set_rect(b_in, 640.0, 0.0, 600.0, 300.0);
        mark_body_descendants(&mut d);
        let f = check_first_viewport_column_overflow_dom(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(
            f[0].finding.detail,
            format!(
                "{} opens the page with one column running 163% of the viewport tall while a sibling fits in 38% — the fold falls deep inside the section",
                class_selector(&d, grid)
            )
        );
        d.set_rect(b_in, 640.0, 0.0, 600.0, 900.0);
        assert!(check_first_viewport_column_overflow_dom(&d).is_empty());
    }

    #[test]
    fn cream_palette_tailwind_fallback() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_attr(body, "class", "bg-amber-50 text-black");
        let f = check_cream_palette(&d);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].snippet, "cream/beige page background (Tailwind bg-amber-50)");
    }
}
