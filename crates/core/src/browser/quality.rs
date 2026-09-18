//! `checkQuality` and its browser adapters from `checks.mjs` Section 5:
//! `checkQuality` (every branch, including the rect-gated ones the static
//! engine never reaches), `checkElementQualityDOM`,
//! `hasVisibleBackgroundBoundary`, `hasMeaningfulDirectText`,
//! `textDescendantsFlushSides`, `isVisuallyHidden`, `isNonRenderedText`,
//! `checkPageQualityFromDoc`, `checkPageQualityDOM`.

#![allow(unused_imports)]
use super::dom::{
    closest_or_none, direct_text, has_direct_text_longer_than, matches_or_false, pf0, safe_id,
    style_px, tag_lower, Dom, ElId, Rect,
};
use super::{BrowserConfig, BrowserFinding};
use crate::checks::measures::{
    chars_per_line, colors_nearly_match, css_color_is_transparent, is_capitalized_run,
    resolve_length_px, text_wraps_to_multiple_lines, TRACKED_LABEL_MAX_CHARS,
};
use crate::checks::rules::RuleHit;
use super::text_geometry::{
    holds_only_phrasing, line_pitch_px, phrasing_holds_break, phrasing_text_extent, phrasing_text_font,
    scrolling_ancestor_cuts, text_line_count,
};
use crate::checks::text_rules::{
    average_glyph_advance_em_at, font_weight_number, is_bold_title_leading, is_cjk_text,
    is_line_clamp, is_under_ui_text_floor, justifies_without_word_spaces_text,
    tracking_is_crushed, ALL_CAPS_LONG_RUN, LEADING_BOLD_TITLE_WEIGHT, SMALLPRINT_TEXT_FLOOR_PX,
    UI_TEXT_FLOOR_PX,
    JUSTIFY_NARROW_CHARS_PER_LINE, LEADING_DISPLAY_TYPE_PX, LEADING_HEADING_CONTEXT,
    LEADING_HEADING_TEXT_TAGS, LEADING_MIN_LINE_BOXES, NON_RENDERED_TAGS, QUALITY_TEXT_TAGS,
    SR_ONLY_SELECTOR, TEXT_EDGE_TAGS,
};
use crate::js::{self, math_round, number_to_string, parse_float, to_fixed};
use crate::js_ext_b::{slice_utf16_prefix, utf16_len};
use once_cell::sync::Lazy;
use regex::Regex;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

re!(WS_RE, format!("{}+", js::WS));
// JS `/url\(/i` in checkQuality's buried-raster branch.
re!(QUALITY_RASTER_URL_RE, format!(r"{}\(", js::ci("url")));
re!(CLIP_RECT_RE, format!(r"rect\({}*0", js::WS));
re!(
    CLIP_INSET_RE,
    format!(r"inset\({}*(?:50%|99|100%)", js::WS)
);
re!(OUTLINE_W_RE, r"([0-9]+(?:\.[0-9]+)?)\s*px");
re!(
    OUTLINE_STYLE_RE,
    r"(?-u:\b)(solid|dashed|dotted|double|groove|ridge|inset|outset)(?-u:\b)"
);
re!(
    OUTLINE_COLOR_RE,
    format!(r"(rgba?\([^)]+\)|#[0-9a-fA-F]{{3,8}}|[a-zA-Z]+){}*$", js::WS)
);

/// JS `s.replace(/\s+/g, ' ')`.
pub fn collapse_ws(s: &str) -> String {
    WS_RE.replace_all(s, " ").into_owned()
}

const FLUSH_SKIP_TAGS: &[&str] = &[
    "HTML", "BODY", "MAIN", "HEADER", "FOOTER", "NAV", "ARTICLE", "ASIDE", "BUTTON", "A", "LABEL",
    "SUMMARY", "CODE", "PRE", "INPUT", "TEXTAREA", "SELECT", "FORM", "FIGURE", "TABLE", "TBODY",
    "THEAD", "TR", "TD", "TH",
];

const TINY_TEXT_UI_CONTEXT: &str = "button, a, label, summary, pre, [role=\"button\"], [role=\"link\"], [role=\"tab\"], [role=\"menuitem\"], [role=\"option\"], nav, footer, [aria-hidden=\"true\"], [class*=\"badge\" i], [class*=\"caption\" i], [class*=\"chip\" i], [class*=\"code\" i], [class*=\"console\" i], [class*=\"diff\" i], [class*=\"label\" i], [class*=\"meta\" i], [class*=\"mock\" i], [class*=\"pill\" i], [class*=\"preview\" i], [class*=\"tag\" i], [class*=\"terminal\" i], [class*=\"writes\" i]";
const EXEMPT_CONTEXT: &str = "pre, code, kbd, samp, var, svg, [aria-hidden=\"true\"], [class*=\"terminal\" i], [class*=\"console\" i], [class*=\"code\" i], [class*=\"mock\" i], [class*=\"editor\" i], [class*=\"syntax\" i], [class*=\"diff\" i]";
const INTERACTIVE: &str = "a[href], button, summary, label, select, textarea, [role=\"button\"], [role=\"link\"], [role=\"tab\"], [role=\"menuitem\"], [role=\"menuitemcheckbox\"], [role=\"menuitemradio\"], [role=\"option\"], [role=\"checkbox\"], [role=\"radio\"], [role=\"switch\"], [role=\"treeitem\"], [tabindex]";
const FURNITURE: &str = "nav, [role=\"navigation\"], td, th, [role=\"gridcell\"], [role=\"cell\"], caption, figcaption, dt, dd, footer, [class*=\"meta\" i], [class*=\"label\" i], [class*=\"badge\" i], [class*=\"chip\" i], [class*=\"pill\" i], [class*=\"tag\" i], [class*=\"kicker\" i], [class*=\"eyebrow\" i], [class*=\"breadcrumb\" i], [class*=\"timestamp\" i], [class*=\"category\" i], [class*=\"caption\" i], [class*=\"nav\" i]";
const SMALLPRINT: &str = "small, footer, [class*=\"legal\" i], [class*=\"copyright\" i], [class*=\"fineprint\" i], [class*=\"fine-print\" i], [class*=\"smallprint\" i], [class*=\"small-print\" i], [class*=\"disclaimer\" i], [class*=\"disclosure\" i], [class*=\"footnote\" i]";
const TEXT_EDGE_QUERY: &str =
    "a, button, code, dd, dt, figcaption, h1, h2, h3, h4, h5, h6, li, p, pre, span, td, th";

/// JS `(el.matches && el.matches(sel)) || (el.closest && el.closest(sel))`.
fn matches_or_closest(dom: &dyn Dom, el: ElId, sel: &str) -> bool {
    matches_or_false(dom, el, sel) || closest_or_none(dom, el, sel).is_some()
}

/// The colour a browser paints behind a page that sets no background of its
/// own, under the light colour scheme: a white box on an unpainted light page
/// draws no edge.
pub const CANVAS_BACKGROUND: &str = "rgb(255, 255, 255)";

/// Whether the canvas under an unpainted chain is the light one
/// [`CANVAS_BACKGROUND`] names. A page that asks for a dark scheme gets a dark
/// canvas from the browser, and any value that mentions `dark` may resolve
/// that way, so only a plainly light scheme lets the comparison run.
pub fn canvas_is_light(scheme: &str) -> bool {
    !js::to_lower_case(scheme).contains("dark")
}

/// JS: checks.mjs#hasVisibleBackgroundBoundary(style, el, win) — browser:
/// `style` is `el`'s own computed style, `win` the live window. The JS
/// answered `true` when no ancestor painted; the canvas under an unpainted
/// light chain is white, so a white box there is compared against it like any
/// other ground.
pub fn has_visible_background_boundary(dom: &dyn Dom, el: ElId) -> bool {
    let bg = dom.style(el, "backgroundColor");
    if css_color_is_transparent(Some(&bg)) {
        return false;
    }
    let mut parent = dom.parent(el);
    while let Some(p) = parent {
        let parent_bg = dom.style(p, "backgroundColor");
        if !css_color_is_transparent(Some(&parent_bg)) {
            return !colors_nearly_match(Some(&bg), Some(&parent_bg));
        }
        parent = dom.parent(p);
    }
    // `colorScheme` is inherited, so the element's own computed value is the
    // page's.
    if !canvas_is_light(&dom.style(el, "colorScheme")) {
        return true;
    }
    !colors_nearly_match(Some(&bg), Some(CANVAS_BACKGROUND))
}

/// The part of `inner` that falls inside `outer`, or `None` when the two miss
/// each other.
///
/// `direct_text_rect` is a font-metric box, not an ink box. A line box tighter
/// than the font's ascent and descent pushes it out of the element's own
/// border box, and so do the tall marks of Devanagari and Thai; no glyph lands
/// out there. Only the part inside that box is what a reader gets, so every
/// edge measurement clamps first.
fn clamp_to(inner: &Rect, outer: &Rect) -> Option<Rect> {
    let left = js::math_max(inner.left, outer.left);
    let top = js::math_max(inner.top, outer.top);
    let w = js::math_min(inner.right, outer.right) - left;
    let h = js::math_min(inner.bottom, outer.bottom) - top;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(Rect::from_xywh(left, top, w, h))
}

/// The space the element's own glyphs keep from each inner edge of its box
/// (`[top, right, bottom, left]`), or `None` when it paints no direct text.
///
/// `direct_text_rect` is the union of the client rects of the element's own
/// text nodes, so this is the room a reader sees rather than the room the
/// stylesheet declares: a fixed-height flex or grid box centres its label
/// with no padding at all, and half-leading adds space of its own.
fn direct_text_insets(dom: &dyn Dom, el: ElId, rect: &Rect, border: &[f64; 4]) -> Option<[f64; 4]> {
    let t = dom.direct_text_rect(el)?;
    if t.width <= 0.0 || t.height <= 0.0 {
        return None;
    }
    // Text that overruns its own box still reads as cramped: the clamped rect
    // lands on the border, an inset of zero.
    let t = clamp_to(&t, rect)?;
    Some([
        t.top - (rect.top + border[0]),
        (rect.right - border[1]) - t.right,
        (rect.bottom - border[2]) - t.bottom,
        t.left - (rect.left + border[3]),
    ])
}

/// Whether text laid out at `tr` survives the clipping between `node` and
/// `el`. A panel held at `max-height: 0` and a drawer collapsed to zero width
/// still lay their text out; none of it reaches the screen, so it cannot be
/// flush against anything.
fn text_rect_survives_clipping(dom: &dyn Dom, el: ElId, node: ElId, tr: &Rect) -> bool {
    let mut cur = dom.parent(node);
    while let Some(p) = cur {
        let clips = |k: &str| {
            let v = dom.style(p, k);
            v == "hidden" || v == "clip" || v == "scroll" || v == "auto"
        };
        if clips("overflow") || clips("overflowX") || clips("overflowY") {
            let cr = dom.rect(p);
            let w = js::math_min(tr.right, cr.right) - js::math_max(tr.left, cr.left);
            let h = js::math_min(tr.bottom, cr.bottom) - js::math_max(tr.top, cr.top);
            if w < 1.0 || h < 1.0 {
                return false;
            }
        }
        if p == el {
            break;
        }
        cur = dom.parent(p);
    }
    true
}

/// JS: checks.mjs#hasMeaningfulDirectText(node)
pub fn has_meaningful_direct_text(dom: &dyn Dom, el: ElId) -> bool {
    has_direct_text_longer_than(dom, el, 4)
}

/// The tallest box `cramped-padding` measures by its glyphs rather than by the
/// content area of its text. Chips run 24 to 28px; the price and step chips
/// the r3-20 evidence rests on measure exactly 28 and 24.
pub const SMALL_CHIP_MAX_HEIGHT_PX: f64 = 28.0;

/// The band of `t`, a text rect of `node`, that its glyphs occupy: each line's
/// em box, one font size tall and centred on the line's content area. A Range
/// rect spans the font's ascent plus descent, which for most faces runs 1.2
/// to 1.4em and leaves room above the capitals and below the descenders; that
/// room, like CSS half-leading, is space a reader sees between the glyphs and
/// the edge. Only the vertical edges move; a rect already no taller than its
/// em boxes is returned as it is.
pub fn glyph_band(dom: &dyn Dom, node: ElId, t: &Rect) -> Rect {
    let font_size = parse_float(&dom.style(node, "fontSize"));
    if !(font_size.is_finite() && font_size > 0.0) {
        return *t;
    }
    let own = resolve_length_px(Some(&dom.style(node, "lineHeight")), font_size)
        .filter(|lh| lh.is_finite() && *lh > 0.0)
        .unwrap_or(font_size * NORMAL_LINE_HEIGHT_EM);
    let pitch = line_pitch_px(dom, node, own);
    let lines = text_line_count(t.height, pitch, font_size);
    let content = t.height - (lines - 1.0) * pitch;
    let inset = (content - font_size) / 2.0;
    if !(inset > 0.0) || inset * 2.0 >= t.height {
        return *t;
    }
    Rect::from_xywh(t.left, t.top + inset, t.width, t.height - inset * 2.0)
}

/// JS: checks.mjs#textDescendantsFlushSides(el, rect) → [top, right, bottom, left]
///
/// Each candidate is measured by its own text rect, not by its border box: a
/// padded button, a centred heading and a table cell all fill the box they sit
/// in while their glyphs stay well inside it, and it is the glyphs a reader
/// sees crowding the boundary.
///
/// In a chip at most [`SMALL_CHIP_MAX_HEIGHT_PX`] tall the text is measured by
/// its glyphs ([`glyph_band`]) rather than by the content area its rect spans:
/// the line box there already holds the glyphs off the edge, and two pixels
/// of declared padding read as enough. Taste call r3-20 (2026-09-18).
pub fn text_descendants_flush_sides(dom: &dyn Dom, el: ElId, rect: &Rect) -> [bool; 4] {
    let mut flush = [false; 4];
    const TEXT_EDGE_THRESHOLD: f64 = 4.0;
    let small_chip = rect.height > 0.0 && rect.height <= SMALL_CHIP_MAX_HEIGHT_PX;
    let candidates = dom.query_all(Some(el), TEXT_EDGE_QUERY).unwrap_or_default();
    for node in candidates {
        let tag_name = dom.tag_name(node);
        if !TEXT_EDGE_TAGS.contains(&tag_name.as_str()) || !has_meaningful_direct_text(dom, node) {
            continue;
        }
        let br = dom.rect(node);
        if br.width <= 0.0 || br.height <= 0.0 {
            continue;
        }
        if br.bottom < rect.top || br.top > rect.bottom || br.right < rect.left || br.left > rect.right {
            continue;
        }
        // Glyphs a reader sees are inside the node's own box, so a box that
        // reaches no edge of `el` has no text that reaches one. Rejecting on
        // the box first keeps the text measurement, a range walk in the page,
        // off the many candidates that sit well inside.
        let box_sides = [
            br.top - rect.top <= TEXT_EDGE_THRESHOLD,
            rect.right - br.right <= TEXT_EDGE_THRESHOLD,
            rect.bottom - br.bottom <= TEXT_EDGE_THRESHOLD,
            br.left - rect.left <= TEXT_EDGE_THRESHOLD,
        ];
        if !box_sides.iter().any(|s| *s) {
            continue;
        }
        // A Dom that cannot measure text falls back to the box, the behaviour
        // this rule had before, rather than going silent. In a small chip the
        // glyphs are measured rather than the font's content area.
        let nr = match dom.direct_text_rect(node) {
            Some(t) if t.width > 0.0 && t.height > 0.0 => match clamp_to(
                &if small_chip { glyph_band(dom, node, &t) } else { t },
                &br,
            ) {
                Some(c) => c,
                None => continue,
            },
            _ => br,
        };
        let sides = [
            nr.top - rect.top <= TEXT_EDGE_THRESHOLD,
            rect.right - nr.right <= TEXT_EDGE_THRESHOLD,
            rect.bottom - nr.bottom <= TEXT_EDGE_THRESHOLD,
            nr.left - rect.left <= TEXT_EDGE_THRESHOLD,
        ];
        // The two remaining tests run only for text that reached an edge.
        if !sides.iter().any(|s| *s) {
            continue;
        }
        if is_visually_hidden(dom, node) || !text_rect_survives_clipping(dom, el, node, &nr) {
            continue;
        }
        for s in 0..4 {
            flush[s] |= sides[s];
        }
    }
    flush
}

