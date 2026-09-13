//! Whether an element is painted at capture time.
//!
//! The element pass measures text and images from their computed style and
//! their box. A box can carry a real measurement and still show nothing: a
//! mobile submenu held at zero width, the cells of a horizontal scroller past
//! its visible edge, an animated demo step that has not played, a poster at
//! opacity 0 over a playing video. A reader cannot see any of those at rest,
//! so a rule that scores what a reader sees has nothing to score.
//!
//! [`unpainted_at_capture`] is the one predicate every such rule shares, and
//! the driver applies it to the rules [`paint_gate`] names. It reads only what
//! the element pass already reads (style, rects, `checkVisibility`) and walks
//! the ancestor chain once, so its cost is bounded by the depth of the element,
//! and the driver asks it only for elements that produced a gated finding.
//!
//! `content-hidden-at-rest` is deliberately not gated: hidden text is what it
//! reports.

use super::dom::{Dom, ElId, Rect};
use super::element_checks::effective_opacity_dom;
use super::BrowserFinding;
use crate::js;

/// Why an element is not painted at capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unpainted {
    /// `display: none` or `content-visibility: hidden` on an ancestor, or
    /// `visibility: hidden` / `checkVisibility()` false on the element.
    NotRendered,
    /// The element's effective opacity is at or near 0.
    Transparent,
    /// A near-transparent raster that declares an opacity transition or
    /// animation: a crossfade, slideshow or lazy-load layer between states.
    StateLayer,
    /// An ancestor that clips its overflow has no area on a clipped axis, or
    /// does not overlap the element on it.
    ClippedOut,
    /// The box lies wholly outside the scrollable document (or, inside a
    /// fixed layer, wholly outside the viewport).
    OutsideDocument,
}

/// How the element's own opacity takes part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnOpacity {
    /// The element's own opacity decides whether it is seen (text rules).
    Counts,
    /// The rule measures the element's own opacity (`buried-raster`), so only
    /// ancestors count toward transparency, and a near-transparent element
    /// that declares an opacity transition or animation is a state layer.
    Measured,
}

/// The text measurements that need painted text. Style tells (gradient text,
/// palette, fonts, borders) describe authored CSS whatever state is showing
/// and are not gated.
pub const PAINT_GATED_TEXT_RULES: &[&str] = &[
    "all-caps-body",
    "body-text-viewport-edge",
    "cramped-padding",
    "extreme-negative-tracking",
    "gray-on-color",
    "justified-text",
    "line-length",
    "low-contrast",
    "text-overflow",
    "tight-leading",
    "tiny-text",
    "undersized-ui-text",
    "wide-tracking",
];

/// The opacity at or below which an element reads as not painted, the same
/// floor [`effective_opacity_dom`] collapses to zero.
const TRANSPARENT_FLOOR: f64 = 0.02;

/// The own-opacity ceiling of `buried-raster`'s opacity form.
const STATE_LAYER_OPACITY: f64 = 0.15;

/// Which gate a rule's findings pass through, or `None` for an ungated rule.
pub fn paint_gate(rule_id: &str) -> Option<OwnOpacity> {
    if rule_id == "buried-raster" {
        Some(OwnOpacity::Measured)
    } else if PAINT_GATED_TEXT_RULES.contains(&rule_id) {
        Some(OwnOpacity::Counts)
    } else {
        None
    }
}

/// Drop the findings on `el` whose rule needs a painted element when `el` is
/// not painted. Each gate is evaluated at most once per element, and not at
/// all when no finding needs it.
pub fn retain_painted(dom: &dyn Dom, el: ElId, findings: &mut Vec<BrowserFinding>) {
    let mut counts: Option<bool> = None;
    let mut measured: Option<bool> = None;
    findings.retain(|f| match paint_gate(&f.type_) {
        None => true,
        Some(OwnOpacity::Counts) => {
            *counts.get_or_insert_with(|| unpainted_at_capture(dom, el, OwnOpacity::Counts).is_none())
        }
        Some(OwnOpacity::Measured) => *measured
            .get_or_insert_with(|| unpainted_at_capture(dom, el, OwnOpacity::Measured).is_none()),
    });
}

