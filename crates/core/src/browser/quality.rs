//! `checkQuality` and its browser adapters from `checks.mjs` Section 5:
//! `checkQuality` (every branch, including the rect-gated ones the static
//! engine never reaches), `checkElementQualityDOM`,
//! `hasVisibleBackgroundBoundary`, `hasMeaningfulDirectText`,
//! `textDescendantsFlushSides`, `isVisuallyHidden`, `isNonRenderedText`,
//! `checkPageQualityFromDoc`, `checkPageQualityDOM`.

#![allow(unused_imports)]
use super::dom::{
    closest_or_none, direct_text, has_direct_text_longer_than, matches_or_false, pf0, safe_id,
    renders_no_text, style_px, tag_lower, Dom, DomChild, ElId, Rect,
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
    average_glyph_advance_em_at, is_cjk_text, justifies_without_word_spaces_text, tracking_is_crushed,
    ALL_CAPS_LONG_RUN,
    JUSTIFY_NARROW_CHARS_PER_LINE, LEADING_DISPLAY_TYPE_PX, LEADING_HEADING_CONTEXT,
    LEADING_MIN_LINE_BOXES, NON_RENDERED_TAGS, QUALITY_TEXT_TAGS,
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

/// The width of every line the element's text rendered on, or `None` when
/// the DOM cannot say where the lines are.
///
/// `Dom::text_line_rects` has already merged the fragments of a line back
/// together, so each rect here is one line box and nothing is divided by
/// anything: a leading tighter than the glyph box used to turn one rect into
/// two identical "lines" and charge a single long line twice.
fn rendered_line_widths(dom: &dyn Dom, el: ElId) -> Option<Vec<f64>> {
    Some(
        dom.text_line_rects(el)?
            .into_iter()
            .filter(|r| r.width > 0.0 && r.height > 0.0)
            .map(|r| r.width)
            .collect(),
    )
}

static COMBINING_OR_FORMAT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\p{M}\p{Cf}]").expect("COMBINING_OR_FORMAT_RE"));

/// The running count of [`rendered_text_len`], fed one text node at a time.
///
/// White space is held back until a character that is not white space
/// follows it, so a leading run is dropped and a trailing run never counts:
/// the ends are trimmed as `String.prototype.trim` trims them, wherever the
/// node boundaries fall. A run of collapsible white space counts once, and it
/// runs on across node boundaries, the way `a <b> b</b>` renders one space.
#[derive(Default)]
struct RenderedTextCount {
    count: usize,
    /// White space seen since the last counted character.
    pending: usize,
    /// The last white space seen was collapsible, so more of it joins that run.
    in_collapsible_run: bool,
    /// Count white space that cannot collapse (preserved, or a no-break
    /// space) where it stands, edges included. Set for an atomic inline's
    /// contents, whose edges trim only collapsible white space.
    keep_fixed_spaces: bool,
}

impl RenderedTextCount {
    fn feed(&mut self, text: &str, preserved: bool) {
        let collapsible = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}');
        let mut buf = [0u8; 4];
        for c in text.chars() {
            if collapsible(c) && !preserved {
                if !self.in_collapsible_run {
                    self.pending += 1;
                    self.in_collapsible_run = true;
                }
                continue;
            }
            if js::is_js_whitespace(c) && !self.keep_fixed_spaces {
                self.pending += 1;
                self.in_collapsible_run = false;
                continue;
            }
            // A mark is not on the line; it neither ends a run of white space
            // nor counts as a character.
            if COMBINING_OR_FORMAT_RE.is_match(c.encode_utf8(&mut buf)) {
                continue;
            }
            if self.count > 0 {
                self.count += self.pending;
            }
            self.pending = 0;
            self.in_collapsible_run = false;
            self.count += 1;
        }
    }

    /// Takes in an atomic inline (an image, an inline-block) whose own
    /// contents counted `inner` characters. The box sits in the line like a
    /// character, so a collapsible space on each side of it renders: the run
    /// ends at the box. Its contents were counted in a count of their own,
    /// because they are laid out in their own formatting context: their edge
    /// collapsible white space is trimmed there and never joins the outer
    /// run, while preserved and no-break spaces at the edges still render.
    fn add_atomic_inline(&mut self, inner: usize) {
        self.in_collapsible_run = false;
        if inner == 0 {
            return;
        }
        if self.count > 0 {
            self.count += self.pending;
        }
        self.pending = 0;
        self.count += inner;
    }
}

/// Replaced elements, which lay out as atomic inlines in a line of text.
const REPLACED_TAGS: [&str; 11] =
    ["img", "svg", "video", "canvas", "iframe", "object", "embed", "input", "select", "textarea", "button"];

/// An atomic inline box: a replaced element or an `inline-*` display.
fn is_atomic_inline(dom: &dyn Dom, el: ElId) -> bool {
    REPLACED_TAGS.contains(&tag_lower(dom, el).as_str())
        || js::to_lower_case(&dom.style(el, "display")).trim().starts_with("inline-")
}