/// JS: checks.mjs#isVisuallyHidden(el, style)
pub fn is_visually_hidden(dom: &dyn Dom, el: ElId) -> bool {
    if matches_or_closest(dom, el, SR_ONLY_SELECTOR) {
        return true;
    }
    let pos = dom.style(el, "position");
    if pos == "absolute" || pos == "fixed" {
        let clip = dom.style(el, "clip");
        let clip_path = {
            let a = dom.style(el, "clipPath");
            if !a.is_empty() {
                a
            } else {
                let b = dom.style(el, "webkitClipPath");
                if !b.is_empty() {
                    b
                } else {
                    dom.style(el, "clip-path")
                }
            }
        };
        if CLIP_RECT_RE.is_match(&clip) || CLIP_INSET_RE.is_match(&clip_path) {
            return true;
        }
        let w = parse_float(&dom.style(el, "width"));
        let h = parse_float(&dom.style(el, "height"));
        let overflow = dom.style(el, "overflow");
        if (w == 1.0 || h == 1.0) && (overflow == "hidden" || overflow == "clip") {
            return true;
        }
    }
    false
}

/// Whether this element carries heading text, for the tight-leading floor:
/// the element is a heading (or takes the ARIA role), one of the inline tags
/// a heading's text sits in, or any other box under a heading (the `div` a
/// design system wraps heading copy in). A reading block nested inside a
/// heading (a `p`, an `li`, and whatever sits inside one) is body copy and
/// keeps the floor.
pub fn is_heading_text(dom: &dyn Dom, el: ElId, tag: &str) -> bool {
    if matches_or_false(dom, el, LEADING_HEADING_CONTEXT) {
        return true;
    }
    let Some(heading) = closest_or_none(dom, el, LEADING_HEADING_CONTEXT) else {
        return false;
    };
    if LEADING_HEADING_TEXT_TAGS.contains(&tag) {
        return true;
    }
    let mut cur = Some(el);
    while let Some(c) = cur {
        if c == heading {
            break;
        }
        if QUALITY_TEXT_TAGS.contains(&tag_lower(dom, c).as_str()) {
            return false;
        }
        cur = dom.parent(c);
    }
    true
}

/// How far up from a text element the tight-leading floor looks for the box
/// that clamps or clips its lines: the element, the inline run it may sit in,
/// and the title wrapper around that (ynet.co.il sets its headline in a `div`
/// inside a link inside the clamped `div.slotTitle`).
const LINE_CLAMP_SEARCH_DEPTH: usize = 4;

/// The lines of `t`, the text rect of `el`, that render: whether a line clamp
/// holds them, and where the nearest box that clips them on the y axis cuts
/// them off.
///
/// A box between `el` and that clipping box with a `-webkit-box` line clamp
/// ([`is_line_clamp`], read from its `display` and `webkitLineClamp`) holds
/// the text in a clamp. A box that clips without a clamp (a `max-height` or
/// a fixed height with `overflow: hidden`) is no clamp: it only cuts the
/// count at its content box, so bold text it shows three lines of still
/// reports.
fn rendered_lines(dom: &dyn Dom, el: ElId, t: &Rect) -> (bool, f64) {
    let mut cur = Some(el);
    let mut depth = 0;
    while let Some(c) = cur {
        if depth >= LINE_CLAMP_SEARCH_DEPTH {
            break;
        }
        if is_line_clamp(&dom.style(c, "display"), &dom.style(c, "webkitLineClamp")) {
            return (true, t.bottom);
        }
        let clips = |k: &str| matches!(dom.style(c, k).as_str(), "hidden" | "clip");
        if clips("overflowY") || clips("overflow") {
            let r = dom.rect(c);
            let content_bottom =
                r.bottom - style_px(dom, c, "borderBottomWidth") - style_px(dom, c, "paddingBottom");
            if !(content_bottom.is_finite() && t.all_finite()) || content_bottom >= t.bottom {
                return (false, t.bottom);
            }
            return (false, js::math_max(content_bottom, t.top));
        }
        cur = dom.parent(c);
        depth += 1;
    }
    (false, t.bottom)
}

/// The tags whose prose is measured for `line-length` when its words sit
/// wholly in inline children (`<p><i>…</i></p>`).
const LINE_PROSE_TAGS: &[&str] = &["p", "li", "dd", "blockquote"];

/// The line-height `normal` stands for when counting line boxes: a text rect
/// one line tall is at most about 1.5em, two lines at least about 2.3em.
const NORMAL_LINE_HEIGHT_EM: f64 = 1.2;

/// How much of its content box a block's widest line fills before
/// `body-text-viewport-edge` takes the box's edges as the text's. A wrapped
/// paragraph's ragged right is under a word short of its column, and the box
/// is what an author sets.
const TEXT_FILLS_MEASURE: f64 = 0.9;

/// How much of its box a block's widest line fills before `line-length`
/// takes the box as the measure. The widest line's count is estimated at half
/// an em a glyph, which runs 10 to 20% over a narrow sans, so a line within
/// that of its box may hold as many characters as the box estimate says.
const LINE_FILLS_MEASURE: f64 = 0.8;

/// Characters on a line `width_px` wide at `font_size_px`, with glyphs
/// `advance_em` wide on average.
fn chars_per_line_at(width_px: f64, font_size_px: f64, advance_em: f64) -> f64 {
    width_px / (font_size_px * advance_em)
}

/// The height of one line box of an element's own box. An inline box that
/// wraps reports the union of its fragments, two 21px highlight lines as one
/// 43px box, while each fragment a reader sees is one line tall. Blocks, and
/// an inline box whose lines cannot be counted, keep their box height.
fn own_line_box_height(
    dom: &dyn Dom,
    el: ElId,
    rect: &Rect,
    own_line_height: Option<f64>,
    font_size: f64,
) -> f64 {
    if dom.style(el, "display") != "inline" {
        return rect.height;
    }
    let (Some(own), Some(t)) = (own_line_height, dom.direct_text_rect(el)) else {
        return rect.height;
    };
    if !(own > 0.0) || !t.all_finite() || t.height <= 0.0 {
        return rect.height;
    }
    let lines = text_line_count(t.height, line_pitch_px(dom, el, own), font_size);
    rect.height / lines
}

/// JS: checks.mjs#isNonRenderedText(el, tag, style)
pub fn is_non_rendered_text(dom: &dyn Dom, el: ElId, tag: &str) -> bool {
    let t = js::to_lower_case(tag);
    if NON_RENDERED_TAGS.contains(&t.as_str()) {
        return true;
    }
    if closest_or_none(dom, el, "head").is_some() {
        return true;
    }
    if dom.style(el, "display") == "none" {
        return true;
    }
    let vis = dom.style(el, "visibility");
    if vis == "hidden" || vis == "collapse" {
        return true;
    }
    false
}

/// Inputs of `checkQuality` as the browser adapter builds them.
pub struct QualityInput {
    pub el: ElId,
    pub tag: String,
    pub has_direct_text: bool,
    pub text_len: usize,
    pub font_size: f64,
    pub line_height_px: Option<f64>,
    pub letter_spacing_px: Option<f64>,
    pub rect: Rect,
    pub line_max: f64,
    pub viewport_width: f64,
}

/// The largest box, on either axis, that reads as an icon rather than a
/// picture.
const RASTER_ICON_MAX_PX: f64 = 48.0;

/// The `blur()` radius past which a faint raster is a blur-up placeholder.
const RASTER_PLACEHOLDER_MIN_BLUR_PX: f64 = 4.0;

/// Whether a near-transparent raster is one state of a layer rather than
/// buried material: vector art, an icon-sized box, a blurred low-resolution
/// placeholder, or a frame stacked under a painted raster in the same box (a
/// crossfade whose visible frame is a sibling, a placeholder under a parent
/// that paints the loaded picture).
fn raster_is_state_layer(dom: &dyn Dom, el: ElId, tag: &str, bg: &str, rect: &Rect) -> bool {
    if crate::checks::measures::raster_source_is_svg(tag == "img", dom.attr(el, "src").as_deref(), bg) {
        return true;
    }
    if rect.width > 0.0
        && rect.height > 0.0
        && rect.width <= RASTER_ICON_MAX_PX
        && rect.height <= RASTER_ICON_MAX_PX
    {
        return true;
    }
    if filter_blur_px(&dom.style(el, "filter")) >= RASTER_PLACEHOLDER_MIN_BLUR_PX {
        return true;
    }
    let area = rect.width * rect.height;
    if !(area > 0.0) {
        return false;
    }
    let covers = |other: &Rect| {
        let w = (rect.right.min(other.right) - rect.left.max(other.left)).max(0.0);
        let h = (rect.bottom.min(other.bottom) - rect.top.max(other.top)).max(0.0);
        w * h >= area * 0.5
    };
    let paints_raster = |node: ElId| {
        let own = parse_float(&dom.style(node, "opacity"));
        let visible = !own.is_finite() || own >= 0.15;
        let t = tag_lower(dom, node);
        let raster = matches!(t.as_str(), "img" | "picture" | "video" | "canvas")
            || QUALITY_RASTER_URL_RE.is_match(&dom.style(node, "backgroundImage"));
        visible && raster && dom.style(node, "display") != "none" && covers(&dom.rect(node))
    };
    let Some(parent) = dom.parent(el) else {
        return false;
    };
    if dom
        .children(parent)
        .into_iter()
        .any(|sibling| sibling != el && paints_raster(sibling))
    {
        return true;
    }
    let mut ancestor = Some(parent);
    for _ in 0..2 {
        let Some(node) = ancestor else {
            break;
        };
        if Some(node) == dom.body() || Some(node) == dom.document_element() {
            break;
        }
        if paints_raster(node) {
            return true;
        }
        ancestor = dom.parent(node);
    }
    false
}

/// The largest `blur()` radius in a computed `filter`, 0 when there is none.
fn filter_blur_px(filter: &str) -> f64 {
    let mut max = 0.0f64;
    let lower = filter.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(start) = rest.find("blur(") {
        let after = &rest[start + 5..];
        let end = after.find(')').unwrap_or(after.len());
        let v = parse_float(js::trim(&after[..end]));
        if v.is_finite() {
            max = max.max(v);
        }
        rest = &after[end..];
    }
    max
}