/// Whether a visitor sees `el` painted at rest, counting its own opacity.
pub fn painted_at_capture(dom: &dyn Dom, el: ElId) -> bool {
    unpainted_at_capture(dom, el, OwnOpacity::Counts).is_none()
}

/// Why `el` is not painted at capture, or `None` when it is (or when the Dom
/// cannot measure it, which keeps the finding).
pub fn unpainted_at_capture(dom: &dyn Dom, el: ElId, own: OwnOpacity) -> Option<Unpainted> {
    if Some(el) == dom.body() || Some(el) == dom.document_element() {
        return None;
    }
    if dom.check_visibility(el) == Some(false) {
        return Some(Unpainted::NotRendered);
    }
    // `visibility` inherits, so the element's computed value covers its
    // ancestors; `display: none` and `content-visibility: hidden` do not, and
    // the walk below reads them.
    let visibility = js::to_lower_case(&dom.style(el, "visibility"));
    if visibility == "hidden" || visibility == "collapse" || dom.style(el, "display") == "none" {
        return Some(Unpainted::NotRendered);
    }

    match own {
        OwnOpacity::Counts => {
            if effective_opacity_dom(dom, el) <= TRANSPARENT_FLOOR {
                return Some(Unpainted::Transparent);
            }
        }
        OwnOpacity::Measured => {
            if let Some(p) = dom.parent(el) {
                if effective_opacity_dom(dom, p) <= TRANSPARENT_FLOOR {
                    return Some(Unpainted::Transparent);
                }
            }
            let op = js::parse_float(&dom.style(el, "opacity"));
            if op.is_finite()
                && op < STATE_LAYER_OPACITY
                && (declares_opacity_transition(dom, el) || declares_opacity_animation(dom, el))
            {
                return Some(Unpainted::StateLayer);
            }
        }
    }

    let rect = dom.rect(el);
    if !rect.all_finite() {
        return None;
    }
    let viewport_w = finite_or(dom.inner_width(), 0.0);
    let viewport_h = finite_or(dom.inner_height(), 0.0);

    let body = dom.body();
    let root = dom.document_element();
    let mut placement = Placement::of(dom, el);
    // The outermost fixed box on the containing chain, once one is found.
    let mut fixed_rect = if placement == Placement::Fixed { Some(rect) } else { None };
    let mut cur = dom.parent(el);
    while let Some(p) = cur {
        let display = dom.style(p, "display");
        if display == "none" || js::to_lower_case(&dom.style(p, "contentVisibility")) == "hidden" {
            return Some(Unpainted::NotRendered);
        }
        if placement.clipped_by(dom, p) {
            // The page's own overflow propagates to the viewport, whose
            // scrolling is what brings content into view; the document test
            // below covers what it can never reach.
            let is_page = Some(p) == body || Some(p) == root;
            if !is_page && clips_contents(&display) {
                if let Some(reason) = clip_outcome(dom, p, &rect, viewport_w, viewport_h) {
                    return Some(reason);
                }
            }
            placement = Placement::of(dom, p);
            if placement == Placement::Fixed {
                fixed_rect = Some(dom.rect(p));
            }
        }
        cur = dom.parent(p);
    }

    if placement == Placement::Fixed {
        // A fixed layer no transformed ancestor contains is viewport-relative
        // and never scrolled to: when the layer's own box lies outside the
        // viewport (a parked drawer), nothing in it is painted. Content that
        // runs past the edge of a layer on screen is left to the clip tests,
        // which is how a smooth-scroll viewport keeps its page.
        if let Some(fr) = fixed_rect.filter(Rect::all_finite) {
            if viewport_w > 0.0 && viewport_h > 0.0 && misses(&fr, 0.0, viewport_w, 0.0, viewport_h) {
                return Some(Unpainted::OutsideDocument);
            }
        }
        return None;
    }
    outside_document(dom, &rect, viewport_w)
}

/// How an element's box is placed, which decides which ancestors clip it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    InFlow,
    Absolute,
    Fixed,
}