/// Feeds the text nodes under `el` in document order, each under its own
/// parent's `white-space`, skipping every descendant that renders no text
/// ([`renders_no_text`]: unrendered tags, `display: none`, and the contents
/// of a `content-visibility: hidden` box). `Dom::text_line_rects` skips the
/// same subtrees, so the count and the line widths it is divided among
/// always describe the same text.
fn feed_rendered_text(dom: &dyn Dom, el: ElId, out: &mut RenderedTextCount) {
    let mut preserved: Option<bool> = None;
    for child in dom.child_nodes(el) {
        match child {
            DomChild::Text(text) => {
                let preserved = *preserved.get_or_insert_with(|| {
                    let white_space = dom.style(el, "whiteSpace");
                    white_space == "pre" || white_space == "pre-wrap" || white_space == "break-spaces"
                });
                out.feed(&text, preserved);
            }
            DomChild::Element(child) => {
                if renders_no_text(dom, child) {
                    continue;
                }
                if is_atomic_inline(dom, child) {
                    let mut inner = RenderedTextCount { keep_fixed_spaces: true, ..Default::default() };
                    feed_rendered_text(dom, child, &mut inner);
                    out.add_atomic_inline(inner.count);
                } else {
                    feed_rendered_text(dom, child, out);
                }
            }
        }
    }
}

/// How many characters of `el`'s text reached its rendered lines, which is
/// the count the line rects have to be divided among.
///
/// `textContent` is the source, not the rendering, and it runs over in three
/// ways. It includes the text of an inline `<style>` or `<script>` child and
/// of a `display: none` one, none of which is on any line. It keeps the
/// source's indentation and newlines, which `white-space: normal` collapses
/// to a single space. And it counts code units, so a combining mark, which
/// sits on its base and advances nothing, is charged as a character of its
/// own: a third of a Devanagari paragraph is marks.
///
/// So the count is built from the text nodes themselves, in order: the
/// unrendered descendants are never visited, each node's collapsible white
/// space is folded to one space unless the element it sits in preserves it
/// (a `pre-wrap` span inside a normal paragraph keeps its spaces, which are
/// on the line), and combining marks and format characters (zero-width
/// joiners, soft hyphens) are left out.
fn rendered_text_len(dom: &dyn Dom, el: ElId) -> usize {
    let mut count = RenderedTextCount::default();
    feed_rendered_text(dom, el, &mut count);
    count.count
}