/// JS: checks.mjs#checkQuality(opts), browser adapter inputs (`rect` set,
/// `win` = window).
pub fn check_quality(dom: &dyn Dom, q: &QualityInput) -> Vec<RuleHit> {
    let el = q.el;
    let tag = q.tag.as_str();
    let font_size = q.font_size;
    let text_len = q.text_len;
    let rect = &q.rect;
    let line_max = q.line_max;
    let viewport_width = q.viewport_width;
    let has_direct_text = q.has_direct_text;
    let mut findings: Vec<RuleHit> = Vec::new();

    let el_id = safe_id(dom, el);
    if el_id.starts_with("claude-") || el_id.starts_with("cic-") {
        return findings;
    }

    let st = |k: &str| dom.style(el, k);
    let spx = |k: &str| style_px(dom, el, k);

    // A raster (<img>, or an element with a background url) at near-zero
    // opacity never reaches the screen: the produced material ships as a
    // compliance token. The CSS-text scan catches the stylesheet form; this
    // catches computed opacity on the element itself (both engines).
    {
        let op = parse_float(&st("opacity"));
        if op.is_finite() && op < 0.15 && op >= 0.0 {
            let bg = st("backgroundImage");
            if (tag == "img" || QUALITY_RASTER_URL_RE.is_match(&bg))
                && !raster_is_state_layer(dom, el, tag, &bg, rect)
            {
                let label = if tag == "img" {
                    dom.attr(el, "alt").unwrap_or_default()
                } else {
                    slice_utf16_prefix(js::trim(&dom.text_content(el)), 40)
                };
                findings.push(RuleHit::new(
                    "buried-raster",
                    format!(
                        "{} at opacity {}{}",
                        if tag == "img" { "<img>" } else { "raster background" },
                        number_to_string(op),
                        if label.is_empty() {
                            String::new()
                        } else {
                            format!(" \"{label}\"")
                        }
                    ),
                ));
            }
        }
    }

    // --- Line length too long ---
    // Measured on the text where it can be. Text that renders as one line
    // never sends the eye back across a column, however wide its box; a
    // centred line, or one ended by a `<br>`, is as long as its glyphs rather
    // than its box; and a line holds no more characters than the block does.
    // Full-width CJK glyphs take an em each. Prose whose words sit wholly in
    // inline children is measured on the block that sets its lines. Where the
    // text cannot be measured the box stands in, as before.
    let prose_in_phrasing =
        !has_direct_text && LINE_PROSE_TAGS.contains(&tag) && holds_only_phrasing(dom, el);
    if (has_direct_text || prose_in_phrasing)
        && QUALITY_TEXT_TAGS.contains(&tag)
        && rect.width > 0.0
        && (text_len as f64) > line_max
    {
        let text = collapse_ws(js::trim(&dom.text_content(el)));
        // Glyphs are as wide as the font of the runs that set them: a 16px
        // paragraph whose words sit in a 24px span holds a third fewer
        // characters a line, and a monospace face advances 0.6em a glyph.
        let (text_size, latin_advance) = phrasing_text_font(dom, el, font_size);
        let advance = average_glyph_advance_em_at(&text, latin_advance);
        let estimate = match phrasing_text_extent(dom, el) {
            Some(t) => {
                let pitch = q
                    .line_height_px
                    .filter(|lh| *lh > 0.0)
                    .unwrap_or(font_size * NORMAL_LINE_HEIGHT_EM);
                let lines = text_line_count(t.height, pitch, text_size);
                if lines >= 2.0 {
                    let chars = utf16_len(&text) as f64;
                    // The half-em advance runs 10 to 20% wide of a real face,
                    // so a line within that of its box cannot be told from
                    // one that fills it in a narrow sans (veeza.ai's lines
                    // at 88%), and the box estimate stands. Lines broken by
                    // a `<br>` are short where the author ended them, and
                    // keep the widest line up to the 90% a wrapped column
                    // fills.
                    let fill = if phrasing_holds_break(dom, el) {
                        TEXT_FILLS_MEASURE
                    } else {
                        LINE_FILLS_MEASURE
                    };
                    let estimate = if t.width >= rect.width * fill {
                        chars_per_line_at(rect.width, text_size, advance)
                    } else {
                        // The widest line from its glyphs. However narrow the
                        // face, some line holds at least the average count
                        // when all of the block's text sits in these lines.
                        let from_width = chars_per_line_at(t.width, text_size, advance);
                        if holds_only_phrasing(dom, el) {
                            js::math_max(from_width, chars / lines)
                        } else {
                            from_width
                        }
                    };
                    Some(js::math_min(estimate, chars))
                } else {
                    None
                }
            }
            None if has_direct_text => Some(chars_per_line_at(rect.width, text_size, advance)),
            None => None,
        };
        if let Some(cpl) = estimate.filter(|cpl| *cpl > line_max + 5.0) {
            findings.push(RuleHit::new(
                "line-length",
                format!(
                    "~{} chars/line (aim for <{})",
                    number_to_string(math_round(cpl)),
                    number_to_string(line_max)
                ),
            ));
        }
    }

    // --- Cramped padding ---
    let is_inline_code = tag == "code" && closest_or_none(dom, el, "pre").is_none();
    if !is_inline_code
        && has_direct_text
        && text_len > 20
        && rect.width > 100.0
        && own_line_box_height(dom, el, rect, q.line_height_px, font_size) > 30.0
    {
        let borders = [
            spx("borderTopWidth"),
            spx("borderRightWidth"),
            spx("borderBottomWidth"),
            spx("borderLeftWidth"),
        ];
        // A `border: 1px solid transparent` focus-ring placeholder draws no
        // edge, so it bounds nothing.
        let border_visible = [
            borders[0] > 0.0 && !css_color_is_transparent(Some(&st("borderTopColor"))),
            borders[1] > 0.0 && !css_color_is_transparent(Some(&st("borderRightColor"))),
            borders[2] > 0.0 && !css_color_is_transparent(Some(&st("borderBottomColor"))),
            borders[3] > 0.0 && !css_color_is_transparent(Some(&st("borderLeftColor"))),
        ];
        let border_count = border_visible.iter().filter(|v| **v).count();
        let has_bg = has_visible_background_boundary(dom, el);
        if border_count >= 2 || has_bg {
            let mut v_sides: Vec<usize> = Vec::new();
            let mut h_sides: Vec<usize> = Vec::new();
            if has_bg || border_visible[0] {
                v_sides.push(0);
            }
            if has_bg || border_visible[2] {
                v_sides.push(2);
            }
            if has_bg || border_visible[3] {
                h_sides.push(3);
            }
            if has_bg || border_visible[1] {
                h_sides.push(1);
            }
            let pad_names = ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"];
            let min_over = |sides: &[usize], f: &dyn Fn(usize) -> f64| {
                sides.iter().map(|&s| f(s)).fold(f64::INFINITY, js::math_min)
            };
            let pad_of = |s: usize| spx(pad_names[s]);
            let v_min = min_over(&v_sides, &pad_of);
            let h_min = min_over(&h_sides, &pad_of);
            let v_thresh = js::math_max(4.0, font_size * 0.3);
            let h_thresh = js::math_max(8.0, font_size * 0.5);
            // Declared padding is what a fix edits, but it is not what the
            // reader sees. Where the element's own text is measurable, the
            // gap its glyphs keep from the bounded edges decides: a 40px
            // flex row centres a 14px label on zero padding, and that label
            // has 12px of air on both sides.
            let insets = direct_text_insets(dom, el, rect, &borders);
            let inset_of = |s: usize| insets.map_or(f64::NEG_INFINITY, |i| i[s]);
            let v_cramped = v_min < v_thresh && min_over(&v_sides, &inset_of) < v_thresh;
            let h_cramped = h_min < h_thresh && min_over(&h_sides, &inset_of) < h_thresh;
            if v_cramped {
                findings.push(RuleHit::new(
                    "cramped-padding",
                    format!(
                        "{}px vertical padding (need ≥{}px for {}px text)",
                        number_to_string(v_min),
                        to_fixed(v_thresh, 1),
                        number_to_string(font_size)
                    ),
                ));
            } else if h_cramped {
                findings.push(RuleHit::new(
                    "cramped-padding",
                    format!(
                        "{}px horizontal padding (need ≥{}px for {}px text)",
                        number_to_string(h_min),
                        to_fixed(h_thresh, 1),
                        number_to_string(font_size)
                    ),
                ));
            }
        }
    }

    // --- Flush against a visible boundary ---
    {
        let upper_tag = js::to_upper_case(tag);
        let el_position = st("position");
        let children = dom.children(el);
        // A box with no area paints no boundary (a drawer collapsed to zero
        // width), and an inline box's border-bottom is an underline: text
        // sitting on it is the point of it, and padding would not move it.
        let el_is_box = rect.width > 0.0 && rect.height > 0.0 && st("display") != "inline";
        if !FLUSH_SKIP_TAGS.contains(&upper_tag.as_str())
            && !has_direct_text
            && el_position != "fixed"
            && el_position != "absolute"
            && el_is_box
            && !children.is_empty()
        {
            let border_w = [
                spx("borderTopWidth"),
                spx("borderRightWidth"),
                spx("borderBottomWidth"),
                spx("borderLeftWidth"),
            ];
            let bc = |k: &str| css_color_is_transparent(Some(&st(k)));
            let border_visible = [
                border_w[0] > 0.0 && !bc("borderTopColor"),
                border_w[1] > 0.0 && !bc("borderRightColor"),
                border_w[2] > 0.0 && !bc("borderBottomColor"),
                border_w[3] > 0.0 && !bc("borderLeftColor"),
            ];
            let mut outline_w = spx("outlineWidth");
            let mut outline_style_val = st("outlineStyle");
            let mut outline_color_val = st("outlineColor");
            let outline_short = st("outline");
            if outline_w == 0.0 && !outline_short.is_empty() {
                if let Some(m) = OUTLINE_W_RE.captures(&outline_short) {
                    outline_w = pf0(m.get(1).map(|x| x.as_str()).unwrap_or(""));
                }
                if outline_style_val.is_empty() {
                    outline_style_val = if OUTLINE_STYLE_RE.is_match(&outline_short) {
                        "solid".to_string()
                    } else {
                        String::new()
                    };
                }
                if outline_color_val.is_empty() {
                    if let Some(m) = OUTLINE_COLOR_RE.captures(&outline_short) {
                        outline_color_val = m.get(1).map(|x| x.as_str()).unwrap_or("").to_string();
                    }
                }
            }
            let outline_visible = outline_w > 0.0
                && !css_color_is_transparent(Some(&outline_color_val))
                && !outline_style_val.is_empty()
                && outline_style_val != "none";
            let bg_visible = has_visible_background_boundary(dom, el);
            let any_visible = border_visible.iter().any(|b| *b) || outline_visible || bg_visible;
            if any_visible {
                let len = |e: ElId, k: &str| {
                    resolve_length_px(Some(&dom.style(e, k)), font_size).unwrap_or(0.0)
                };
                let pad = [
                    len(el, "paddingTop"),
                    len(el, "paddingRight"),
                    len(el, "paddingBottom"),
                    len(el, "paddingLeft"),
                ];
                const PAD_THRESHOLD: f64 = 2.0;
                const CHILD_INSULATE_THRESHOLD: f64 = 4.0;
                let mut children_insulate = [false; 4];
                for &child in &children {
                    let child_pad = [
                        len(child, "paddingTop"),
                        len(child, "paddingRight"),
                        len(child, "paddingBottom"),
                        len(child, "paddingLeft"),
                    ];
                    let child_margin = [
                        len(child, "marginTop"),
                        len(child, "marginRight"),
                        len(child, "marginBottom"),
                        len(child, "marginLeft"),
                    ];
                    let cr = dom.rect(child);
                    if cr.width > 0.0 && cr.height > 0.0 {
                        if cr.top - rect.top >= CHILD_INSULATE_THRESHOLD {
                            children_insulate[0] = true;
                        }
                        if rect.right - cr.right >= CHILD_INSULATE_THRESHOLD {
                            children_insulate[1] = true;
                        }
                        if rect.bottom - cr.bottom >= CHILD_INSULATE_THRESHOLD {
                            children_insulate[2] = true;
                        }
                        if cr.left - rect.left >= CHILD_INSULATE_THRESHOLD {
                            children_insulate[3] = true;
                        }
                    }
                    for s in 0..4 {
                        if child_pad[s] >= CHILD_INSULATE_THRESHOLD
                            || child_margin[s] >= CHILD_INSULATE_THRESHOLD
                        {
                            children_insulate[s] = true;
                        }
                    }
                }

                let text_flush = text_descendants_flush_sides(dom, el, rect);
                let full_bleed_bg_band = viewport_width > 0.0
                    && rect.width >= viewport_width * 0.94
                    && bg_visible
                    && !outline_visible;
                let side_names = ["top", "right", "bottom", "left"];
                let mut flush_sides: Vec<&str> = Vec::new();
                for s in 0..4 {
                    let bg_bounds_side = bg_visible && !(full_bleed_bg_band && (s == 1 || s == 3));
                    let side_bounded = border_visible[s] || outline_visible || bg_bounds_side;
                    if side_bounded && pad[s] <= PAD_THRESHOLD && !children_insulate[s] && text_flush[s] {
                        flush_sides.push(side_names[s]);
                    }
                }

                if !flush_sides.is_empty() {
                    let mut has_text_child = false;
                    for &child in &children {
                        let child_text = js::trim(&dom.text_content(child)).to_string();
                        if utf16_len(&child_text) > 4 {
                            has_text_child = true;
                            break;
                        }
                    }
                    if has_text_child {
                        let cls_all = dom.class_name_prop(el).unwrap_or_default();
                        let cls_all = js::trim(&cls_all).to_string();
                        let cls = if cls_all.is_empty() {
                            String::new()
                        } else {
                            WS_RE.split(&cls_all).next().unwrap_or("").to_string()
                        };
                        let mut boundary_parts: Vec<String> = Vec::new();
                        let border_sides_visible: Vec<&str> = (0..4)
                            .filter(|i| border_visible[*i])
                            .map(|i| side_names[i])
                            .collect();
                        if border_sides_visible.len() == 4 {
                            boundary_parts.push("border".to_string());
                        } else if !border_sides_visible.is_empty() {
                            boundary_parts.push(format!("border-{}", border_sides_visible.join("/")));
                        }
                        if outline_visible {
                            boundary_parts.push("outline".to_string());
                        }
                        if bg_visible {
                            boundary_parts.push("bg".to_string());
                        }
                        let sides_label = if flush_sides.len() == 4 {
                            "all sides".to_string()
                        } else {
                            flush_sides.join("/")
                        };
                        let tl = js::to_lower_case(tag);
                        let ident = if !cls.is_empty() {
                            format!("<{}> \"{}\"", tl, cls)
                        } else {
                            format!("<{}>", tl)
                        };
                        findings.push(RuleHit::new(
                            "cramped-padding",
                            format!(
                                "{}: children flush against {} on {} (no inset)",
                                ident,
                                boundary_parts.join("+"),
                                sides_label
                            ),
                        ));
                    }
                }
            }
        }
    }

    // --- Body text touching viewport edge ---
    // Measured on the text where it can be: a centred or padded paragraph
    // spans the viewport with its box while its glyphs keep a gutter, and a
    // paragraph a horizontal scroller cuts (a slide in a swiped track) meets
    // that track's clip rather than the page edge. A box that only hides its
    // overflow proves no track, so text it cuts at the screen edge reports.
    // Prose whose words sit wholly in inline children is measured the same
    // way. Where the text cannot be measured the box stands in, as before.
    let is_edge_tag = matches!(js::to_upper_case(tag).as_str(), "P" | "LI");
    let edge_prose = !has_direct_text && is_edge_tag && holds_only_phrasing(dom, el);
    if (has_direct_text || edge_prose) && text_len > 40 && is_edge_tag && viewport_width > 0.0 {
        let in_nav_header =
            closest_or_none(dom, el, "nav").is_some() || closest_or_none(dom, el, "header").is_some();
        let bg = st("backgroundColor");
        let has_own_bg = !bg.is_empty() && bg != "rgba(0, 0, 0, 0)" && bg != "transparent";
        let pos = st("position");
        let is_positioned = pos == "fixed" || pos == "absolute";
        let width_ratio = rect.width / viewport_width;
        let span = if in_nav_header || has_own_bg || is_positioned || !(width_ratio > 0.5) {
            None
        } else {
            match phrasing_text_extent(dom, el) {
                Some(t) if scrolling_ancestor_cuts(dom, el, &t) => None,
                Some(t) => {
                    let content_left = rect.left + spx("borderLeftWidth") + spx("paddingLeft");
                    let content_right = rect.right - spx("borderRightWidth") - spx("paddingRight");
                    let pitch = q
                        .line_height_px
                        .filter(|lh| *lh > 0.0)
                        .unwrap_or(font_size * NORMAL_LINE_HEIGHT_EM);
                    if text_line_count(t.height, pitch, font_size) >= 2.0
                        && t.width >= (content_right - content_left) * TEXT_FILLS_MEASURE
                    {
                        // Wrapped lines that fill the column reach its edges;
                        // how ragged the longest line happens to be is not
                        // the gutter.
                        Some((content_left, content_right))
                    } else if st("display") == "list-item" {
                        // A list item's marker is painted, not a text node:
                        // an `inside` bullet sits at the content edge ahead of
                        // the text, so the start side reaches that edge.
                        if st("direction") == "rtl" {
                            Some((t.left, js::math_max(t.right, content_right)))
                        } else {
                            Some((js::math_min(t.left, content_left), t.right))
                        }
                    } else {
                        Some((t.left, t.right))
                    }
                }
                None if has_direct_text => Some((rect.left, rect.right)),
                None => None,
            }
        };
        let (left, right) = span.unwrap_or((f64::NAN, f64::NAN));
        // Text wholly past either side of the viewport meets no edge a reader
        // sees: a desktop column laid out past a phone viewport, a list
        // parked 800px to the right.
        let in_viewport = right > 0.0 && left < viewport_width;
        let left_close = in_viewport && left < 16.0;
        let right_close = in_viewport && right > viewport_width - 16.0;
        if left_close || right_close {
            let l = number_to_string(math_round(left));
            let r = number_to_string(math_round(viewport_width - right));
            let which = if left_close && right_close {
                format!("left {}px / right {}px", l, r)
            } else if left_close {
                format!("left {}px", l)
            } else {
                format!("right {}px", r)
            };
            findings.push(RuleHit::new(
                "body-text-viewport-edge",
                format!(
                    "<{}> with {}-char body bleeds to viewport edge ({})",
                    js::to_lower_case(tag),
                    text_len,
                    which
                ),
            ));
        }
    }

    let is_heading = matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6");

    // --- Tight line height ---
    // The 1.3 floor is a reading-comfort floor for body copy: text a visitor
    // reads at body scale, over more than one line. Several things are not
    // that.
    // Display type sets its own leading, and 1.2 at 32px is craft, not
    // crowding; headings routinely put their text in a child <a> or <span>,
    // so the exemption reads the nearest heading ancestor rather than the
    // element's own tag. Text that renders as a single line box has no gap
    // between lines to crowd. And source text that is never typeset (script,
    // style, noscript, head content, display:none, the sr-only clip patterns,
    // an element with no box at all) has no leading to measure.
    if has_direct_text
        && text_len > 50
        && !is_heading
        && font_size > 0.0
        && font_size < LEADING_DISPLAY_TYPE_PX
    {
        if let Some(own_lh) = q.line_height_px {
            // An inline run's lines are set on the block around it, whose
            // strut is the pitch when it is taller than the run's own value.
            let lh = line_pitch_px(dom, el, own_lh);
            let ratio = lh / font_size;
            // Compare on the ratio the snippet prints, so a page that sets
            // line-height: 1.3 exactly is never flagged for hitting the floor
            // (46.8 / 36 is 1.2999999999999998 in binary floats).
            let shown = js::math_round(ratio * 100.0) / 100.0;
            if ratio > 0.0 && shown < 1.3 {
                let text_rect = dom.direct_text_rect(el).unwrap_or(*rect);
                let wraps = text_rect.height >= lh * LEADING_MIN_LINE_BOXES;
                // A bold run of two rendered lines or fewer, or one in a line
                // clamp, is a title set on a div or span: it gets the heading
                // exemption. Lines a clipping box cuts off do not render.
                let bold_title = || {
                    let weight = font_weight_number(&st("fontWeight"));
                    if weight < LEADING_BOLD_TITLE_WEIGHT {
                        return false;
                    }
                    let (clamped, bottom) = rendered_lines(dom, el, &text_rect);
                    let lines = text_line_count(bottom - text_rect.top, lh, font_size);
                    is_bold_title_leading(weight, Some(lines), clamped)
                };
                if wraps
                    && !is_non_rendered_text(dom, el, tag)
                    && !is_visually_hidden(dom, el)
                    && !is_heading_text(dom, el, tag)
                    && !bold_title()
                {
                    findings.push(RuleHit::new(
                        "tight-leading",
                        format!("line-height {}x (need >=1.3)", to_fixed(ratio, 2)),
                    ));
                }
            }
        }
    }

    // --- Justified text (without hyphens) ---
    // Only a narrow column stretches word spaces far enough to open rivers,
    // and only in a script that justifies on word spaces at all.
    if has_direct_text && st("textAlign") == "justify" && rect.width > 0.0 && font_size > 0.0 {
        let hyphens = {
            let a = st("hyphens");
            if !a.is_empty() {
                a
            } else {
                st("webkitHyphens")
            }
        };
        if hyphens != "auto"
            && chars_per_line(rect.width, font_size) <= JUSTIFY_NARROW_CHARS_PER_LINE
            && !justifies_without_word_spaces_text(&direct_text(dom, el))
        {
            findings.push(RuleHit::new(
                "justified-text",
                "text-align: justify without hyphens: auto".to_string(),
            ));
        }
    }

    // --- Tiny body text ---
    if has_direct_text && text_len > 20 && font_size < 12.0 {
        let skip_tags = ["sub", "sup", "code", "kbd", "samp", "var", "caption", "figcaption"];
        let in_ui_context = closest_or_none(dom, el, TINY_TEXT_UI_CONTEXT).is_some();
        let is_uppercase = st("textTransform") == "uppercase";
        if !skip_tags.contains(&tag)
            && !in_ui_context
            && !is_uppercase
            && !is_non_rendered_text(dom, el, tag)
        {
            findings.push(RuleHit::new(
                "tiny-text",
                format!("{}px body text", number_to_string(font_size)),
            ));
        }
    }

    // --- Undersized functional / UI text ---
    {
        let dt = js::trim(&collapse_ws(&direct_text(dom, el))).to_string();
        let dt_len = utf16_len(&dt);
        let ui_skip_tags = ["sub", "sup", "option"];
        if font_size > 0.0
            && font_size < UI_TEXT_FLOOR_PX
            && dt_len >= 2
            && !ui_skip_tags.contains(&tag)
            // A footnote marker is set small by convention, and so is the
            // link inside it (`<sup><a>[7]</a></sup>`).
            && closest_or_none(dom, el, "sub, sup").is_none()
            && !is_non_rendered_text(dom, el, tag)
        {
            let is_exempt_context = matches_or_closest(dom, el, EXEMPT_CONTEXT);
            if !is_exempt_context && !is_visually_hidden(dom, el) {
                let is_interactive = matches_or_closest(dom, el, INTERACTIVE);
                let is_furniture = matches_or_closest(dom, el, FURNITURE);
                let is_smallprint = matches_or_closest(dom, el, SMALLPRINT);
                let floor = if !is_interactive && is_smallprint {
                    SMALLPRINT_TEXT_FLOOR_PX
                } else {
                    UI_TEXT_FLOOR_PX
                };
                // Fluid type a hair under the floor (10.9688px against 11px)
                // is not smaller text to a reader: the floor keeps a 0.1px
                // tolerance under each value.
                if is_under_ui_text_floor(font_size, floor)
                    && (is_interactive || is_furniture || dt_len <= 20)
                {
                    let excerpt = slice_utf16_prefix(&dt, 40);
                    findings.push(RuleHit::new(
                        "undersized-ui-text",
                        format!(
                            "{}px functional text \"{}\" (below {}px floor)",
                            number_to_string(font_size),
                            excerpt,
                            number_to_string(floor)
                        ),
                    ));
                }
            }
        }
    }

    // --- All-caps body text ---
    // Uppercase on a short run is a convention, not a defect: a button, a nav
    // item, a kicker or an eyebrow is taken in as a shape, so losing word
    // shapes costs nothing. The cost lands when the run is long enough to be
    // read as a sentence. The run is the element's own text: a bar or a form
    // control whose children hold the labels is not one long run, however its
    // subtree adds up.
    if has_direct_text && st("textTransform") == "uppercase" && !is_heading {
        let own_len = utf16_len(js::trim(&collapse_ws(&direct_text(dom, el))));
        if own_len >= ALL_CAPS_LONG_RUN {
            findings.push(RuleHit::new(
                "all-caps-body",
                format!("text-transform: uppercase on {} chars of body text", own_len),
            ));
        }
    }

    // --- Wide letter spacing on body text ---
    if has_direct_text && text_len > 20 {
        if let Some(ls) = q.letter_spacing_px {
            if ls > 0.0 && font_size > 0.0 {
                let tracking_em = ls / font_size;
                if tracking_em > 0.05 {
                    // Wide tracking is the standard treatment for an
                    // uppercase eyebrow, label or button. `text-transform`
                    // says so outright; capitals typed into the markup do
                    // not, so that reading is held to label size on one
                    // line and running text keeps the rule.
                    let caps_label = st("textTransform") == "uppercase"
                        || (text_len <= TRACKED_LABEL_MAX_CHARS
                            && is_capitalized_run(js::trim(&dom.text_content(el)))
                            && !text_wraps_to_multiple_lines(
                                dom.direct_text_rect(el).map(|r| r.height).unwrap_or(0.0),
                                q.line_height_px,
                            ));
                    if !caps_label {
                        findings.push(RuleHit::new(
                            "wide-tracking",
                            format!("letter-spacing: {}em on body text", to_fixed(tracking_em, 2)),
                        ));
                    }
                }
            }
        }
    }

    // --- Crushed letter spacing ---
    if has_direct_text && text_len > 20 && font_size > 0.0 {
        if let Some(ls) = q.letter_spacing_px {
            if ls < 0.0 {
                let tracking_em = ls / font_size;
                if tracking_is_crushed(tracking_em, font_size) {
                    let text = collapse_ws(js::trim(&dom.text_content(el)));
                    if !is_cjk_text(&text) {
                        findings.push(RuleHit::new(
                            "extreme-negative-tracking",
                            format!(
                                "letter-spacing: {}em at {}px — \"{}\"",
                                to_fixed(tracking_em, 2),
                                number_to_string(font_size),
                                slice_utf16_prefix(&text, 40)
                            ),
                        ));
                    }
                }
            }
        }
    }

    findings
}