impl Placement {
    fn of(dom: &dyn Dom, el: ElId) -> Placement {
        match dom.style(el, "position").as_str() {
            "absolute" => Placement::Absolute,
            "fixed" => Placement::Fixed,
            _ => Placement::InFlow,
        }
    }

    /// Whether `p`'s overflow can clip a box placed like this: an in-flow box
    /// is clipped by every ancestor, an absolute box only from its containing
    /// block (the nearest positioned or transformed ancestor) up, a fixed box
    /// only from a transformed ancestor up.
    fn clipped_by(self, dom: &dyn Dom, p: ElId) -> bool {
        match self {
            Placement::InFlow => true,
            Placement::Absolute => is_positioned(dom, p) || contains_fixed(dom, p),
            Placement::Fixed => contains_fixed(dom, p),
        }
    }
}

fn is_positioned(dom: &dyn Dom, el: ElId) -> bool {
    let pos = dom.style(el, "position");
    !pos.is_empty() && pos != "static"
}

/// `transform` and `filter` make an element the containing block of its fixed
/// and absolute descendants.
fn contains_fixed(dom: &dyn Dom, el: ElId) -> bool {
    let set = |v: String| !v.is_empty() && v != "none";
    set(dom.style(el, "transform")) || set(dom.style(el, "filter"))
}

/// `overflow` does not apply to boxes that generate no block of their own.
fn clips_contents(display: &str) -> bool {
    !matches!(display, "contents" | "inline" | "table-row" | "table-row-group")
}