/// JS: checks.mjs#textDescendantsFlushSides(el, rect) → [top, right, bottom, left]
///
/// Each candidate is measured by its own text rect, not by its border box: a
/// padded button, a centred heading and a table cell all fill the box they sit
/// in while their glyphs stay well inside it, and it is the glyphs a reader
/// sees crowding the boundary.
pub fn text_descendants_flush_sides(dom: &dyn Dom, el: ElId, rect: &Rect) -> [bool; 4] {
    let mut flush = [false; 4];
    const TEXT_EDGE_THRESHOLD: f64 = 4.0;
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
        // this rule had before, rather than going silent.
        let nr = match dom.direct_text_rect(node) {
            Some(t) if t.width > 0.0 && t.height > 0.0 => match clamp_to(&t, &br) {
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
pub fn is_heading_text(dom: &dyn Dom, el: ElId) -> bool {
    if matches_or_false(dom, el, LEADING_HEADING_CONTEXT) {
        return true;
    }
    let Some(heading) = closest_or_none(dom, el, LEADING_HEADING_CONTEXT) else {
        return false;
    };
    // An inline tag (an anchor, a span) is heading text too, by the same
    // walk: no reading block sits between it and the heading. One inside a
    // `p` nested in the heading is that paragraph's body copy.
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
    //
    // The measure is the line that rendered, not the box that could have held
    // it. `rect.width / (fontSize * 0.5)` is the box's capacity: a paragraph
    // sitting in a 1022px column whose text stops at 571px was charged with
    // 142 characters a line it never rendered (REN-402). What the reader sees
    // is `text_line_rects`, one rect per line box with the fragments of a
    // line merged back together, and the characters divide between the lines
    // in proportion to the ink each carries — one element's text is one font
    // at one size, so the average advance is the same on every line of it.
    //
    // The rects cover the element's whole rendered text, descendants and all,
    // so the characters divided among them are the rendered ones too
    // (`rendered_text_len`): measuring the direct text alone and then
    // charging it with the characters of an inline `<strong>` inflated every
    // paragraph that had one, and so does charging the lines with an inline
    // `<style>` child, the source's indentation, or a script's combining
    // marks, none of which takes up any of a line.
    //
    // A DOM that cannot say where the lines are gets no finding. The union of
    // a long first line and a short tail is the same union as two even lines,
    // so there is nothing in it to read a line off, and a rule that guesses
    // there is charging noise.
    //
    // Charged when at least two rendered lines run past the maximum. The harm
    // this rule names is the eye losing its place tracking back to the start
    // of the next line, so it takes a column of long lines to do the damage;
    // one long line and a short tail is a sentence that wrapped once.
    //
    // Prose whose words sit wholly in inline children is read the same way:
    // the rects cover every text node under the element, so the block that
    // sets the lines is the one measured.
    let prose_in_phrasing =
        !has_direct_text && LINE_PROSE_TAGS.contains(&tag) && holds_only_phrasing(dom, el);
    if (has_direct_text || prose_in_phrasing)
        && QUALITY_TEXT_TAGS.contains(&tag)
        && rect.width > 0.0
        && (text_len as f64) > line_max
    {
        if let Some(widths) = rendered_line_widths(dom, el) {
            let total: f64 = widths.iter().sum();
            if total > 0.0 {
                let over = line_max + 5.0;
                let rendered_len = rendered_text_len(dom, el) as f64;
                let chars = |w: f64| rendered_len * w / total;
                let long = widths.iter().filter(|w| chars(**w) > over).count();
                if long >= 2 {
                    let longest = widths.iter().copied().fold(0.0, js::math_max);
                    findings.push(RuleHit::new(
                        "line-length",
                        format!(
                            "~{} chars on {} of {} rendered lines (aim for <{})",
                            number_to_string(math_round(chars(longest))),
                            number_to_string(long as f64),
                            number_to_string(widths.len() as f64),
                            number_to_string(line_max)
                        ),
                    ));
                }
            }
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
        // The space the reader sees, not the space the stylesheet declares:
        // the inset between the rendered text and the inside of the border
        // box. A 44px control with `padding: 0 16px` whose label a flex box
        // centres has 12px of air above the label and was charged with "0px
        // vertical padding" (REN-403). Nothing to measure the text with is
        // nothing to charge on, and the text is measured only for the
        // elements that got this far: the probe builds a Range per call, and
        // a page has a great many boxes that are not bounded at all.
        if let Some(insets) = (border_count >= 2 || has_bg)
            .then(|| direct_text_insets(dom, el, rect, &borders))
            .flatten()
        {
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
            // The measured inset decides and is what the finding reports. The
            // declared padding has to be short as well: the text rect is a
            // font-metric box, and a line box tighter than the face's ascent
            // and descent pushes it toward the edge of a box whose padding is
            // generous.
            let inset_of = |s: usize| insets[s];
            let v_inset = min_over(&v_sides, &inset_of);
            let h_inset = min_over(&h_sides, &inset_of);
            let v_cramped = v_min < v_thresh && v_inset < v_thresh;
            let h_cramped = h_min < h_thresh && h_inset < h_thresh;
            let px = |v: f64| number_to_string(math_round(v * 10.0) / 10.0);
            if v_cramped {
                findings.push(RuleHit::new(
                    "cramped-padding",
                    format!(
                        "{}px of space above and below the text (need ≥{}px for {}px text)",
                        px(v_inset),
                        to_fixed(v_thresh, 1),
                        number_to_string(font_size)
                    ),
                ));
            } else if h_cramped {
                findings.push(RuleHit::new(
                    "cramped-padding",
                    format!(
                        "{}px of space beside the text (need ≥{}px for {}px text)",
                        px(h_inset),
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
                // Content that runs past the box is clipped, not snug. A
                // table with a min-width inside an `overflow: hidden` frame
                // has its far column cut off, which is a defect
                // `clipped-overflow-container` is named for; calling it "no
                // inset" points at the wrong thing (REN-403).
                const OVERFLOW_TOLERANCE: f64 = 1.0;
                let mut children_overflow = [false; 4];
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
                        if rect.top - cr.top > OVERFLOW_TOLERANCE {
                            children_overflow[0] = true;
                        }
                        if cr.right - rect.right > OVERFLOW_TOLERANCE {
                            children_overflow[1] = true;
                        }
                        if cr.bottom - rect.bottom > OVERFLOW_TOLERANCE {
                            children_overflow[2] = true;
                        }
                        if rect.left - cr.left > OVERFLOW_TOLERANCE {
                            children_overflow[3] = true;
                        }
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
                    if side_bounded
                        && pad[s] <= PAD_THRESHOLD
                        && !children_insulate[s]
                        && !children_overflow[s]
                        && text_flush[s]
                    {
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
                if wraps
                    && !is_non_rendered_text(dom, el, tag)
                    && !is_visually_hidden(dom, el)
                    && !is_heading_text(dom, el)
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
            && font_size < 11.0
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
                let floor = if !is_interactive && is_smallprint { 10.0 } else { 11.0 };
                if font_size < floor && (is_interactive || is_furniture || dt_len <= 20) {
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
        let long = "x".repeat(240);
        let p = text_el(&mut d, body, "p", &long, "16px");
        d.set_rect(p, 0.0, 100.0, 1200.0, 72.0);
        // Three rendered lines: two full ones and a tail.
        d.set_text_lines(p, &[(0.0, 100.0, 1180.0, 19.0), (0.0, 124.0, 1180.0, 19.0), (0.0, 148.0, 400.0, 19.0)]);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"line-length"), "{ids:?}");
        assert_eq!(hits[0].snippet, "~103 chars on 2 of 3 rendered lines (aim for <80)");
        assert!(ids.contains(&"body-text-viewport-edge"));
        let edge = hits.iter().find(|h| h.id == "body-text-viewport-edge").unwrap();
        assert_eq!(edge.snippet, "<p> with 240-char body bleeds to viewport edge (left 0px)");
        // narrower, inset paragraph: neither fires
        d.set_rect(p, 40.0, 100.0, 600.0, 72.0);
        d.set_text_lines(p, &[(40.0, 100.0, 580.0, 19.0), (40.0, 124.0, 580.0, 19.0), (40.0, 148.0, 580.0, 19.0)]);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert!(hits.is_empty(), "{hits:?}");
    }

    /// REN-402. Halfday's pricing copy: a 158-character paragraph in a 1022px
    /// card body, rendering 145 characters on its first line and 13 on its
    /// second. The old measurement charged the box (`1022 / (15 * 0.5)` = 136
    /// "chars/line") on every paragraph that shape, including the ones whose
    /// text stops well short of the box.
    #[test]
    fn line_length_reads_the_rendered_line_not_the_box() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let copy = "People are counted on the first of the month. Someone invited on the 3rd is free until the 1st, and someone removed mid-month is credited on the next invoice.";
        assert_eq!(copy.chars().count(), 158);
        let p = text_el(&mut d, body, "p", copy, "15px");
        d.set_style(p, "lineHeight", "24px");
        d.set_rect(p, 200.0, 971.0, 1022.0, 48.0);
        // One long line and a 13-character tail: the eye tracks back once.
        d.set_text_lines(p, &[(200.0, 974.0, 995.4, 18.0), (200.0, 998.0, 85.5, 18.0)]);
        assert_eq!(check_element_quality_dom(&d, p, &BrowserConfig::default()), vec![]);

        // The same box, text that stops at 571px: 89 characters on one line.
        let meta = text_el(&mut d, body, "p", &"y".repeat(89), "14px");
        d.set_style(meta, "lineHeight", "21.7px");
        d.set_rect(meta, 200.0, 122.0, 992.0, 21.7);
        d.set_text_lines(meta, &[(200.0, 124.2, 571.4, 17.0)]);
        assert_eq!(check_element_quality_dom(&d, meta, &BrowserConfig::default()), vec![]);

        // A column of long lines is the defect the rule is named for.
        let wall = text_el(&mut d, body, "p", &"z".repeat(500), "15px");
        d.set_style(wall, "lineHeight", "24px");
        d.set_rect(wall, 200.0, 100.0, 1022.0, 96.0);
        d.set_text_lines(
            wall,
            &[
                (200.0, 100.0, 1000.0, 18.0),
                (200.0, 124.0, 1000.0, 18.0),
                (200.0, 148.0, 1000.0, 18.0),
                (200.0, 172.0, 600.0, 18.0),
            ],
        );
        let hits = check_element_quality_dom(&d, wall, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].snippet, "~139 chars on 3 of 4 rendered lines (aim for <80)");
    }

    /// A line box split across text nodes is still one line. An inline
    /// `<strong>` or `<a>` in the middle of a sentence is its own text node,
    /// so `getClientRects()` hands back a rect per fragment; counting each
    /// fragment as a line divided the paragraph's characters among them and
    /// hid a genuinely long column.
    #[test]
    fn a_line_split_across_fragments_is_one_line() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        // 300 characters over three rendered lines, each interrupted mid-line
        // by an inline element and so measured in two pieces.
        let p = text_el(&mut d, body, "p", &"w".repeat(300), "16px");
        d.set_rect(p, 0.0, 100.0, 1020.0, 72.0);
        d.set_text_lines(
            p,
            &[
                (0.0, 100.0, 520.0, 19.0),
                (520.0, 100.0, 480.0, 19.0),
                (0.0, 124.0, 510.0, 19.0),
                (510.0, 124.0, 490.0, 19.0),
                (0.0, 148.0, 505.0, 19.0),
                (505.0, 148.0, 495.0, 19.0),
            ],
        );
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        let line = hits.iter().find(|h| h.id == "line-length").expect("charged");
        // Three lines of 1000px, not six of ~500: six would have put 50
        // characters on each and charged nothing at all.
        assert_eq!(line.snippet, "~100 chars on 3 of 3 rendered lines (aim for <80)");

        // The same merge the other way: one long line in two fragments plus a
        // short tail is two lines, and one long line is a sentence that
        // wrapped once.
        let q = text_el(&mut d, body, "p", &"w".repeat(190), "16px");
        d.set_rect(q, 0.0, 300.0, 1020.0, 48.0);
        d.set_text_lines(
            q,
            &[
                (0.0, 300.0, 600.0, 19.0),
                (600.0, 300.0, 400.0, 19.0),
                (0.0, 324.0, 120.0, 19.0),
            ],
        );
        let hits = check_element_quality_dom(&d, q, &BrowserConfig::default());
        assert!(!hits.iter().any(|h| h.id == "line-length"), "{hits:?}");
    }

    /// Two columns that happen to sit on the same rows are two flows, not one
    /// page-wide line. Fragments join a row only when they run on from it —
    /// a gap no wider than the row's own line box — so a gutter keeps them
    /// apart and the characters stay where the reader sees them.
    #[test]
    fn columns_on_the_same_rows_are_separate_lines() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &"w".repeat(240), "16px");
        d.set_rect(p, 0.0, 100.0, 1020.0, 72.0);
        // Two 300px columns with a 100px gutter, three rows each.
        d.set_text_lines(
            p,
            &[
                (0.0, 100.0, 300.0, 19.0),
                (400.0, 100.0, 300.0, 19.0),
                (0.0, 124.0, 300.0, 19.0),
                (400.0, 124.0, 300.0, 19.0),
                (0.0, 148.0, 300.0, 19.0),
                (400.0, 148.0, 300.0, 19.0),
            ],
        );
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        // Six short lines of 40 characters each, not three of 700px.
        assert!(!hits.iter().any(|h| h.id == "line-length"), "{hits:?}");
    }

    /// A rect that is already one line is one line, whatever the leading is.
    /// Dividing every rect by the line box turned a paragraph whose leading
    /// is tighter than its glyph box into two copies of the same line, and
    /// two copies of one long line satisfied "at least two long lines".
    #[test]
    fn tight_leading_does_not_double_count_a_line() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &"w".repeat(200), "15px");
        // 10px of leading under an 19px glyph box: `round(19 / 10)` is 2.
        d.set_style(p, "lineHeight", "10px");
        d.set_rect(p, 0.0, 100.0, 1020.0, 19.0);
        d.set_text_lines(p, &[(0.0, 100.0, 1000.0, 19.0)]);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert!(!hits.iter().any(|h| h.id == "line-length"), "{hits:?}");
    }

    /// A DOM that kept only the union of its text rects cannot say where the
    /// lines are, and the rule stands down rather than inventing them. A
    /// snapshot captured before the lines were recorded is that DOM: the
    /// union of a long first line and a short tail is the same union as two
    /// even lines, so any width read off it is a width nothing rendered.
    #[test]
    fn a_dom_without_lines_does_not_charge_line_length() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &"w".repeat(300), "16px");
        d.set_rect(p, 0.0, 100.0, 1020.0, 72.0);
        // The same paragraph the merge test charges, measured once.
        d.set_text_rect(p, 0.0, 100.0, 1000.0, 67.0);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert!(!hits.iter().any(|h| h.id == "line-length"), "{hits:?}");
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
        // The text lands 2px under the top edge, which is what the reader sees
        // and what the declared padding happens to say here.
        d.set_text_lines(p, &[(52.0, 102.0, 276.0, 19.0)]);
        let hits = check_element_quality_dom(&d, p, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "2px of space above and below the text (need ≥4.8px for 16px text)"
        );
    }

    /// REN-403. Halfday's plan button: 44px tall because the design system
    /// says every control is, `padding: 0 16px`, and the label optically
    /// centred by a flex box. The declared vertical padding is zero and the
    /// space above the label is 12px, which is what the reader sees.
    #[test]
    fn cramped_padding_measures_the_space_around_the_text() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let btn = text_el(&mut d, body, "a", "Talk to us about Studio", "15px");
        d.set_rect(btn, 0.0, 778.7, 296.7, 44.0);
        d.set_styles(
            btn,
            &[
                ("backgroundColor", "rgb(255, 255, 255)"),
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgb(220, 220, 220)"),
                ("borderRightColor", "rgb(220, 220, 220)"),
                ("borderBottomColor", "rgb(220, 220, 220)"),
                ("borderLeftColor", "rgb(220, 220, 220)"),
                ("paddingTop", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "16px"),
                ("paddingRight", "16px"),
                ("display", "flex"),
            ],
        );
        d.set_text_lines(btn, &[(68.0, 791.2, 160.0, 18.0)]);
        assert_eq!(check_element_quality_dom(&d, btn, &BrowserConfig::default()), vec![]);

        // The same control with the label actually against the edge: charged.
        d.set_text_lines(btn, &[(68.0, 780.2, 160.0, 18.0)]);
        let hits = check_element_quality_dom(&d, btn, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "0.5px of space above and below the text (need ≥4.5px for 15px text)"
        );
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
        assert_eq!(hits[0].snippet, "1px of space above and below the text (need ≥4.2px for 14px text)");
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

    /// REN-403, the wrapper half of the same rule. Crewline's framed table:
    /// the `<table>` fills the frame edge to edge, and every cell insets its
    /// own text by the padding the stylesheet gives it. Reading the cell's
    /// border box called all four sides flush.
    #[test]
    fn flush_reads_the_text_not_the_cell_that_holds_it() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_style(body, "backgroundColor", "rgb(255, 255, 255)");
        let frame = d.add(Some(body), "div");
        d.set_attr(frame, "class", "table-frame");
        d.set_rect(frame, 0.0, 0.0, 860.0, 300.0);
        d.set_styles(
            frame,
            &[
                ("position", "static"),
                ("borderTopWidth", "1px"),
                ("borderRightWidth", "1px"),
                ("borderBottomWidth", "1px"),
                ("borderLeftWidth", "1px"),
                ("borderTopColor", "rgb(220, 220, 220)"),
                ("borderRightColor", "rgb(220, 220, 220)"),
                ("borderBottomColor", "rgb(220, 220, 220)"),
                ("borderLeftColor", "rgb(220, 220, 220)"),
                ("outlineWidth", "0px"),
                ("backgroundColor", "rgb(250, 250, 250)"),
                ("paddingTop", "0px"),
                ("paddingRight", "0px"),
                ("paddingBottom", "0px"),
                ("paddingLeft", "0px"),
                ("fontSize", "15px"),
            ],
        );
        let table = d.add(Some(frame), "table");
        d.set_rect(table, 0.0, 0.0, 860.0, 300.0);
        for (i, (x, y, w)) in [(0.0, 0.0, 430.0), (430.0, 0.0, 430.0), (0.0, 260.0, 430.0)]
            .into_iter()
            .enumerate()
        {
            let cell = d.add(Some(table), "td");
            d.add_text(cell, "Wednesday afternoon");
            d.set_rect(cell, x, y, w, 20.0);
            // Cell padding: 10px down, 16px across, which is where the text is.
            d.set_text_lines(cell, &[(x + 16.0, y + 10.0, w - 32.0, 17.0)]);
            let _ = i;
        }
        assert_eq!(check_element_quality_dom(&d, frame, &BrowserConfig::default()), vec![]);

        // A cell that really does put its text on the frame line is charged.
        let tight = d.add(Some(table), "td");
        d.add_text(tight, "Wednesday afternoon");
        d.set_rect(tight, 0.0, 140.0, 860.0, 20.0);
        d.set_text_lines(tight, &[(1.0, 140.0, 858.0, 17.0)]);
        let hits = check_element_quality_dom(&d, frame, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<div> \"table-frame\": children flush against border+bg on right/left (no inset)"
        );

        // The same frame at 390px, where the table keeps its min-width and the
        // frame hides what does not fit: the right side is clipped, not snug,
        // and that is `clipped-overflow-container`'s business (REN-403).
        d.set_rect(table, 0.0, 0.0, 1400.0, 300.0);
        let hits = check_element_quality_dom(&d, frame, &BrowserConfig::default());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            hits[0].snippet,
            "<div> \"table-frame\": children flush against border+bg on left (no inset)"
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
        // And so is an inline run inside that paragraph.
        let nested_p = d.add(Some(h3), "p");
        let nested_run = wrapped(&mut d, nested_p, "span", "16px", 17.6);
        assert_eq!(
            leading(&d, nested_run),
            vec!["line-height 1.10x (need >=1.3)"],
            "span in a paragraph nested in a heading"
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

        // Two lines that fill their 1,200px box.
        d.set_text_lines(p, &[(40.0, 102.0, 1180.0, 20.0), (40.0, 126.0, 1180.0, 20.0)]);
        assert_eq!(
            snippets(&d, p, "line-length"),
            vec!["~100 chars on 2 of 2 rendered lines (aim for <80)"]
        );
        // One line in the same box sends the eye nowhere.
        d.set_text_lines(p, &[(40.0, 102.0, 1180.0, 20.0)]);
        assert!(snippets(&d, p, "line-length").is_empty(), "one line");
        // Centred lines well short of the box: 119 characters on two lines
        // is ~60 a line.
        let centred = text_el(&mut d, body, "p", &"word ".repeat(24), "16px");
        d.set_style(centred, "lineHeight", "24px");
        d.set_rect(centred, 40.0, 200.0, 1200.0, 48.0);
        d.set_text_lines(centred, &[(340.0, 202.0, 600.0, 20.0), (340.0, 226.0, 600.0, 20.0)]);
        assert!(snippets(&d, centred, "line-length").is_empty(), "centred block");
        // A union with no lines behind it is not read.
        d.el_mut(p).text_line_rects = None;
        d.set_text_rect(p, 40.0, 102.0, 1180.0, 44.0);
        assert!(snippets(&d, p, "line-length").is_empty(), "no lines recorded");

        // simplybudget.framer.ai: 94 characters on two lines ended by a
        // `<br>`. Each line is as long as the author left it.
        let broken = text_el(&mut d, body, "p", &"word ".repeat(19), "16px");
        d.set_style(broken, "lineHeight", "28.8px");
        d.set_rect(broken, 260.0, 900.0, 760.0, 57.6);
        d.set_text_lines(broken, &[(260.0, 904.0, 660.0, 20.0), (260.0, 933.0, 80.0, 20.0)]);
        assert!(snippets(&d, broken, "line-length").is_empty(), "one long line and a tail");
    }

    /// observations-20 row 29: a full-width CJK glyph is an em wide, so the
    /// half-em estimate doubled so-net.ne.jp's count.
    #[test]
    fn line_length_counts_cjk_glyphs_at_an_em() {
        let copy = "戸建/マンションは、NTTから送付される「開通のご案内」に記載の「ご利用サービス名」など、回線事業者からの案内をご確認のうえタイプに合ったコースをお選びください。".repeat(2);
        let len = utf16_len(&copy);
        assert!(len > 80);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = text_el(&mut d, body, "p", &copy, "16px");
        d.set_style(p, "lineHeight", "24px");
        d.set_rect(p, 110.0, 100.0, 1060.0, 72.0);
        d.set_text_lines(
            p,
            &[(110.0, 104.0, 1048.0, 16.0), (110.0, 128.0, 1048.0, 16.0), (110.0, 152.0, 1048.0, 16.0)],
        );
        assert!(snippets(&d, p, "line-length").is_empty(), "a third of the glyphs a line");
        // A wider CJK column still reports, at its own count.
        let wide = text_el(&mut d, body, "p", &copy.repeat(2), "16px");
        d.set_rect(wide, 0.0, 300.0, 1600.0, 72.0);
        d.set_text_lines(
            wide,
            &[(0.0, 304.0, 1590.0, 16.0), (0.0, 328.0, 1590.0, 16.0), (0.0, 352.0, 1590.0, 16.0)],
        );
        let hits = snippets(&d, wide, "line-length");
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
        // The paragraph's lines are the lines of the text under it.
        d.set_text_lines(intro, &[(0.0, 102.0, 1270.0, 20.0), (0.0, 126.0, 1270.0, 20.0), (0.0, 150.0, 300.0, 20.0)]);
        d.el_mut(intro).direct_text_rect = None;
        let len = utf16_len(js::trim(&d.text_content(intro)));
        assert_eq!(
            snippets(&d, intro, "line-length"),
            vec![format!(
                "~{} chars on 2 of 3 rendered lines (aim for <80)",
                ((len as f64) * 1270.0 / 2840.0).round()
            )]
        );
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
        d.el_mut(intro).text_line_rects = None;
        d.el_mut(b).direct_text_rect = None;
        d.el_mut(i).direct_text_rect = None;
        assert!(snippets(&d, intro, "line-length").is_empty());
        assert!(snippets(&d, intro, "body-text-viewport-edge").is_empty());
        // A paragraph holding a block component is not inline prose.
        d.set_text_rect(i, 0.0, 102.0, 1270.0, 68.0);
        d.set_text_lines(intro, &[(0.0, 102.0, 1270.0, 20.0), (0.0, 126.0, 1270.0, 20.0), (0.0, 150.0, 300.0, 20.0)]);
        d.el_mut(intro).direct_text_rect = None;
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
            vec!["0px of space above and below the text (need ≥4.2px for 14px text)"]
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

#[cfg(test)]
mod rendered_text_tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    /// A paragraph over two rendered lines of 640px and 260px, as
    /// directus.io's hero paragraph rendered.
    fn two_line_p(d: &mut FakeDom, body: ElId) -> ElId {
        let p = d.add(Some(body), "p");
        d.set_styles(
            p,
            &[
                ("fontSize", "18px"),
                ("lineHeight", "normal"),
                ("letterSpacing", "normal"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("position", "static"),
                ("textTransform", "none"),
                ("textAlign", "start"),
                ("whiteSpace", "normal"),
            ],
        );
        d.set_rect(p, 100.0, 270.0, 700.0, 56.0);
        d.set_text_lines(p, &[(100.0, 272.0, 640.0, 24.0), (100.0, 300.0, 260.0, 24.0)]);
        p
    }

    fn line_length(d: &FakeDom, el: ElId) -> Option<String> {
        check_element_quality_dom(d, el, &BrowserConfig::default())
            .into_iter()
            .find(|h| h.id == "line-length")
            .map(|h| h.snippet)
    }

    const SENTENCE: &str = "The collaborative backend that turns any database into an API your whole team can use today.";

    /// An inline `<style>` child is in `textContent` and on no line. Its 240
    /// characters were shared out between the two rendered lines and turned
    /// a 66-character line into a 240-character one.
    #[test]
    fn an_inline_style_child_is_not_counted() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        let style = d.add(Some(p), "style");
        d.set_style(style, "display", "none");
        d.add_text(style, &"@keyframes blink { 0%, 50% { opacity: 1; } } ".repeat(6));
        d.add_text(p, SENTENCE);
        assert_eq!(rendered_text_len(&d, p), SENTENCE.len());
        assert_eq!(line_length(&d, p), None);

        // The same lines with that many characters of prose do report.
        let q = two_line_p(&mut d, body);
        d.add_text(q, &"w".repeat(SENTENCE.len() + 270));
        assert_eq!(
            line_length(&d, q).as_deref(),
            Some("~257 chars on 2 of 2 rendered lines (aim for <80)")
        );
    }

    /// A `display: none` child and a `<script>` are cut the same way, and a
    /// rendered inline child stays in.
    #[test]
    fn hidden_and_script_children_are_cut_and_inline_children_kept() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.add_text(p, "Before ");
        let hidden = d.add(Some(p), "span");
        d.set_style(hidden, "display", "none");
        d.add_text(hidden, &"hidden ".repeat(40));
        let strong = d.add(Some(p), "strong");
        d.set_style(strong, "display", "inline");
        d.add_text(strong, "kept");
        let script = d.add(Some(p), "script");
        d.add_text(script, &"var x = 1; ".repeat(30));
        d.add_text(p, " after");
        assert_eq!(rendered_text_len(&d, p), "Before kept after".len());
    }

    /// Source indentation collapses to one space under `white-space: normal`
    /// and is kept under `pre-wrap`.
    #[test]
    fn source_indentation_collapses_unless_preserved() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.add_text(p, "\n        Copyright 2017, all rights reserved\n                with the Directorate.\n    ");
        assert_eq!(
            rendered_text_len(&d, p),
            "Copyright 2017, all rights reserved with the Directorate.".len()
        );
        d.set_style(p, "whiteSpace", "pre-wrap");
        assert_eq!(
            rendered_text_len(&d, p),
            "Copyright 2017, all rights reserved\n                with the Directorate.".len()
        );
    }

    /// A combining mark sits on its base and a zero-width joiner or soft
    /// hyphen draws nothing, so none of them is a character on the line.
    #[test]
    fn combining_marks_and_format_characters_are_not_counted() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        // "राजस्थान के": र ा ज स ् थ ा न, a space, क े.
        d.add_text(p, "\u{930}\u{93e}\u{91c}\u{938}\u{94d}\u{925}\u{93e}\u{928} \u{915}\u{947}");
        assert_eq!(rendered_text_len(&d, p), 7);
        let q = two_line_p(&mut d, body);
        d.add_text(q, "cafe\u{301} co\u{ad}operate\u{200b}s");
        assert_eq!(rendered_text_len(&d, q), "cafe cooperates".len());
    }

    /// Plain prose counts as before, so a long column still reports the same
    /// number.
    #[test]
    fn plain_prose_counts_every_character() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.set_text_lines(p, &[(100.0, 272.0, 640.0, 24.0), (100.0, 300.0, 640.0, 24.0)]);
        d.add_text(p, &"word ".repeat(40));
        assert_eq!(rendered_text_len(&d, p), 199);
        assert_eq!(
            line_length(&d, p).as_deref(),
            Some("~100 chars on 2 of 2 rendered lines (aim for <80)")
        );
    }

    /// Each text node keeps or folds its white space by the `white-space` it
    /// inherits, not the paragraph's. A `pre-wrap` span inside a normal
    /// paragraph keeps its runs of spaces on the line, so it counts the same
    /// as a `pre-wrap` paragraph and flags the same long lines.
    #[test]
    fn a_preserving_span_inside_a_normal_paragraph_keeps_its_spaces() {
        let text = "word    ".repeat(22);
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.set_text_lines(p, &[(100.0, 272.0, 640.0, 24.0), (100.0, 300.0, 640.0, 24.0)]);
        d.add_text(p, "An opening line ");
        let span = d.add(Some(p), "span");
        d.set_style(span, "display", "inline");
        d.set_style(span, "whiteSpace", "pre-wrap");
        d.add_text(span, &text);
        assert_eq!(rendered_text_len(&d, p), "An opening line ".len() + 22 * 8 - 4);

        let whole = two_line_p(&mut d, body);
        d.set_text_lines(whole, &[(100.0, 272.0, 640.0, 24.0), (100.0, 300.0, 640.0, 24.0)]);
        d.set_style(whole, "whiteSpace", "pre-wrap");
        d.add_text(whole, &format!("An opening line {text}"));
        assert_eq!(rendered_text_len(&d, whole), rendered_text_len(&d, p));
        assert_eq!(line_length(&d, p), line_length(&d, whole));
        assert_eq!(
            line_length(&d, p).as_deref(),
            Some("~94 chars on 2 of 2 rendered lines (aim for <80)")
        );

        // The reverse: a normal span inside a pre-wrap paragraph folds its
        // own runs, and a collapsible run carries on across the boundary of
        // an inline child the way `a <b> b</b>` renders one space.
        let q = two_line_p(&mut d, body);
        d.set_style(q, "whiteSpace", "pre-wrap");
        d.add_text(q, "a  b");
        let normal = d.add(Some(q), "span");
        d.set_style(normal, "display", "inline");
        d.set_style(normal, "whiteSpace", "normal");
        d.add_text(normal, "   c   d");
        assert_eq!(rendered_text_len(&d, q), "a  b c d".len());
        let r = two_line_p(&mut d, body);
        d.add_text(r, "a ");
        let b = d.add(Some(r), "b");
        d.set_style(b, "display", "inline");
        d.add_text(b, " b");
        assert_eq!(rendered_text_len(&d, r), "a b".len());
    }

    /// The text of a hidden child is skipped where it sits, so a hidden run
    /// of white space never takes a visible space with it.
    #[test]
    fn a_hidden_child_is_skipped_where_it_sits() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.add_text(p, "one two");
        let hidden = d.add(Some(p), "span");
        d.set_style(hidden, "display", "none");
        d.add_text(hidden, " ");
        assert_eq!(rendered_text_len(&d, p), "one two".len());

        let q = two_line_p(&mut d, body);
        d.add_text(q, "one two three four");
        for _ in 0..3 {
            let hidden = d.add(Some(q), "span");
            d.set_style(hidden, "display", "none");
            d.add_text(hidden, " ");
            d.add_text(q, "x");
        }
        assert_eq!(rendered_text_len(&d, q), "one two three fourxxx".len());
    }

    /// A `content-visibility: hidden` child lays out its box and renders none
    /// of its contents, so its text is on no line. A DOM that cannot say
    /// (no `contentVisibility` value) counts it.
    #[test]
    fn a_content_visibility_hidden_child_is_skipped() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = two_line_p(&mut d, body);
        d.add_text(p, "Before ");
        let skipped = d.add(Some(p), "span");
        d.set_style(skipped, "display", "inline-block");
        d.set_style(skipped, "contentVisibility", "hidden");
        d.add_text(skipped, &"unrendered ".repeat(30));
        d.add_text(p, "after");
        assert_eq!(rendered_text_len(&d, p), "Before after".len());

        let q = two_line_p(&mut d, body);
        d.add_text(q, "Before ");
        let unknown = d.add(Some(q), "span");
        d.set_style(unknown, "display", "inline");
        d.add_text(unknown, "kept ");
        d.add_text(q, "after");
        assert_eq!(rendered_text_len(&d, q), "Before kept after".len());

        // content-visibility does not apply to a plain inline box or to
        // display: contents, so their text renders and counts.
        for display in ["inline", "contents"] {
            let r = two_line_p(&mut d, body);
            d.add_text(r, "Before ");
            let inline = d.add(Some(r), "span");
            d.set_style(inline, "display", display);
            d.set_style(inline, "contentVisibility", "hidden");
            d.add_text(inline, "shown ");
            d.add_text(r, "after");
            assert_eq!(rendered_text_len(&d, r), "Before shown after".len(), "{display}");
        }
    }

    /// An image or an inline-block is an atomic inline: the space on each
    /// side of it renders, so the pair does not fold into one, and a
    /// populated box's own edge white space is trimmed inside it.
    #[test]
    fn spaces_around_an_atomic_inline_both_count() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        for (tag, display) in [("img", "inline"), ("span", "inline-block")] {
            let r = two_line_p(&mut d, body);
            d.add_text(r, "word ");
            let boxed = d.add(Some(r), tag);
            d.set_style(boxed, "display", display);
            d.add_text(r, " word");
            assert_eq!(rendered_text_len(&d, r), "word  word".len(), "{tag}");
        }

        // A populated box trims its own edge white space in its own
        // formatting context: none of it joins the spaces outside.
        for (tag, display) in [("span", "inline-block"), ("button", "inline-block")] {
            let r = two_line_p(&mut d, body);
            d.add_text(r, "word ");
            let boxed = d.add(Some(r), tag);
            d.set_style(boxed, "display", display);
            d.add_text(boxed, "\n      word\n    ");
            d.add_text(r, " word");
            assert_eq!(rendered_text_len(&d, r), "word word word".len(), "{tag}");
        }

        // Edge spaces that cannot collapse render inside the box: preserved
        // ones under white-space: pre, and no-break spaces.
        let r = two_line_p(&mut d, body);
        d.add_text(r, "word ");
        let pre = d.add(Some(r), "span");
        d.set_style(pre, "display", "inline-block");
        d.set_style(pre, "whiteSpace", "pre");
        d.add_text(pre, " word ");
        d.add_text(r, " word");
        assert_eq!(rendered_text_len(&d, r), "word  word  word".len());

        let r = two_line_p(&mut d, body);
        d.add_text(r, "word ");
        let nbsp = d.add(Some(r), "span");
        d.set_style(nbsp, "display", "inline-block");
        d.add_text(nbsp, "\u{a0}word\u{a0}");
        d.add_text(r, " word");
        assert_eq!(rendered_text_len(&d, r), "word _word_ word".len());
    }
}