/// JS: checks.mjs#checkElementQualityDOM(el)
pub fn check_element_quality_dom(dom: &dyn Dom, el: ElId, config: &BrowserConfig) -> Vec<RuleHit> {
    let tag = tag_lower(dom, el);
    let has_direct_text = has_direct_text_longer_than(dom, el, 10);
    let text_len = utf16_len(js::trim(&dom.text_content(el)));
    let font_size = {
        let n = parse_float(&dom.style(el, "fontSize"));
        if crate::js_ext_a::num_truthy(n) {
            n
        } else {
            16.0
        }
    };
    let line_height_px = resolve_length_px(Some(&dom.style(el, "lineHeight")), font_size);
    let letter_spacing_px = resolve_length_px(Some(&dom.style(el, "letterSpacing")), font_size);
    let rect = dom.rect(el);
    let line_max = config.line_max();
    let viewport_width = {
        let w = dom.inner_width();
        if crate::js_ext_a::num_truthy(w) {
            w
        } else {
            0.0
        }
    };
    check_quality(
        dom,
        &QualityInput {
            el,
            tag,
            has_direct_text,
            text_len,
            font_size,
            line_height_px,
            letter_spacing_px,
            rect,
            line_max,
            viewport_width,
        },
    )
}

/// JS: checks.mjs#checkPageQualityFromDoc(doc)
pub fn check_page_quality_from_doc(dom: &dyn Dom) -> Vec<RuleHit> {
    let mut findings = Vec::new();
    let mut prev_level: i64 = 0;
    let mut prev_text = String::new();
    for h in dom.query_all(None, "h1, h2, h3, h4, h5, h6").unwrap_or_default() {
        let tag = dom.tag_name(h);
        // JS `parseInt(h.tagName[1])`
        let level = js::parse_int(&tag.chars().nth(1).map(|c| c.to_string()).unwrap_or_default(), 10);
        let level = if level.is_nan() { 0 } else { level as i64 };
        let text = slice_utf16_prefix(&collapse_ws(js::trim(&dom.text_content(h))), 60);
        if prev_level > 0 && level > prev_level + 1 {
            findings.push(RuleHit::new(
                "skipped-heading",
                format!(
                    "<h{}> \"{}\" followed by <h{}> \"{}\" (missing h{})",
                    prev_level,
                    prev_text,
                    level,
                    text,
                    prev_level + 1
                ),
            ));
        }
        prev_level = level;
        prev_text = text;
    }
    findings
}