fn finite_or(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

/// Whether `rect` has no overlap with the band `[lo, hi]` on each axis.
fn misses(rect: &Rect, left: f64, right: f64, top: f64, bottom: f64) -> bool {
    misses_axis(rect.left, rect.right, rect.width, left, right)
        || misses_axis(rect.top, rect.bottom, rect.height, top, bottom)
}

/// A box with extent overlaps when at least 1px of it lies inside the band; a
/// box with none overlaps when its edge lies inside it.
fn misses_axis(start: f64, end: f64, extent: f64, lo: f64, hi: f64) -> bool {
    if extent > 0.0 {
        js::math_min(end, hi) - js::math_max(start, lo) < 1.0
    } else {
        start < lo || start > hi
    }
}

fn clips(value: &str) -> bool {
    matches!(value, "hidden" | "clip" | "auto" | "scroll")
}

/// The clip test for one ancestor `p` that clips `rect`'s box.
fn clip_outcome(dom: &dyn Dom, p: ElId, rect: &Rect, viewport_w: f64, viewport_h: f64) -> Option<Unpainted> {
    let ox = dom.style(p, "overflowX");
    let oy = dom.style(p, "overflowY");
    let (ox, oy) = if ox.is_empty() && oy.is_empty() {
        let o = dom.style(p, "overflow");
        (o.clone(), o)
    } else {
        (ox, oy)
    };
    let clip_x = clips(&ox);
    let clip_y = clips(&oy);
    if !clip_x && !clip_y {
        return None;
    }
    let cr = dom.rect(p);
    if !cr.all_finite() {
        return None;
    }
    // A clipping box with no area on a clipped axis shows nothing: a submenu
    // held at zero width, a panel at `max-height: 0`.
    if (clip_x && cr.width < 1.0) || (clip_y && cr.height < 1.0) {
        return Some(Unpainted::ClippedOut);
    }
    // Horizontally, every clipping or scrolling box hides what lies past its
    // edge at rest: carousel tracks, the columns of a scrolled table.
    if clip_x && misses_axis(rect.left, rect.right, rect.width, cr.left, cr.right) {
        return Some(Unpainted::ClippedOut);
    }
    // Vertically only a box that hides its overflow does. A vertical scroll
    // container is how an app shell scrolls its page, and a full-viewport
    // fixed layer that hides overflow is how a smooth-scroll library does; the
    // content below their fold is reached by the ordinary scroll.
    let hides_y = matches!(oy.as_str(), "hidden" | "clip");
    if hides_y
        && !is_viewport_layer(dom, p, &cr, viewport_w, viewport_h)
        && misses_axis(rect.top, rect.bottom, rect.height, cr.top, cr.bottom)
    {
        return Some(Unpainted::ClippedOut);
    }
    None
}

fn is_viewport_layer(dom: &dyn Dom, p: ElId, cr: &Rect, viewport_w: f64, viewport_h: f64) -> bool {
    dom.style(p, "position") == "fixed"
        && viewport_w > 0.0
        && viewport_h > 0.0
        && cr.left <= 1.0
        && cr.top <= 1.0
        && cr.right >= viewport_w - 1.0
        && cr.bottom >= viewport_h - 1.0
}

/// Whether the box lies wholly where the document cannot be scrolled to:
/// before its start, or past its scroll width.
fn outside_document(dom: &dyn Dom, rect: &Rect, viewport_w: f64) -> Option<Unpainted> {
    let sx = finite_or(dom.scroll_x(), 0.0);
    let sy = finite_or(dom.scroll_y(), 0.0);
    let left = rect.left + sx;
    let right = rect.right + sx;
    if rect.height > 0.0 && rect.bottom + sy <= 0.0 {
        return Some(Unpainted::OutsideDocument);
    }
    let root = dom.document_element()?;
    let doc_w = finite_or(dom.scroll_width(root), 0.0);
    let rtl = js::to_lower_case(&dom.style(root, "direction")) == "rtl";
    // Scrollable x range: `[0, doc_w]` left to right, `[vw - doc_w, vw]` right
    // to left. Without a measured width, only the side the scroll origin sits
    // on is known.
    let (start, end) = if doc_w > 0.0 {
        if rtl {
            (viewport_w - doc_w, viewport_w)
        } else {
            (0.0, doc_w)
        }
    } else if rtl {
        (f64::NEG_INFINITY, viewport_w)
    } else {
        (0.0, f64::INFINITY)
    };
    if rect.width > 0.0 && (right <= start || left >= end) {
        return Some(Unpainted::OutsideDocument);
    }
    None
}

/// `transition-property` / `transition-duration` pair `opacity` or `all` with
/// a non-zero duration (the lists repeat to the longer one).
fn declares_opacity_transition(dom: &dyn Dom, el: ElId) -> bool {
    let props = dom.style(el, "transitionProperty");
    let durations = dom.style(el, "transitionDuration");
    let props: Vec<&str> = props.split(',').map(js::trim).collect();
    let durations: Vec<&str> = durations.split(',').map(js::trim).collect();
    if durations.is_empty() {
        return false;
    }
    props.iter().enumerate().any(|(i, p)| {
        (*p == "opacity" || *p == "all") && css_time_seconds(durations[i % durations.len()]) > 0.0
    })
}

fn css_time_seconds(value: &str) -> f64 {
    let v = js::trim(value);
    let n = if let Some(ms) = v.strip_suffix("ms") {
        js::parse_float(ms) / 1000.0
    } else if let Some(s) = v.strip_suffix('s') {
        js::parse_float(s)
    } else {
        0.0
    };
    finite_or(n, 0.0)
}

/// An `animation-name` whose keyframes animate opacity, or whose keyframes
/// the capture could not read.
fn declares_opacity_animation(dom: &dyn Dom, el: ElId) -> bool {
    let names = dom.style(el, "animationName");
    names.split(',').map(js::trim).any(|name| {
        if name.is_empty() || name == "none" {
            return false;
        }
        match dom.keyframes(name) {
            Some(frames) => frames.iter().any(|f| f.decls.iter().any(|(p, _)| p == "opacity")),
            None => true,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    fn page() -> (FakeDom, ElId) {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 1280.0, 4000.0);
        d.set_rect(body, 0.0, 0.0, 1280.0, 4000.0);
        d.el_mut(html).scroll_width = 1280.0;
        (d, body)
    }

    fn why(d: &FakeDom, el: ElId) -> Option<Unpainted> {
        unpainted_at_capture(d, el, OwnOpacity::Counts)
    }

    #[test]
    fn a_plain_paragraph_is_painted() {
        let (mut d, body) = page();
        let p = d.add(Some(body), "p");
        d.set_rect(p, 40.0, 100.0, 400.0, 60.0);
        assert_eq!(why(&d, p), None);
        assert!(painted_at_capture(&d, p));
    }

    #[test]
    fn hidden_and_undisplayed_elements_are_not_rendered() {
        let (mut d, body) = page();
        let p = d.add(Some(body), "p");
        d.set_rect(p, 40.0, 100.0, 400.0, 60.0);
        d.el_mut(p).check_visibility = Some(false);
        assert_eq!(why(&d, p), Some(Unpainted::NotRendered));

        let menu = d.add(Some(body), "div");
        d.set_style(menu, "display", "none");
        let a = d.add(Some(menu), "a");
        assert_eq!(why(&d, a), Some(Unpainted::NotRendered));

        let q = d.add(Some(body), "p");
        d.set_style(q, "visibility", "hidden");
        assert_eq!(why(&d, q), Some(Unpainted::NotRendered));
    }

    #[test]
    fn a_demo_step_that_has_not_played_is_transparent() {
        let (mut d, body) = page();
        let step = d.add(Some(body), "div");
        d.set_styles(step, &[("opacity", "0"), ("transitionProperty", "opacity, transform"), ("transitionDuration", "0.45s, 0.45s")]);
        d.set_rect(step, 100.0, 1500.0, 339.0, 260.0);
        let label = d.add(Some(step), "span");
        d.set_rect(label, 350.0, 1530.0, 72.0, 16.0);
        assert_eq!(why(&d, label), Some(Unpainted::Transparent));
    }

    #[test]
    fn a_collapsed_submenu_clips_its_items() {
        let (mut d, body) = page();
        let li = d.add(Some(body), "li");
        d.set_rect(li, 0.0, 60.0, 390.0, 40.0);
        // max-height: 0 panel under the item.
        let sub = d.add(Some(li), "ul");
        d.set_styles(sub, &[("overflow", "hidden"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(sub, 0.0, 100.0, 390.0, 0.0);
        let p = d.add(Some(sub), "p");
        d.set_rect(p, 16.0, 100.0, 358.0, 54.0);
        assert_eq!(why(&d, p), Some(Unpainted::ClippedOut));

        // The adm.com shape: an absolute scroller at zero width.
        let side = d.add(Some(body), "ul");
        d.set_styles(side, &[("position", "absolute"), ("overflowX", "scroll"), ("overflowY", "scroll")]);
        d.set_rect(side, 0.0, 73.0, 0.0, 771.0);
        let item = d.add(Some(side), "li");
        let copy = d.add(Some(item), "p");
        d.set_rect(copy, 66.0, 268.0, 65.0, 378.0);
        assert_eq!(why(&d, copy), Some(Unpainted::ClippedOut));
    }

    #[test]
    fn a_zero_size_wrapper_that_hides_overflow_shows_nothing() {
        let (mut d, body) = page();
        let wrap = d.add(Some(body), "div");
        d.set_styles(wrap, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(wrap, 40.0, 300.0, 0.0, 0.0);
        let p = d.add(Some(wrap), "p");
        d.set_rect(p, 40.0, 300.0, 400.0, 60.0);
        assert_eq!(why(&d, p), Some(Unpainted::ClippedOut));
    }

    #[test]
    fn a_horizontal_scroller_hides_cells_past_its_edge() {
        let (mut d, body) = page();
        let scroller = d.add(Some(body), "div");
        d.set_styles(scroller, &[("overflowX", "auto"), ("overflowY", "hidden")]);
        d.set_rect(scroller, 40.0, 600.0, 310.0, 330.0);
        let track = d.add(Some(scroller), "div");
        d.set_rect(track, 40.0, 600.0, 640.0, 330.0);
        let first = d.add(Some(track), "div");
        d.set_rect(first, 40.0, 600.0, 193.0, 62.0);
        let third = d.add(Some(track), "div");
        d.set_rect(third, 382.0, 600.0, 148.0, 62.0);
        let peeking = d.add(Some(track), "div");
        d.set_rect(peeking, 233.0, 600.0, 148.0, 62.0);
        assert_eq!(why(&d, first), None);
        assert_eq!(why(&d, peeking), None);
        assert_eq!(why(&d, third), Some(Unpainted::ClippedOut));
    }

    #[test]
    fn a_vertical_scroll_container_keeps_content_below_its_fold() {
        let (mut d, body) = page();
        let shell = d.add(Some(body), "main");
        d.set_styles(shell, &[("overflowX", "hidden"), ("overflowY", "auto")]);
        d.set_rect(shell, 0.0, 0.0, 1280.0, 800.0);
        let p = d.add(Some(shell), "p");
        d.set_rect(p, 40.0, 2400.0, 600.0, 80.0);
        assert_eq!(why(&d, p), None);
    }

    #[test]
    fn a_smooth_scroll_viewport_keeps_content_below_its_fold() {
        let (mut d, body) = page();
        let wrapper = d.add(Some(body), "div");
        d.set_styles(wrapper, &[("position", "fixed"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(wrapper, 0.0, 0.0, 1280.0, 800.0);
        let content = d.add(Some(wrapper), "div");
        d.set_style(content, "transform", "matrix(1, 0, 0, 1, 0, 0)");
        d.set_rect(content, 0.0, 0.0, 1280.0, 6000.0);
        let p = d.add(Some(content), "p");
        d.set_rect(p, 40.0, 2400.0, 600.0, 80.0);
        assert_eq!(why(&d, p), None);
    }

    #[test]
    fn page_level_overflow_does_not_clip_the_fold() {
        let (mut d, body) = page();
        d.set_styles(body, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(body, 0.0, 0.0, 1280.0, 800.0);
        let p = d.add(Some(body), "p");
        d.set_rect(p, 40.0, 2400.0, 600.0, 80.0);
        assert_eq!(why(&d, p), None);
    }

    #[test]
    fn an_absolute_popover_escapes_a_clip_below_its_containing_block() {
        let (mut d, body) = page();
        let card = d.add(Some(body), "div");
        d.set_style(card, "position", "relative");
        d.set_rect(card, 40.0, 100.0, 400.0, 300.0);
        let strip = d.add(Some(card), "div");
        d.set_styles(strip, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(strip, 40.0, 100.0, 400.0, 40.0);
        let pop = d.add(Some(strip), "div");
        d.set_style(pop, "position", "absolute");
        d.set_rect(pop, 40.0, 160.0, 200.0, 80.0);
        let link = d.add(Some(pop), "a");
        d.set_rect(link, 48.0, 168.0, 80.0, 14.0);
        assert_eq!(why(&d, link), None);

        // Once the clipping strip is itself the containing block, it clips.
        d.set_style(strip, "position", "relative");
        assert_eq!(why(&d, link), Some(Unpainted::ClippedOut));
    }

    #[test]
    fn content_outside_the_document_is_not_painted() {
        let (mut d, body) = page();
        let slide = d.add(Some(body), "div");
        d.set_rect(slide, -5275.0, 3200.0, 580.0, 326.0);
        assert_eq!(why(&d, slide), Some(Unpainted::OutsideDocument));
        let past = d.add(Some(body), "div");
        d.set_rect(past, 1300.0, 3200.0, 580.0, 326.0);
        assert_eq!(why(&d, past), Some(Unpainted::OutsideDocument));
        let edge = d.add(Some(body), "div");
        d.set_rect(edge, -66.0, 560.0, 70.0, 20.0);
        assert_eq!(why(&d, edge), None);

        // Right to left, the document extends past the left edge instead.
        let root = d.document_element.unwrap();
        d.set_style(root, "direction", "rtl");
        d.el_mut(root).scroll_width = 2560.0;
        let before = d.add(Some(body), "div");
        d.set_rect(before, -800.0, 3200.0, 580.0, 326.0);
        assert_eq!(why(&d, before), None);
    }

    #[test]
    fn a_fixed_drawer_off_the_viewport_is_not_painted() {
        let (mut d, body) = page();
        let drawer = d.add(Some(body), "aside");
        d.set_style(drawer, "position", "fixed");
        d.set_rect(drawer, 1280.0, 0.0, 320.0, 800.0);
        let link = d.add(Some(drawer), "a");
        d.set_rect(link, 1296.0, 40.0, 120.0, 16.0);
        assert_eq!(why(&d, link), Some(Unpainted::OutsideDocument));

        d.set_rect(drawer, 960.0, 0.0, 320.0, 800.0);
        d.set_rect(link, 976.0, 40.0, 120.0, 16.0);
        assert_eq!(why(&d, link), None);
    }

    #[test]
    fn a_crossfade_layer_is_a_state_for_the_raster_rule() {
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        d.set_style(stack, "position", "relative");
        d.set_rect(stack, 40.0, 400.0, 320.0, 200.0);
        let poster = d.add(Some(stack), "img");
        d.set_styles(poster, &[("position", "absolute"), ("opacity", "0"), ("transitionProperty", "opacity"), ("transitionDuration", "120ms")]);
        d.set_rect(poster, 40.0, 400.0, 320.0, 200.0);
        assert_eq!(unpainted_at_capture(&d, poster, OwnOpacity::Measured), Some(Unpainted::StateLayer));

        let all = d.add(Some(stack), "img");
        d.set_styles(all, &[("opacity", "0"), ("transitionProperty", "all"), ("transitionDuration", "0.1s")]);
        d.set_rect(all, 40.0, 400.0, 320.0, 200.0);
        assert_eq!(unpainted_at_capture(&d, all, OwnOpacity::Measured), Some(Unpainted::StateLayer));

        // A raster held near zero with nothing moving it is what the rule is for.
        let buried = d.add(Some(body), "img");
        d.set_styles(buried, &[("opacity", "0.05"), ("transitionProperty", "all"), ("transitionDuration", "0s")]);
        d.set_rect(buried, 40.0, 700.0, 320.0, 200.0);
        assert_eq!(unpainted_at_capture(&d, buried, OwnOpacity::Measured), None);

        // An animation that moves opacity marks a slideshow layer; one that
        // only moves transform does not.
        let slide = d.add(Some(stack), "div");
        d.set_styles(slide, &[("opacity", "0"), ("animationName", "trophy-fade"), ("backgroundImage", "url(a.png)")]);
        d.set_rect(slide, 40.0, 400.0, 320.0, 200.0);
        d.keyframes.insert(
            "trophy-fade".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("opacity".to_string(), "1".to_string())] }],
        );
        assert_eq!(unpainted_at_capture(&d, slide, OwnOpacity::Measured), Some(Unpainted::StateLayer));
        d.keyframes.insert(
            "trophy-fade".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("transform".to_string(), "none".to_string())] }],
        );
        assert_eq!(unpainted_at_capture(&d, slide, OwnOpacity::Measured), None);

        // A raster inside a transparent layer is not painted either way.
        let hidden_stack = d.add(Some(body), "div");
        d.set_style(hidden_stack, "opacity", "0");
        let inner = d.add(Some(hidden_stack), "img");
        d.set_style(inner, "opacity", "0.05");
        assert_eq!(unpainted_at_capture(&d, inner, OwnOpacity::Measured), Some(Unpainted::Transparent));
    }

    #[test]
    fn retain_painted_only_touches_gated_rules() {
        let (mut d, body) = page();
        let wrap = d.add(Some(body), "div");
        d.set_styles(wrap, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(wrap, 40.0, 300.0, 0.0, 0.0);
        let a = d.add(Some(wrap), "a");
        d.set_rect(a, 40.0, 300.0, 80.0, 14.0);
        let mut findings = vec![
            BrowserFinding::new("undersized-ui-text", "10px functional text"),
            BrowserFinding::new("gradient-text", "gradient"),
            BrowserFinding::new("low-contrast", "2.0:1"),
        ];
        retain_painted(&d, a, &mut findings);
        let ids: Vec<&str> = findings.iter().map(|f| f.type_.as_str()).collect();
        assert_eq!(ids, vec!["gradient-text"]);
        assert_eq!(paint_gate("content-hidden-at-rest"), None);
    }
}