/// JS: checks.mjs#checkPageQualityDOM() — `{ type, detail }` shape.
pub fn check_page_quality_dom(dom: &dyn Dom) -> Vec<BrowserFinding> {
    check_page_quality_from_doc(dom)
        .iter()
        .map(BrowserFinding::from_hit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    fn raster(d: &mut FakeDom, parent: ElId, tag: &str, rect: (f64, f64, f64, f64)) -> ElId {
        let el = d.add(Some(parent), tag);
        d.set_styles(el, &[("opacity", "0"), ("backgroundImage", "none"), ("filter", "none")]);
        d.set_rect(el, rect.0, rect.1, rect.2, rect.3);
        el
    }

    fn buried(d: &FakeDom, el: ElId) -> bool {
        check_quality(
            d,
            &QualityInput {
                el,
                tag: tag_lower(d, el),
                has_direct_text: false,
                text_len: 0,
                font_size: 16.0,
                line_height_px: None,
                letter_spacing_px: None,
                rect: d.rect(el),
                line_max: 80.0,
                viewport_width: 1280.0,
            },
        )
        .iter()
        .any(|h| h.id == "buried-raster")
    }

    /// climatempo.com.br's icon states, picomq.com's copy button,
    /// exxonmobil.com's blur-up placeholders, resurf.so's crossfade frames.
    #[test]
    fn buried_raster_skips_state_layers() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let photo = raster(&mut d, body, "img", (0.0, 0.0, 480.0, 300.0));
        d.set_attr(photo, "src", "/texture.png");
        assert!(buried(&d, photo));
        d.set_attr(photo, "src", "/dist/images/v2/svg/location-granted.svg");
        assert!(!buried(&d, photo), "vector art");

        let copy = raster(&mut d, body, "button", (0.0, 400.0, 480.0, 300.0));
        d.set_style(copy, "backgroundImage", "url(\"data:image/svg+xml,%3Csvg%3E\")");
        assert!(!buried(&d, copy), "an SVG data URI");

        let icon = raster(&mut d, body, "img", (0.0, 800.0, 16.0, 16.0));
        d.set_attr(icon, "src", "/pin.png");
        assert!(!buried(&d, icon), "an icon-sized raster");

        let placeholder = raster(&mut d, body, "canvas", (0.0, 1000.0, 353.0, 199.0));
        d.set_style(placeholder, "backgroundImage", "url(\"/keytopic.jpg?w=40\")");
        assert!(buried(&d, placeholder));
        d.set_style(placeholder, "filter", "blur(10px)");
        assert!(!buried(&d, placeholder), "a blurred placeholder");

        let card = d.add(Some(body), "article");
        d.set_style(card, "backgroundImage", "url(\"/keytopic.jpg?w=2048\")");
        d.set_rect(card, 16.0, 1600.0, 321.0, 181.0);
        let under = raster(&mut d, card, "canvas", (0.0, 1590.0, 353.0, 199.0));
        d.set_style(under, "backgroundImage", "url(\"/keytopic.jpg?w=40\")");
        assert!(!buried(&d, under), "under a parent painting the loaded picture");

        let stack = d.add(Some(body), "div");
        let shown = raster(&mut d, stack, "img", (160.0, 3012.0, 960.0, 600.0));
        d.set_style(shown, "opacity", "1");
        let frame = raster(&mut d, stack, "img", (160.0, 3012.0, 960.0, 600.0));
        d.set_attr(frame, "src", "/screenshot-inbox.png");
        assert!(!buried(&d, frame), "a crossfade frame under a painted sibling");
        d.set_style(shown, "opacity", "0");
        assert!(buried(&d, frame), "no painted frame over it");
    }

    /// copperhead.sh: `<sup><a>[7]</a></sup>` at 10.2px.
    #[test]
    fn undersized_ui_text_skips_links_inside_markers() {
        let ui = |d: &FakeDom, el: ElId| {
            check_quality(
                d,
                &QualityInput {
                    el,
                    tag: "a".to_string(),
                    has_direct_text: true,
                    text_len: 3,
                    font_size: 10.2,
                    line_height_px: None,
                    letter_spacing_px: None,
                    rect: Rect::from_xywh(0.0, 0.0, 12.0, 13.0),
                    line_max: 80.0,
                    viewport_width: 1280.0,
                },
            )
            .iter()
            .any(|h| h.id == "undersized-ui-text")
        };
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = d.add(Some(body), "p");
        d.add_text(p, "The board was routed in one pass");
        let sup = d.add(Some(p), "sup");
        let link = d.add(Some(sup), "a");
        d.add_text(link, "[7]");
        d.add_selector(link, INTERACTIVE);
        assert!(!ui(&d, link));
        let nav_link = d.add(Some(body), "a");
        d.add_text(nav_link, "[7]");
        d.add_selector(nav_link, INTERACTIVE);
        assert!(ui(&d, nav_link));
    }

    fn text_el(d: &mut FakeDom, body: ElId, tag: &str, text: &str, font: &str) -> ElId {
        let p = d.add(Some(body), tag);
        d.add_text(p, text);
        d.set_styles(
            p,
            &[
                ("fontSize", font),
                ("lineHeight", "normal"),
                ("letterSpacing", "normal"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("position", "static"),
                ("textTransform", "none"),
                ("textAlign", "start"),
            ],
        );
        p
    }

    #[test]
    fn line_length_and_viewport_edge() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let long = "x".repeat(120);
        let p = text_el(&mut d, body, "p", &long, "16px");
        d.set_rect(p, 0.0, 100.0, 1200.0, 40.0);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"line-length"), "{ids:?}");
        assert_eq!(hits[0].snippet, "~150 chars/line (aim for <80)");
        assert!(ids.contains(&"body-text-viewport-edge"));
        let edge = hits.iter().find(|h| h.id == "body-text-viewport-edge").unwrap();
        assert_eq!(edge.snippet, "<p> with 120-char body bleeds to viewport edge (left 0px)");
        // narrower, inset paragraph: neither fires
        d.set_rect(p, 40.0, 100.0, 600.0, 40.0);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert!(hits.is_empty(), "{hits:?}");
    }

    #[test]
    fn cramped_padding_vertical() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let p = text_el(&mut d, body, "div", &"word ".repeat(10), "16px");
        d.set_rect(p, 40.0, 100.0, 300.0, 60.0);
        d.set_styles(
            p,
            &[
                ("backgroundColor", "rgb(240, 240, 240)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("paddingTop", "2px"),
                ("paddingBottom", "12px"),
                ("paddingLeft", "12px"),
                ("paddingRight", "12px"),
            ],
        );
        // The glyphs sit 2px under the top edge, as the padding says.
        d.set_text_rect(p, 52.0, 102.0, 200.0, 40.0);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "2px vertical padding (need ≥4.8px for 16px text)");
    }

    /// A fixed-height flex row centres its label on zero padding: the padding
    /// property says 0, the reader sees 12px. Only a box where the glyphs
    /// really do crowd the edge is cramped.
    #[test]
    fn cramped_padding_reads_the_text_box_not_the_padding() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let btn = text_el(&mut d, body, "div", "Book a free consultation", "14px");
        d.set_rect(btn, 0.0, 0.0, 240.0, 40.0);
        d.set_styles(
            btn,
            &[
                ("display", "flex"),
                ("backgroundColor", "rgb(37, 99, 235)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("paddingTop", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "32px"),
                ("paddingRight", "32px"),
            ],
        );
        // 14px label centred in the 40px box: 12px of air above and below.
        d.set_text_rect(btn, 32.0, 12.0, 160.0, 16.0);
        assert!(check_element_quality_dom(&d, btn, &BrowserConfig::default()).is_empty());

        // Same declared padding, but the label fills the box.
        d.set_text_rect(btn, 32.0, 1.0, 160.0, 38.0);
        let hits = check_element_quality_dom(&d, btn, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "0px vertical padding (need ≥4.2px for 14px text)");
    }

    /// `border: 1px solid transparent` (a focus-ring placeholder) draws no
    /// edge, so a full-width row with no side padding crowds nothing.
    #[test]
    fn cramped_padding_ignores_a_transparent_border() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let row = text_el(&mut d, body, "div", "What does the free plan include?", "18px");
        d.set_rect(row, 0.0, 0.0, 640.0, 60.0);
        d.set_styles(
            row,
            &[
                ("paddingTop", "20px"),
                ("paddingBottom", "20px"),
                ("paddingLeft", "0px"),
                ("paddingRight", "0px"),
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgba(0, 0, 0, 0)"),
                ("borderRightColor", "rgba(0, 0, 0, 0)"),
                ("borderBottomColor", "rgba(0, 0, 0, 0)"),
                ("borderLeftColor", "rgba(0, 0, 0, 0)"),
            ],
        );
        d.set_text_rect(row, 0.0, 21.0, 400.0, 18.0);
        assert!(check_element_quality_dom(&d, row, &BrowserConfig::default()).is_empty());
    }

    #[test]
    fn flush_children_against_border() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let card = d.add(Some(body), "section");
        d.set_attr(card, "class", "card-frame extra");
        d.set_rect(card, 0.0, 0.0, 400.0, 200.0);
        d.set_styles(
            card,
            &[
                ("position", "static"),
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgb(0, 0, 0)"),
                ("borderRightColor", "rgb(0, 0, 0)"),
                ("borderBottomColor", "rgb(0, 0, 0)"),
                ("borderLeftColor", "rgb(0, 0, 0)"),
                ("outlineWidth", "0px"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("paddingTop", "28px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("fontSize", "16px"),
            ],
        );
        // No text rect: a Dom that cannot measure glyphs keeps the boxes this
        // rule read before.
        let p = text_el(&mut d, card, "p", "Hello there friend", "16px");
        d.set_rect(p, 0.0, 28.0, 400.0, 20.0);
        d.set_styles(p, &[("paddingTop", "0px"), ("paddingRight", "0px"), ("paddingBottom", "0px"), ("paddingLeft", "0px"), ("marginTop", "0px"), ("marginRight", "0px"), ("marginBottom", "0px"), ("marginLeft", "0px")]);
        let hits = check_element_quality_dom(&d, card, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<section> \"card-frame\": children flush against border on right/left (no inset)"
        );
    }

    /// The accordion row every component library emits: `div.border` >
    /// `h3` > `button.py-4`. The direct child carries no padding, so the
    /// insulation test sees nothing; the button's box fills the row; only
    /// its text rect shows the 16px the reader gets.
    fn accordion_row(d: &mut FakeDom) -> (ElId, ElId) {
        let (_h, body) = d.with_page();
        let row = d.add(Some(body), "div");
        d.set_attr(row, "class", "border");
        d.set_rect(row, 0.0, 0.0, 600.0, 58.0);
        d.set_styles(
            row,
            &[
                ("position", "static"),
                ("display", "block"),
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgb(200, 200, 200)"),
                ("borderRightColor", "rgb(200, 200, 200)"),
                ("borderBottomColor", "rgb(200, 200, 200)"),
                ("borderLeftColor", "rgb(200, 200, 200)"),
                ("outlineWidth", "0px"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("paddingTop", "0px"),
                ("paddingRight", "24px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "24px"),
                ("fontSize", "16px"),
            ],
        );
        let h3 = d.add(Some(row), "h3");
        d.set_rect(h3, 24.0, 0.0, 552.0, 58.0);
        let zero = [
            ("paddingTop", "0px"),
            ("paddingRight", "0px"),
            ("paddingBottom", "0px"),
            ("paddingLeft", "0px"),
            ("marginTop", "0px"),
            ("marginRight", "0px"),
            ("marginBottom", "0px"),
            ("marginLeft", "0px"),
        ];
        d.set_styles(h3, &zero);
        let button = d.add(Some(h3), "button");
        d.add_text(button, "Which games does it work with?");
        d.set_rect(button, 24.0, 0.0, 552.0, 58.0);
        d.set_styles(button, &zero);
        (row, button)
    }

    #[test]
    fn flush_children_measure_text_not_boxes() {
        let mut d = FakeDom::new();
        let (row, button) = accordion_row(&mut d);
        // The button's own padding puts its label 20px off both rules.
        d.set_text_rect(button, 24.0, 20.0, 300.0, 18.0);
        assert!(check_element_quality_dom(&d, row, &BrowserConfig::default()).is_empty());

        // A row whose two-line label really does run into the rules.
        d.set_text_rect(button, 24.0, 1.0, 300.0, 56.0);
        let hits = check_element_quality_dom(&d, row, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<div> \"border\": children flush against border on top/bottom (no inset)"
        );
    }

    #[test]
    fn flush_ignores_hidden_and_clipped_text() {
        let mut d = FakeDom::new();
        let (row, button) = accordion_row(&mut d);
        d.set_text_rect(button, 24.0, 20.0, 300.0, 18.0);

        // A screen-reader-only heading at the box origin paints nothing.
        let sr = d.add(Some(row), "h2");
        d.add_text(sr, "Frequently asked questions");
        d.set_rect(sr, 24.0, 0.0, 1.0, 1.0);
        d.set_text_rect(sr, 24.0, 0.0, 200.0, 16.0);
        d.add_selector(sr, SR_ONLY_SELECTOR);
        assert!(check_element_quality_dom(&d, row, &BrowserConfig::default()).is_empty());

        // The collapsed answer panel lays its text out below the row and
        // clips every pixel of it away.
        let panel = d.add(Some(row), "div");
        d.set_rect(panel, 24.0, 58.0, 552.0, 0.0);
        d.set_styles(panel, &[("overflow", "hidden")]);
        let answer = d.add(Some(panel), "p");
        d.add_text(answer, "Any game with a public leaderboard.");
        d.set_rect(answer, 24.0, 58.0, 552.0, 20.0);
        d.set_text_rect(answer, 24.0, 59.0, 400.0, 18.0);
        assert!(check_element_quality_dom(&d, row, &BrowserConfig::default()).is_empty());
    }

    /// White on an unpainted page draws no edge, so the page shell is not a
    /// card whose text is flush against anything.
    #[test]
    fn white_on_the_canvas_is_not_a_boundary() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgba(0, 0, 0, 0)");
        let shell = d.add(Some(body), "div");
        d.set_attr(shell, "class", "wrapper");
        d.set_rect(shell, 0.0, 0.0, 990.0, 400.0);
        d.set_styles(
            shell,
            &[
                ("position", "static"),
                ("display", "block"),
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("outlineWidth", "0px"),
                ("paddingTop", "0px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("fontSize", "16px"),
            ],
        );
        assert!(!has_visible_background_boundary(&d, shell));
        let p = text_el(&mut d, shell, "p", "Today's headlines, in full", "16px");
        d.set_rect(p, 0.0, 0.0, 990.0, 20.0);
        d.set_text_rect(p, 0.0, 2.0, 400.0, 16.0);
        assert!(check_element_quality_dom(&d, shell, &BrowserConfig::default()).is_empty());

        // A tinted card on the same page still bounds its text.
        d.set_style(shell, "backgroundColor", "rgb(15, 23, 42)");
        let hits = check_element_quality_dom(&d, shell, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<div> \"wrapper\": children flush against bg on top/left (no inset)"
        );

        // The same white shell on a page that asks for a dark scheme sits on
        // the browser's dark canvas, where it is a strong edge.
        d.set_style(shell, "backgroundColor", "rgb(255, 255, 255)");
        d.set_style(shell, "colorScheme", "dark");
        assert!(has_visible_background_boundary(&d, shell));
        assert_eq!(
            check_element_quality_dom(&d, shell, &BrowserConfig::default()).len(),
            1
        );
    }

    /// A chip `height` tall on a tinted fill, 2px of vertical padding, and one
    /// label whose box and text rect are given. Returns `(chip, label)`.
    fn chip(d: &mut FakeDom, height: f64, font: &str, line_height: &str, label: (f64, f64)) -> (ElId, ElId) {
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let chip = d.add(Some(body), "div");
        d.set_attr(chip, "class", "faq-content__step");
        d.set_rect(chip, 620.0, 100.0, 66.0, height);
        d.set_styles(
            chip,
            &[
                ("position", "static"),
                ("display", "flex"),
                ("backgroundColor", "rgb(255, 170, 1)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("outlineWidth", "0px"),
                ("paddingTop", "2px"),
                ("paddingRight", "8px"),
                ("paddingBottom", "2px"),
                ("paddingLeft", "8px"),
                ("fontSize", "16px"),
            ],
        );
        let span = text_el(d, chip, "span", "Point", font);
        d.set_styles(
            span,
            &[
                ("display", "block"),
                ("lineHeight", line_height),
                ("paddingTop", "0px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("marginTop", "0px"),
                ("marginRight", "0px"),
                ("marginBottom", "0px"),
                ("marginLeft", "0px"),
            ],
        );
        let (top, h) = label;
        d.set_rect(span, 628.0, 100.0 + top, 41.0, h);
        d.set_text_rect(span, 628.0, 100.0 + top, 41.0, h);
        (chip, span)
    }

    fn cramped(d: &FakeDom, el: ElId) -> Vec<String> {
        check_element_quality_dom(d, el, &BrowserConfig::default())
            .into_iter()
            .filter(|h| h.id == "cramped-padding")
            .map(|h| h.snippet)
            .collect()
    }

    /// Taste call r3-20: a chip at most 28px tall is measured by its glyphs,
    /// half-leading included. yungching.com.tw's 24px step chip sets a 14px
    /// label on a 20px `normal` line 2px off its edges; the glyphs' em box
    /// sits 5px off. haraj.com.sa's 28px price chip holds a 19px content
    /// area 4px off on a 24px line; its em box is 5.5px off.
    #[test]
    fn cramped_padding_measures_small_chips_by_their_glyphs() {
        let mut d = FakeDom::new();
        let (step, _) = chip(&mut d, 24.0, "14px", "normal", (2.0, 20.0));
        assert!(cramped(&d, step).is_empty(), "{:?}", cramped(&d, step));

        let mut d = FakeDom::new();
        let (price, _) = chip(&mut d, 28.0, "16px", "24px", (4.0, 19.0));
        assert!(cramped(&d, price).is_empty(), "28px is still a small chip");

        // The same label geometry past 28px keeps the content-area measure.
        let mut d = FakeDom::new();
        let (tall, _) = chip(&mut d, 30.0, "14px", "normal", (2.0, 20.0));
        assert_eq!(
            cramped(&d, tall),
            vec!["<div> \"faq-content__step\": children flush against bg on top (no inset)"]
        );

        // A chip whose glyphs really do touch its edges still reports: a 16px
        // label on a 16px line in a 20px chip, its 19px content area 0.5px
        // off, keeps its em box 2px off the top and bottom.
        let mut d = FakeDom::new();
        let (touching, label) = chip(&mut d, 20.0, "16px", "16px", (2.0, 16.0));
        d.set_text_rect(label, 628.0, 100.5, 41.0, 19.0);
        assert_eq!(
            cramped(&d, touching),
            vec!["<div> \"faq-content__step\": children flush against bg on top/bottom (no inset)"]
        );
    }

    #[test]
    fn glyph_band_centres_the_em_box_on_each_line() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let one = text_el(&mut d, body, "span", "Point", "14px");
        let t = Rect::from_xywh(0.0, 10.0, 40.0, 20.0);
        let g = glyph_band(&d, one, &t);
        assert_eq!((g.top, g.bottom, g.left, g.right), (13.0, 27.0, 0.0, 40.0));

        // Two 18px lines 18px apart: the content area is 21.6px (1.2em)
        // and the union 39.6px; the em box moves 1.8px in at each end.
        let two = text_el(&mut d, body, "span", "Two lines of label text", "18px");
        d.set_style(two, "lineHeight", "18px");
        let t = Rect::from_xywh(0.0, 0.0, 200.0, 39.6);
        let g = glyph_band(&d, two, &t);
        assert!((g.top - 1.8).abs() < 1e-9 && (g.bottom - 37.8).abs() < 1e-9, "{g:?}");

        // A content area no taller than the font size has nothing to take off.
        let tight = text_el(&mut d, body, "span", "Tight face", "16px");
        let t = Rect::from_xywh(0.0, 0.0, 60.0, 15.0);
        assert_eq!(glyph_band(&d, tight, &t), t);
    }

    /// `direct_text_rect` is a font-metric box, not an ink box: half-leading
    /// and scripts with tall marks push it out of the box that paints the
    /// text, where no glyph can land. Measure the part inside that box.
    #[test]
    fn flush_clamps_text_to_the_box_that_paints_it() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let panel = d.add(Some(body), "div");
        d.set_attr(panel, "class", "story-body");
        d.set_rect(panel, 0.0, 0.0, 360.0, 120.0);
        d.set_styles(
            panel,
            &[
                ("position", "static"),
                ("display", "block"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "0px"),
                ("borderBottomColor", "rgb(200, 200, 200)"),
                ("outlineWidth", "0px"),
                ("paddingTop", "0px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("fontSize", "16px"),
            ],
        );
        // A plain wrapper, so the paragraph below is not a direct child and
        // the child-box insulation says nothing about it.
        let inner = d.add(Some(panel), "div");
        d.set_rect(inner, 0.0, 0.0, 360.0, 120.0);
        let p = text_el(&mut d, inner, "p", "एक पूरी कहानी यहाँ पढ़ें", "16px");
        d.set_styles(
            p,
            &[
                ("paddingTop", "0px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("marginTop", "0px"),
                ("marginRight", "0px"),
                ("marginBottom", "0px"),
                ("marginLeft", "0px"),
            ],
        );
        // The paragraph ends 10px above the rule; its metric box runs 15px
        // past its own box and so past the rule.
        d.set_rect(p, 0.0, 10.0, 360.0, 100.0);
        d.set_text_rect(p, 0.0, 6.0, 340.0, 119.0);
        assert!(check_element_quality_dom(&d, panel, &BrowserConfig::default()).is_empty());

        // The shape the corpus confirms harmful: the label's own box overruns
        // the panel and its glyphs come with it.
        d.set_rect(p, 0.0, 10.0, 360.0, 115.0);
        d.set_text_rect(p, 0.0, 12.0, 340.0, 108.0);
        let hits = check_element_quality_dom(&d, panel, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<div> \"story-body\": children flush against border-bottom on bottom (no inset)"
        );
    }

    #[test]
    fn typography_rules() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let text = "a".repeat(90);
        let p = text_el(&mut d, body, "p", &text, "16px");
        d.set_rect(p, 40.0, 100.0, 300.0, 40.0);
        d.set_styles(p, &[("lineHeight", "16px"), ("textAlign", "justify"), ("hyphens", "manual"), ("letterSpacing", "2px")]);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["tight-leading", "justified-text", "wide-tracking"], "{hits:?}");
        assert_eq!(hits[0].snippet, "line-height 1.00x (need >=1.3)");
        assert_eq!(hits[2].snippet, "letter-spacing: 0.13em on body text");
        d.set_styles(p, &[("lineHeight", "24px"), ("textAlign", "left"), ("letterSpacing", "-1.6px"), ("textTransform", "uppercase")]);
        // 90 characters of uppercase: past the length where a run is read.
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["all-caps-body", "extreme-negative-tracking"], "{hits:?}");
        assert_eq!(hits[1].snippet, format!("letter-spacing: -0.10em at 16px — \"{}\"", "a".repeat(40)));
    }

    #[test]
    fn crushed_tracking_is_size_scaled_and_latin_only() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let latin = "Tightened headline copy running past twenty characters";

        let flags = |d: &FakeDom, el: ElId| {
            check_element_quality_dom(d, el, &BrowserConfig::default())
                .into_iter()
                .any(|h| h.id == "extreme-negative-tracking")
        };

        // Reading size: -0.05em (Tailwind's tracking-tighter) and -0.06em pass.
        let p = text_el(&mut d, body, "p", latin, "16px");
        d.set_rect(p, 40.0, 100.0, 300.0, 40.0);
        for ls in ["-0.05em", "-0.06em"] {
            d.set_styles(p, &[("letterSpacing", ls)]);
            assert!(!flags(&d, p), "{ls} at 16px should pass");
        }
        d.set_styles(p, &[("letterSpacing", "-0.08em")]);
        assert!(flags(&d, p), "-0.08em at 16px should flag");

        // Display size: the same -0.08em is conventional optical tightening.
        let h = text_el(&mut d, body, "h1", latin, "48px");
        d.set_rect(h, 40.0, 200.0, 600.0, 60.0);
        d.set_styles(h, &[("letterSpacing", "-0.08em")]);
        assert!(!flags(&d, h), "-0.08em at 48px should pass");
        d.set_styles(h, &[("letterSpacing", "-0.1em")]);
        assert!(flags(&d, h), "-0.1em at 48px should flag");

        // CJK glyphs are out of scope whatever the lang attribute says.
        let cjk = text_el(&mut d, body, "p", "赓续长征精神奋进复兴征程福建守护红色家底新时代新征程", "18px");
        d.set_rect(cjk, 40.0, 300.0, 300.0, 40.0);
        d.set_styles(cjk, &[("letterSpacing", "-0.1em")]);
        assert!(!flags(&d, cjk), "CJK text should pass");
    }

    /// The tight-leading floor is a body-copy floor. Reviewing the rule's
    /// output on real pages found it applied to display type, to heading text
    /// that sits in a child anchor or span, to text that renders one line, to
    /// source text nothing typesets, and to pages that set the floor exactly.
    #[test]
    fn tight_leading_carve_outs() {
        const COPY: &str = "This card description is comfortably longer than the fifty characters the leading check asks for.";

        fn leading(d: &FakeDom, el: ElId) -> Vec<String> {
            check_element_quality_dom(d, el, &BrowserConfig::default())
                .into_iter()
                .filter(|h| h.id == "tight-leading")
                .map(|h| h.snippet)
                .collect()
        }
        // Two wrapped lines of 16px copy on a 20px line box.
        fn wrapped(d: &mut FakeDom, parent: ElId, tag: &str, font: &str, lh: f64) -> ElId {
            let el = text_el(d, parent, tag, COPY, font);
            d.set_style(el, "lineHeight", &format!("{lh}px"));
            d.set_rect(el, 40.0, 100.0, 240.0, lh * 2.0);
            d.el_mut(el).direct_text_rect = Some(Rect::from_xywh(40.0, 100.0, 240.0, lh * 2.0));
            el
        }

        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();

        // Body copy under the floor: the case the rule is for.
        let copy = wrapped(&mut d, body, "p", "16px", 20.0);
        assert_eq!(leading(&d, copy), vec!["line-height 1.25x (need >=1.3)"]);

        // Display type carries its own leading.
        let display = wrapped(&mut d, body, "p", "32px", 38.4);
        assert!(leading(&d, display).is_empty(), "32px display type");
        let boundary = wrapped(&mut d, body, "p", "22px", 26.0);
        assert_eq!(
            leading(&d, boundary),
            vec!["line-height 1.18x (need >=1.3)"],
            "22px is still reading copy"
        );

        // One line box: there is no gap between lines to crowd.
        let one_line = wrapped(&mut d, body, "p", "16px", 20.0);
        d.set_rect(one_line, 40.0, 100.0, 900.0, 20.0);
        d.el_mut(one_line).direct_text_rect = Some(Rect::from_xywh(40.0, 100.0, 900.0, 20.0));
        assert!(leading(&d, one_line).is_empty(), "single line box");

        // Heading text in a child anchor, and a card title on the ARIA role.
        let h3 = d.add(Some(body), "h3");
        let link = wrapped(&mut d, h3, "a", "18px", 21.6);
        assert!(leading(&d, link).is_empty(), "anchor inside a heading");
        let titled = wrapped(&mut d, body, "span", "18px", 21.6);
        d.add_selector(titled, "[role=\"heading\"]");
        assert!(leading(&d, titled).is_empty(), "role=heading card title");
        // A block of reading copy nested inside a heading is still body copy.
        let nested = wrapped(&mut d, h3, "p", "16px", 17.6);
        assert_eq!(
            leading(&d, nested),
            vec!["line-height 1.10x (need >=1.3)"],
            "paragraph nested in a heading"
        );

        // line-height: 1.3 on 18px computes to 23.4px, and 23.4 / 18 lands
        // just under 1.3 in binary floats.
        let at_floor = wrapped(&mut d, body, "p", "18px", 23.4);
        assert!(leading(&d, at_floor).is_empty(), "exactly at the floor");

        // Source text nothing typesets, and text with no box at all.
        let script = wrapped(&mut d, body, "script", "16px", 16.0);
        assert!(leading(&d, script).is_empty(), "script source");
        let hidden = wrapped(&mut d, body, "p", "16px", 16.0);
        d.set_style(hidden, "display", "none");
        assert!(leading(&d, hidden).is_empty(), "display:none");
        let sr = wrapped(&mut d, body, "p", "16px", 16.0);
        d.add_selector(sr, SR_ONLY_SELECTOR);
        assert!(leading(&d, sr).is_empty(), "screen-reader-only copy");
        let boxless = wrapped(&mut d, body, "p", "16px", 16.0);
        d.set_rect(boxless, 0.0, 0.0, 0.0, 0.0);
        d.el_mut(boxless).direct_text_rect = None;
        assert!(leading(&d, boxless).is_empty(), "zero-area box");
    }

    fn snippets(d: &FakeDom, el: ElId, rule: &str) -> Vec<String> {
        check_element_quality_dom(d, el, &BrowserConfig::default())
            .into_iter()
            .filter(|h| h.id == rule)
            .map(|h| h.snippet)
            .collect()
    }

    /// observations-20 row 8: the estimate read the box, so a one-line note
    /// in a wide box, a centred footer line and a block that never fills its
    /// column reported. Where the text is measured, the text decides.
    #[test]
    fn line_length_measures_the_rendered_text() {
        let long = "word ".repeat(40);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &long, "16px");
        d.set_style(p, "lineHeight", "24px");
        d.set_rect(p, 40.0, 100.0, 1200.0, 48.0);

        // Two lines that fill their 1,200px box keep the box estimate.
        d.set_text_rect(p, 40.0, 102.0, 1180.0, 44.0);
        assert_eq!(snippets(&d, p, "line-length"), vec!["~150 chars/line (aim for <80)"]);
        // One line in the same box sends the eye nowhere.
        d.set_text_rect(p, 40.0, 102.0, 1180.0, 20.0);
        assert!(snippets(&d, p, "line-length").is_empty(), "one line");
        // Centred lines well short of the box: 119 characters on two lines of
        // at most 600px of glyphs is ~75 chars a line.
        let centred = text_el(&mut d, body, "p", &"word ".repeat(24), "16px");
        d.set_style(centred, "lineHeight", "24px");
        d.set_rect(centred, 40.0, 200.0, 1200.0, 48.0);
        d.set_text_rect(centred, 340.0, 202.0, 600.0, 44.0);
        assert!(snippets(&d, centred, "line-length").is_empty(), "centred block");
        // With no text rect the box stands in, as before.
        d.el_mut(p).direct_text_rect = None;
        assert_eq!(snippets(&d, p, "line-length"), vec!["~150 chars/line (aim for <80)"]);

        // `line-height: normal` counts lines at 1.2em.
        d.set_style(p, "lineHeight", "normal");
        d.set_text_rect(p, 40.0, 102.0, 1180.0, 19.0);
        assert!(snippets(&d, p, "line-length").is_empty(), "one line at normal");
        d.set_text_rect(p, 40.0, 102.0, 1180.0, 38.0);
        assert_eq!(snippets(&d, p, "line-length").len(), 1, "two lines at normal");

        // A line holds no more characters than the block: 94 of them on two
        // lines, not the 150 the box would fit.
        let short = text_el(&mut d, body, "p", &"word ".repeat(19), "16px");
        d.set_style(short, "lineHeight", "24px");
        d.set_rect(short, 40.0, 300.0, 1200.0, 48.0);
        d.set_text_rect(short, 40.0, 302.0, 1180.0, 44.0);
        assert_eq!(snippets(&d, short, "line-length"), vec!["~94 chars/line (aim for <80)"]);

        // hnmatchmaker.com: 175 characters on two lines of a narrow 12px face.
        // The widest line is 486px, 89% of its box, within what the half-em
        // advance runs wide of a narrow face: the box estimate stands, as
        // base printed it.
        let teaser = text_el(&mut d, body, "p", &"word ".repeat(35), "12px");
        d.set_style(teaser, "lineHeight", "16px");
        d.set_rect(teaser, 73.0, 500.0, 544.0, 32.0);
        d.set_text_rect(teaser, 73.0, 500.0, 486.0, 32.0);
        assert_eq!(snippets(&d, teaser, "line-length"), vec!["~91 chars/line (aim for <80)"]);
        // Lines well short of the box (beside a float) are read from the
        // widest line, ~67 half-em glyphs here; but some line holds at least
        // the average of 87 when all of the block's text sits in these lines.
        d.set_text_rect(teaser, 73.0, 500.0, 400.0, 32.0);
        assert_eq!(snippets(&d, teaser, "line-length"), vec!["~87 chars/line (aim for <80)"]);
        // A block holding a component: its text is not all in these lines,
        // so only the widest line counts.
        let card = d.add(Some(teaser), "div");
        d.set_style(card, "display", "block");
        assert!(snippets(&d, teaser, "line-length").is_empty());

        // simplybudget.framer.ai: 94 characters on two lines ended by a
        // `<br>`, 660px of glyphs in a 760px box. The author broke them, so
        // the widest line decides however much of the box it fills.
        let broken = text_el(&mut d, body, "p", &"word ".repeat(19), "16px");
        d.set_style(broken, "lineHeight", "28.8px");
        d.set_rect(broken, 260.0, 900.0, 760.0, 57.6);
        d.set_text_rect(broken, 260.0, 904.0, 660.0, 49.0);
        assert_eq!(snippets(&d, broken, "line-length"), vec!["~94 chars/line (aim for <80)"], "no break");
        let br = d.add(Some(broken), "br");
        d.set_style(br, "display", "inline");
        assert!(snippets(&d, broken, "line-length").is_empty(), "~83 on the line the break ends");
        // A broken block whose lines fill 90% of the box is a column the
        // breaks never shortened, and the box estimate stands.
        d.set_text_rect(broken, 260.0, 904.0, 700.0, 49.0);
        assert_eq!(snippets(&d, broken, "line-length"), vec!["~94 chars/line (aim for <80)"], "a full column");
    }

    /// Review of observations-20 row 8. A Range rect spans one content area
    /// plus a pitch per extra line, so two lines at `line-height: 2.4` are
    /// shorter than 1.5 pitches and read as one. And a narrow sans fills 80
    /// to 90% of its box with as many characters as the box estimate says.
    #[test]
    fn line_length_counts_lines_by_the_line_height_and_keeps_nearly_full_lines() {
        let copy = "A long reading paragraph set with a very airy line-height of 2.4, set across a very wide column with no max-width at all, so the lines run far past a comfortable measure and the eye has a long way to travel back to the start of the next line every single time it reaches the end of one of them.";
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        // prose.html `pl-lh`: Georgia at 16px on 38.4px lines, 1,100px wide.
        let airy = text_el(&mut d, body, "p", copy, "16px");
        d.set_style(airy, "lineHeight", "38.4px");
        d.set_rect(airy, 24.0, 100.0, 1100.0, 76.8);
        d.set_text_rect(airy, 24.0, 110.0, 1090.0, 56.6);
        assert_eq!(snippets(&d, airy, "line-length"), vec!["~138 chars/line (aim for <80)"], "two lines");
        d.set_text_rect(airy, 24.0, 110.0, 1090.0, 18.2);
        assert!(snippets(&d, airy, "line-length").is_empty(), "one line at the same pitch");

        // veeza.ai 106327: DM Sans at 16px on 26px lines, 621.6px of glyphs
        // (88% of a 704px box) that really hold about 86 characters.
        let item = text_el(
            &mut d,
            body,
            "li",
            "Our AI fills the official application and prepares your supporting documents. You book your own appointment.",
            "16px",
        );
        d.set_style(item, "lineHeight", "26px");
        d.set_rect(item, 288.0, 670.5, 704.0, 52.0);
        d.set_text_rect(item, 288.0, 672.5, 621.578125, 47.0);
        assert_eq!(snippets(&d, item, "line-length"), vec!["~88 chars/line (aim for <80)"]);
    }

    /// observations-20 row 29: a full-width CJK glyph is an em wide, so the
    /// half-em estimate doubled so-net.ne.jp's count.
    #[test]
    fn line_length_counts_cjk_glyphs_at_an_em() {
        let copy = "戸建/マンションは、NTTから送付される「開通のご案内」に記載の「ご利用サービス名」など、回線事業者からの案内をご確認のうえタイプに合ったコースをお選びください。".repeat(2);
        assert!(utf16_len(&copy) > 80);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &copy, "16px");
        d.set_style(p, "lineHeight", "24px");
        d.set_rect(p, 110.0, 100.0, 1060.0, 72.0);
        d.set_text_rect(p, 110.0, 104.0, 1048.0, 64.0);
        assert!(snippets(&d, p, "line-length").is_empty(), "~68 glyphs a line");
        // The box estimate reads the script too.
        d.el_mut(p).direct_text_rect = None;
        assert!(snippets(&d, p, "line-length").is_empty());
        // A wider CJK column still reports, at its own count.
        d.set_rect(p, 0.0, 100.0, 1600.0, 72.0);
        let hits = snippets(&d, p, "line-length");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].starts_with("~10"), "{hits:?}");
    }

    /// walkthroughs-20 miss 2: prose whose words sit wholly in `<b>`, `<i>` or
    /// `<span>` was never measured, because the paragraph has no direct text.
    #[test]
    fn prose_in_inline_children_is_measured_on_its_paragraph() {
        let long = "word ".repeat(40);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.inner_width = 1280.0;
        let intro = text_el(&mut d, body, "p", "", "16px");
        d.set_styles(intro, &[("lineHeight", "24px"), ("display", "block")]);
        d.set_rect(intro, 0.0, 100.0, 1280.0, 72.0);
        let b = d.add(Some(intro), "b");
        d.set_style(b, "display", "inline");
        d.add_text(b, "Opening words ");
        d.set_text_rect(b, 0.0, 102.0, 120.0, 20.0);
        let i = d.add(Some(intro), "i");
        d.set_style(i, "display", "inline");
        d.add_text(i, &long);
        d.set_text_rect(i, 0.0, 102.0, 1270.0, 68.0);
        assert_eq!(snippets(&d, intro, "line-length"), vec!["~160 chars/line (aim for <80)"]);
        let len = utf16_len(js::trim(&d.text_content(intro)));
        assert_eq!(
            snippets(&d, intro, "body-text-viewport-edge"),
            vec![format!("<p> with {len}-char body bleeds to viewport edge (left 0px / right 0px)")]
        );
        // The inline children report neither rule themselves.
        for child in [b, i] {
            let hits = check_element_quality_dom(&d, child, &BrowserConfig::default());
            assert!(
                !hits.iter().any(|h| h.id == "line-length" || h.id == "body-text-viewport-edge"),
                "{hits:?}"
            );
        }
        // Inline prose the Dom cannot measure stays silent, as before.
        d.el_mut(b).direct_text_rect = None;
        d.el_mut(i).direct_text_rect = None;
        assert!(snippets(&d, intro, "line-length").is_empty());
        assert!(snippets(&d, intro, "body-text-viewport-edge").is_empty());
        // A paragraph holding a block component is not inline prose.
        d.set_text_rect(i, 0.0, 102.0, 1270.0, 68.0);
        let card = d.add(Some(intro), "div");
        d.set_style(card, "display", "block");
        assert!(snippets(&d, intro, "line-length").is_empty());
    }

    /// observations-20 row 31: a centred or padded paragraph spans the
    /// viewport with its box while its glyphs keep a gutter, and a slide cut
    /// by its carousel track meets the track's clip, not the page edge.
    #[test]
    fn viewport_edge_measures_the_text_not_the_box() {
        let copy = "Two sides. One rivalry. Zero middle ground. Show them where you stand today.";
        let len = utf16_len(copy);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.inner_width = 1280.0;
        let p = text_el(&mut d, body, "p", copy, "16px");
        d.set_style(p, "lineHeight", "24px");
        d.set_rect(p, 0.0, 100.0, 1280.0, 24.0);
        // Centred glyphs, 300px off both edges.
        d.set_text_rect(p, 300.0, 102.0, 680.0, 20.0);
        assert!(snippets(&d, p, "body-text-viewport-edge").is_empty(), "centred");
        // Padded: the glyphs start 32px in.
        d.set_text_rect(p, 32.0, 102.0, 680.0, 20.0);
        assert!(snippets(&d, p, "body-text-viewport-edge").is_empty(), "padded");
        // Glyphs at the edge report, with the text's own distances.
        d.set_text_rect(p, 0.0, 102.0, 680.0, 20.0);
        assert_eq!(
            snippets(&d, p, "body-text-viewport-edge"),
            vec![format!("<p> with {len}-char body bleeds to viewport edge (left 0px)")]
        );
        // With no text rect the box stands in, as before.
        d.el_mut(p).direct_text_rect = None;
        assert_eq!(
            snippets(&d, p, "body-text-viewport-edge"),
            vec![format!("<p> with {len}-char body bleeds to viewport edge (left 0px / right 0px)")]
        );

        // Text a box that only hides overflow cuts at 1,270px: an
        // `overflow-hidden` section (v0-optimus-delta.vercel.app) cannot be
        // told from a carousel track, and the text reports as base did. Its
        // two lines fill the paragraph, so the paragraph's edge is the one
        // printed.
        let track = d.add(Some(body), "div");
        d.set_styles(track, &[("overflowX", "hidden"), ("overflow", "hidden")]);
        d.set_rect(track, 10.0, 300.0, 1260.0, 200.0);
        d.el_mut(track).client_width = 1260.0;
        d.el_mut(track).scroll_width = 1590.0;
        let slide = text_el(&mut d, track, "p", copy, "16px");
        d.set_style(slide, "lineHeight", "24px");
        d.set_rect(slide, 900.0, 320.0, 700.0, 48.0);
        d.set_text_rect(slide, 900.0, 322.0, 690.0, 44.0);
        let cut = vec![format!("<p> with {len}-char body bleeds to viewport edge (right -320px)")];
        assert_eq!(snippets(&d, slide, "body-text-viewport-edge"), cut, "cut by overflow: hidden");
        d.set_styles(track, &[("overflowX", "hidden"), ("overflow", "hidden auto")]);
        assert_eq!(snippets(&d, slide, "body-text-viewport-edge"), cut, "cut by overflow-x-hidden");
        // A track that scrolls on x, with the slide to scroll to, brings the
        // text into view: its clip is the track's, not the page's gutter.
        d.set_styles(track, &[("overflowX", "auto"), ("overflow", "auto")]);
        assert!(snippets(&d, slide, "body-text-viewport-edge").is_empty(), "a swiped track");
        // Out of the track, the same text runs off the page and reports.
        d.set_styles(track, &[("overflowX", "visible"), ("overflow", "visible")]);
        assert_eq!(snippets(&d, slide, "body-text-viewport-edge"), cut);

        // Wrapped lines that fill a padded paragraph sit on its content box.
        let padded = text_el(&mut d, body, "p", copy, "16px");
        d.set_styles(padded, &[("lineHeight", "24px"), ("paddingLeft", "24px"), ("paddingRight", "24px")]);
        d.set_rect(padded, 0.0, 700.0, 1280.0, 48.0);
        d.set_text_rect(padded, 24.0, 702.0, 1220.0, 44.0);
        assert!(snippets(&d, padded, "body-text-viewport-edge").is_empty(), "24px padding");
        d.set_styles(padded, &[("paddingLeft", "8px"), ("paddingRight", "8px")]);
        d.set_text_rect(padded, 8.0, 702.0, 1230.0, 44.0);
        assert_eq!(
            snippets(&d, padded, "body-text-viewport-edge"),
            vec![format!("<p> with {len}-char body bleeds to viewport edge (left 8px / right 8px)")]
        );

        // A list item's `inside` marker paints at the content edge ahead of
        // its text, so the start side reaches that edge.
        let li = text_el(&mut d, body, "li", copy, "16px");
        d.set_styles(li, &[("lineHeight", "24px"), ("display", "list-item"), ("paddingLeft", "0px"), ("borderLeftWidth", "0px")]);
        d.set_rect(li, 0.0, 600.0, 1280.0, 24.0);
        d.set_text_rect(li, 18.0, 602.0, 700.0, 20.0);
        assert_eq!(
            snippets(&d, li, "body-text-viewport-edge"),
            vec![format!("<li> with {len}-char body bleeds to viewport edge (left 0px)")]
        );
        // Given a gutter of its own, the item keeps off the edge.
        d.set_style(li, "paddingLeft", "24px");
        d.set_text_rect(li, 42.0, 602.0, 700.0, 20.0);
        assert!(snippets(&d, li, "body-text-viewport-edge").is_empty());
    }

    /// observations-20 row 40: an inline run at `line-height: 11px` inside a
    /// 14px block sits on 14px lines, and a label at 18px inside a 22.4px
    /// block sits on 22.4px ones.
    /// Taste call r3-03: bold titles set on a div or span get the heading
    /// exemption at two rendered lines or fewer, or in a line clamp.
    #[test]
    fn tight_leading_exempts_bold_titles() {
        const TITLE: &str = "Two southern residents charged over an international trafficking ring";
        fn title(d: &mut FakeDom, parent: ElId, weight: &str, lines: f64) -> ElId {
            let el = text_el(d, parent, "div", TITLE, "14px");
            d.set_styles(el, &[("lineHeight", "16px"), ("fontWeight", weight), ("display", "block")]);
            let h = 16.8 + 16.0 * (lines - 1.0);
            d.set_rect(el, 355.0, 300.0, 265.0, 16.0 * lines);
            d.el_mut(el).direct_text_rect = Some(Rect::from_xywh(355.0, 300.0, 265.0, h));
            el
        }
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();

        let bold = title(&mut d, body, "700", 2.0);
        assert!(snippets(&d, bold, "tight-leading").is_empty(), "bold, two lines");
        let semibold = title(&mut d, body, "600", 2.0);
        assert!(snippets(&d, semibold, "tight-leading").is_empty(), "600 is bold");
        let medium = title(&mut d, body, "500", 2.0);
        assert_eq!(snippets(&d, medium, "tight-leading"), vec!["line-height 1.14x (need >=1.3)"]);
        let three = title(&mut d, body, "700", 3.0);
        assert_eq!(
            snippets(&d, three, "tight-leading"),
            vec!["line-height 1.14x (need >=1.3)"],
            "bold body text of three lines keeps the floor"
        );

        // A clamp exempts a bold title however many lines it lays out.
        // Current Chrome computes a CSS clamp that takes effect as
        // `flow-root` carrying `webkitLineClamp`; a `-webkit-box` whose
        // clamp is left to a script (Taboola's) computes as itself.
        let boxed = title(&mut d, body, "670", 4.0);
        d.set_styles(boxed, &[("display", "-webkit-box"), ("webkitLineClamp", "none")]);
        assert!(snippets(&d, boxed, "tight-leading").is_empty(), "-webkit-box clamp");
        let flow = title(&mut d, body, "670", 4.0);
        d.set_styles(
            flow,
            &[("display", "flow-root"), ("webkitLineClamp", "2"), ("overflow", "hidden"), ("overflowY", "hidden")],
        );
        d.set_rect(flow, 355.0, 300.0, 265.0, 32.0);
        assert!(snippets(&d, flow, "tight-leading").is_empty(), "flow-root clamp");
        // A clamp of three lines that hides nothing still exempts.
        let exact = title(&mut d, body, "700", 3.0);
        d.set_styles(
            exact,
            &[("display", "flow-root"), ("webkitLineClamp", "3"), ("overflow", "hidden"), ("overflowY", "hidden")],
        );
        assert!(snippets(&d, exact, "tight-leading").is_empty(), "clamp of three, nothing hidden");
        // A clipping flow-root box with no clamp only cuts the count: three
        // of six laid-out lines render, and bold text of three lines reports.
        let maxed = title(&mut d, body, "700", 6.0);
        d.set_styles(
            maxed,
            &[("display", "flow-root"), ("webkitLineClamp", "none"), ("overflow", "hidden"), ("overflowY", "hidden")],
        );
        d.set_rect(maxed, 355.0, 300.0, 265.0, 48.0);
        assert_eq!(snippets(&d, maxed, "tight-leading").len(), 1, "max-height clip, no clamp");
        // The same box cut to two lines is a two-line bold title.
        d.set_rect(maxed, 355.0, 300.0, 265.0, 32.0);
        assert!(snippets(&d, maxed, "tight-leading").is_empty(), "clipped to two lines");
        // A clamp on a plain block clamps nothing, and is no clamp.
        let stray = title(&mut d, body, "670", 4.0);
        d.set_styles(stray, &[("display", "block"), ("webkitLineClamp", "2")]);
        assert_eq!(snippets(&d, stray, "tight-leading").len(), 1, "clamp on a plain block");
        // Regular copy in a clamp keeps the floor.
        let regular = title(&mut d, body, "400", 4.0);
        d.set_styles(regular, &[("display", "flow-root"), ("webkitLineClamp", "2")]);
        assert_eq!(snippets(&d, regular, "tight-leading").len(), 1, "regular weight in a clamp");

        // ynet.co.il's `div.slotTitle.medium`: the headline's own div lays out
        // three lines, and the clamped wrapper two levels up (through the
        // link) shows two of them.
        let wrapper = d.add(Some(body), "div");
        d.set_styles(
            wrapper,
            &[("display", "flow-root"), ("webkitLineClamp", "2"), ("overflow", "hidden"), ("overflowY", "hidden")],
        );
        d.set_rect(wrapper, 230.0, 4109.0, 190.0, 38.0);
        let link = d.add(Some(wrapper), "a");
        d.set_style(link, "display", "inline");
        let headline = text_el(&mut d, link, "div", TITLE, "15px");
        d.set_styles(headline, &[("display", "block"), ("lineHeight", "19px"), ("fontWeight", "670")]);
        d.set_rect(headline, 230.0, 4109.0, 190.0, 38.0);
        d.el_mut(headline).direct_text_rect = Some(Rect::from_xywh(252.0, 4111.0, 167.0, 53.0));
        assert!(snippets(&d, headline, "tight-leading").is_empty(), "two of three lines render");
        d.set_style(headline, "fontWeight", "400");
        assert_eq!(snippets(&d, headline, "tight-leading").len(), 1, "regular weight keeps the floor");

        // An inline bold run wrapping to two lines on its block.
        let block = d.add(Some(body), "div");
        d.set_styles(block, &[("display", "block"), ("fontSize", "16px"), ("lineHeight", "normal")]);
        let run = text_el(&mut d, block, "span", TITLE, "16px");
        d.set_styles(run, &[("display", "inline"), ("lineHeight", "19px"), ("fontWeight", "670")]);
        d.set_rect(run, 466.0, 8866.0, 374.0, 34.0);
        d.el_mut(run).direct_text_rect = Some(Rect::from_xywh(466.0, 8866.0, 374.0, 34.0));
        assert!(snippets(&d, run, "tight-leading").is_empty(), "inline bold run, two lines");
    }

    #[test]
    fn tight_leading_reads_the_block_an_inline_run_sits_on() {
        const COPY: &str = "Free furniture, free books, free clothes, free computers, and more besides.";
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let block = d.add(Some(body), "div");
        d.set_styles(block, &[("display", "inline-block"), ("fontSize", "14px"), ("lineHeight", "14px")]);
        let run = text_el(&mut d, block, "span", COPY, "11px");
        d.set_styles(run, &[("display", "inline"), ("lineHeight", "11px")]);
        d.set_rect(run, 46.0, 100.0, 298.0, 40.0);
        d.set_text_rect(run, 46.0, 100.0, 280.0, 40.0);
        assert_eq!(snippets(&d, run, "tight-leading"), vec!["line-height 1.27x (need >=1.3)"]);
        d.set_style(block, "lineHeight", "22.4px");
        assert!(snippets(&d, run, "tight-leading").is_empty(), "set on the block's 22.4px");
        // A block strut that cannot be resolved leaves the run's own value.
        d.set_style(block, "lineHeight", "normal");
        assert_eq!(snippets(&d, run, "tight-leading"), vec!["line-height 1.00x (need >=1.3)"]);
    }

    /// walkthroughs-20 note 13: tchibo.de sets its teaser headlines as
    /// `<h5><div>…</div></h5>`. Any box inside a heading carries heading text,
    /// unless it is a reading block nested there.
    #[test]
    fn tight_leading_exempts_heading_copy_in_a_block_wrapper() {
        const COPY: &str = "Jede Woche neu! Lassen Sie sich von unseren Kollektionen immer wieder neu inspirieren";
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let h5 = d.add(Some(body), "h5");
        let wrapper = text_el(&mut d, h5, "div", COPY, "19px");
        d.set_style(wrapper, "lineHeight", "24px");
        d.set_rect(wrapper, 12.0, 100.0, 366.0, 72.0);
        d.set_text_rect(wrapper, 12.0, 100.0, 330.0, 71.0);
        assert!(snippets(&d, wrapper, "tight-leading").is_empty(), "div in a heading");
        // A paragraph of body copy in a heading keeps the floor, and so does
        // what sits inside it.
        let para = text_el(&mut d, h5, "p", COPY, "16px");
        d.set_style(para, "lineHeight", "17.6px");
        d.set_rect(para, 12.0, 200.0, 300.0, 70.4);
        d.set_text_rect(para, 12.0, 200.0, 300.0, 70.4);
        assert_eq!(snippets(&d, para, "tight-leading"), vec!["line-height 1.10x (need >=1.3)"]);
        let inner = text_el(&mut d, para, "div", COPY, "16px");
        d.set_style(inner, "lineHeight", "17.6px");
        d.set_rect(inner, 12.0, 300.0, 300.0, 70.4);
        d.set_text_rect(inner, 12.0, 300.0, 300.0, 70.4);
        assert_eq!(snippets(&d, inner, "tight-leading").len(), 1, "a box inside the paragraph");
    }

    /// observations-20 row 41: a two-line inline highlight reports the union
    /// of its fragments, 43px, while each fragment is one 21px line.
    #[test]
    fn cramped_padding_judges_an_inline_box_per_line() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let hl = text_el(&mut d, body, "span", "carrier's own estimating guide", "14px");
        d.set_styles(
            hl,
            &[
                ("display", "inline"),
                ("lineHeight", "25.9px"),
                ("backgroundColor", "rgb(254, 240, 138)"),
                ("borderTopWidth", "0px"),
                ("borderRightWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("borderLeftWidth", "0px"),
                ("paddingTop", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "5px"),
                ("paddingRight", "5px"),
            ],
        );
        d.set_rect(hl, 55.0, 100.0, 234.0, 42.9);
        d.set_text_rect(hl, 55.0, 100.0, 234.0, 42.9);
        assert!(snippets(&d, hl, "cramped-padding").is_empty(), "two one-line fragments");
        // A box one 43px line tall is past the gate.
        d.set_style(hl, "display", "inline-block");
        assert_eq!(
            snippets(&d, hl, "cramped-padding"),
            vec!["0px vertical padding (need ≥4.2px for 14px text)"]
        );
    }

    #[test]
    fn wide_tracking_exempts_short_capital_labels() {
        // A tracked label: one line, capitals, inside the label length.
        let label_text = "LIMITED EDITION RELEASE 2026";
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let s = text_el(&mut d, body, "span", label_text, "12px");
        d.set_rect(s, 40.0, 100.0, 260.0, 18.0);
        d.set_styles(s, &[("lineHeight", "18px"), ("letterSpacing", "2px")]);
        d.els[s as usize].direct_text_rect = Some(Rect::from_xywh(40.0, 100.0, 260.0, 18.0));
        assert!(check_element_quality_dom(&d, s, &BrowserConfig::default()).is_empty());

        // The same label typed lowercase keeps the finding.
        let mixed = text_el(&mut d, body, "span", "Fall 2026 technology preview", "12px");
        d.set_rect(mixed, 40.0, 130.0, 260.0, 18.0);
        d.set_styles(mixed, &[("lineHeight", "18px"), ("letterSpacing", "2px")]);
        d.els[mixed as usize].direct_text_rect = Some(Rect::from_xywh(40.0, 130.0, 260.0, 18.0));
        let hits = check_element_quality_dom(&d, mixed, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["wide-tracking"], "{hits:?}");

        // The same words declared uppercase were always exempt.
        let up = text_el(&mut d, body, "span", "Fall 2026 technology preview", "12px");
        d.set_rect(up, 40.0, 160.0, 260.0, 18.0);
        d.set_styles(
            up,
            &[("lineHeight", "18px"), ("letterSpacing", "2px"), ("textTransform", "uppercase")],
        );
        d.els[up as usize].direct_text_rect = Some(Rect::from_xywh(40.0, 160.0, 260.0, 18.0));
        assert!(check_element_quality_dom(&d, up, &BrowserConfig::default()).is_empty());

        // Typed capitals over two lines are no longer a label.
        d.set_rect(s, 40.0, 100.0, 140.0, 36.0);
        d.els[s as usize].direct_text_rect = Some(Rect::from_xywh(40.0, 100.0, 140.0, 36.0));
        let hits = check_element_quality_dom(&d, s, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["wide-tracking"], "{hits:?}");

        // Past the label length, one line or not, it is running text.
        let long = text_el(
            &mut d,
            body,
            "p",
            "SUPPORT HOURS RUN MONDAY TO FRIDAY FROM NINE UNTIL SIX",
            "16px",
        );
        d.set_rect(long, 40.0, 200.0, 600.0, 26.0);
        d.set_styles(long, &[("lineHeight", "26px"), ("letterSpacing", "2px")]);
        d.els[long as usize].direct_text_rect = Some(Rect::from_xywh(40.0, 200.0, 600.0, 26.0));
        let hits = check_element_quality_dom(&d, long, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["wide-tracking"], "{hits:?}");
        assert_eq!(hits[0].snippet, "letter-spacing: 0.13em on body text");
    }

    /// Uppercase costs reading only when the run is long enough to be read as
    /// a sentence. A label keeps its capitals however narrow the viewport is,
    /// and however much text the element's children hold.
    #[test]
    fn all_caps_body_needs_a_long_run() {
        let caps = |text: &str| -> Vec<RuleHit> {
            let mut d = FakeDom::new();
            let (_h, body) = d.with_page();
            let el = text_el(&mut d, body, "span", text, "12px");
            d.set_rect(el, 40.0, 100.0, 300.0, 18.0);
            d.set_styles(el, &[("lineHeight", "18px"), ("textTransform", "uppercase")]);
            check_element_quality_dom(&d, el, &BrowserConfig::default())
        };
        let flagged = |hits: &[RuleHit]| hits.iter().any(|h| h.id == "all-caps-body");

        // A card CTA or an eyebrow: conventional at any width, so silent.
        let label = "How Mintlify is scaling sales-led GTM";
        assert!(!flagged(&caps(label)), "37-char label");

        // Real sites run labels into the seventies; a sentence starts at 80.
        let long = "Every order placed before noon ships the same day from our warehouse today";
        assert_eq!(utf16_len(long), 74);
        assert!(!flagged(&caps(long)), "74-char label");
        let longer = format!("{long} or later");
        assert_eq!(utf16_len(&longer), 83);
        let hits = caps(&longer);
        assert!(flagged(&hits), "83-char run: {hits:?}");
        assert_eq!(
            hits.iter().find(|h| h.id == "all-caps-body").unwrap().snippet,
            "text-transform: uppercase on 83 chars of body text"
        );

        // The run is measured as rendered: markup whitespace collapses.
        let spaced = format!("\n      {longer}\n    ");
        let hits = caps(&spaced);
        assert_eq!(
            hits.iter().find(|h| h.id == "all-caps-body").unwrap().snippet,
            "text-transform: uppercase on 83 chars of body text"
        );

        // A bar whose own label is short and whose child holds a second one:
        // neither run is a sentence, so neither element is charged for both.
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let bar = text_el(&mut d, body, "div", "Audit trail complete ", "13px");
        d.set_rect(bar, 40.0, 100.0, 300.0, 20.0);
        d.set_styles(bar, &[("lineHeight", "20px"), ("textTransform", "uppercase")]);
        let badge_text = "immutable log of every configuration change your team makes";
        let badge = text_el(&mut d, bar, "span", badge_text, "13px");
        d.set_rect(badge, 40.0, 100.0, 300.0, 20.0);
        d.set_styles(badge, &[("lineHeight", "20px"), ("textTransform", "uppercase")]);
        // The subtree reaches the sentence length; neither run in it does.
        assert_eq!(utf16_len(&d.text_content(bar)), 80);
        assert_eq!(utf16_len(badge_text), 59);
        for el in [bar, badge] {
            let hits = check_element_quality_dom(&d, el, &BrowserConfig::default());
            assert!(!flagged(&hits), "{hits:?}");
        }
    }

    #[test]
    fn justified_text_narrows_to_rivers() {
        let latin = "word ".repeat(24);
        let chinese = "永慶房屋於一九八八年成立專注本業堅持創新秉持先誠實再成交的精神".to_string();
        let arabic = "يعتمد ضبط النص في الخط العربي على استطالة الحروف على السطر".to_string();
        let thai = "การจัดวางข้อความแบบชิดขอบทั้งสองด้านในภาษาไทยไม่ได้ดึงช่องว่าง".to_string();
        let justified = |text: &str, width: f64, hyphens: &str| {
            let mut d = FakeDom::new();
            let (_h, body) = d.with_page();
            let p = text_el(&mut d, body, "p", text, "16px");
            d.set_rect(p, 40.0, 100.0, width, 60.0);
            d.set_styles(p, &[("lineHeight", "26px"), ("textAlign", "justify"), ("hyphens", hyphens)]);
            let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
            hits.iter().any(|h| h.id == "justified-text")
        };

        // 300px at 16px is ~37 chars/line; 800px is ~100.
        assert!(justified(&latin, 300.0, "manual"));
        assert!(!justified(&latin, 800.0, "manual"));
        assert!(!justified(&latin, 300.0, "auto"));
        // Scripts that justify without stretching word spaces.
        assert!(!justified(&chinese, 300.0, "manual"));
        assert!(!justified(&arabic, 300.0, "manual"));
        assert!(!justified(&thai, 300.0, "manual"));
        // A Latin paragraph carrying a few ideographs is still Latin.
        assert!(justified(&format!("{latin} 永慶房屋"), 300.0, "manual"));
    }

    #[test]
    fn tiny_and_undersized_text() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", "This is small body copy text", "10px");
        d.set_rect(p, 40.0, 100.0, 300.0, 40.0);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, vec!["tiny-text"], "{hits:?}");
        assert_eq!(hits[0].snippet, "10px body text");
        // short functional label under 11px
        let s = text_el(&mut d, body, "span", "Meta 12:00", "9px");
        d.set_rect(s, 40.0, 100.0, 60.0, 12.0);
        let hits = check_element_quality_dom(&d, s, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "9px functional text \"Meta 12:00\" (below 11px floor)");
        // smallprint context softens the floor to 10px
        d.add_selector(s, SMALLPRINT);
        d.set_style(s, "fontSize", "10px");
        assert!(check_element_quality_dom(&d, s, &BrowserConfig::default()).is_empty());
        // sr-only exempts
        d.set_style(s, "fontSize", "9px");
        d.add_selector(s, SR_ONLY_SELECTOR);
        assert!(check_element_quality_dom(&d, s, &BrowserConfig::default()).is_empty());
    }

    /// Taste call r3-19: each floor keeps a 0.1px tolerance, so fluid type a
    /// hair under it stops reporting and text 0.1px under or more reports.
    #[test]
    fn undersized_ui_text_tolerates_fluid_sizes_a_hair_under_the_floor() {
        fn undersized(d: &FakeDom, el: ElId) -> Vec<String> {
            check_element_quality_dom(d, el, &BrowserConfig::default())
                .into_iter()
                .filter(|h| h.id == "undersized-ui-text")
                .map(|h| h.snippet)
                .collect()
        }
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let s = text_el(&mut d, body, "span", "透天厝", "10.9688px");
        d.set_rect(s, 23.0, 711.0, 33.0, 11.0);
        assert!(undersized(&d, s).is_empty(), "0.03px under 11px");
        d.set_style(s, "fontSize", "10.95px");
        assert!(undersized(&d, s).is_empty(), "0.05px under 11px");
        d.set_style(s, "fontSize", "10.9px");
        assert_eq!(undersized(&d, s), vec!["10.9px functional text \"透天厝\" (below 11px floor)"]);
        d.set_style(s, "fontSize", "10.944px");
        assert!(undersized(&d, s).is_empty(), "swipeloan.in's slider ticks");

        // The smallprint floor keeps the same tolerance.
        let legal = text_el(&mut d, body, "span", "Terms apply", "9.95px");
        d.set_rect(legal, 23.0, 800.0, 60.0, 11.0);
        d.add_selector(legal, SMALLPRINT);
        assert!(undersized(&d, legal).is_empty(), "0.05px under 10px");
        d.set_style(legal, "fontSize", "9.9px");
        assert_eq!(undersized(&d, legal), vec!["9.9px functional text \"Terms apply\" (below 10px floor)"]);

        // Text in a link keeps the interactive floor, smallprint class or not
        // (taste call r3-07, kept).
        let card = d.add(Some(body), "a");
        d.set_attr(card, "href", "/story");
        d.add_selector(card, INTERACTIVE);
        let credit = text_el(&mut d, card, "div", "Photo: Reuters", "10.5px");
        d.set_rect(credit, 23.0, 900.0, 80.0, 12.0);
        d.add_selector(credit, INTERACTIVE);
        d.add_selector(credit, SMALLPRINT);
        assert_eq!(
            undersized(&d, credit),
            vec!["10.5px functional text \"Photo: Reuters\" (below 11px floor)"]
        );
    }

    #[test]
    fn skipped_heading() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let h1 = d.add(Some(body), "h1");
        d.add_text(h1, "  Title   here ");
        let h3 = d.add(Some(body), "h3");
        d.add_text(h3, "Sub");
        let f = check_page_quality_dom(&d);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].type_, "skipped-heading");
        assert_eq!(f[0].detail, "<h1> \"Title here\" followed by <h3> \"Sub\" (missing h2)");
    }

    #[test]
    fn visually_hidden_and_non_rendered() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let s = d.add(Some(body), "span");
        d.set_styles(s, &[("position", "absolute"), ("clip", "rect(0px, 0px, 0px, 0px)")]);
        assert!(is_visually_hidden(&d, s));
        d.set_styles(s, &[("clip", "auto"), ("width", "1px"), ("height", "20px"), ("overflow", "hidden")]);
        assert!(is_visually_hidden(&d, s));
        d.set_style(s, "overflow", "visible");
        assert!(!is_visually_hidden(&d, s));
        assert!(is_non_rendered_text(&d, s, "script"));
        d.set_style(s, "visibility", "collapse");
        assert!(is_non_rendered_text(&d, s, "span"));
    }
}
