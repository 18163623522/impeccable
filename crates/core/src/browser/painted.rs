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
//! the element pass already reads (style, rects, scroll metrics,
//! `checkVisibility`) and walks the ancestor chain once, so its cost is
//! bounded by the depth of the element, and the driver asks it only for
//! elements that produced a gated finding.
//!
//! What it cannot decide it keeps: a property or metric the capture did not
//! record (a snapshot older than the measurement) never removes a finding.
//!
//! `content-hidden-at-rest` is deliberately not gated: hidden text is what it
//! reports.

use super::dom::{class_attr, tag_lower, Dom, ElId, Rect};
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
    /// A near-transparent raster that is one state of a moving layer: a
    /// crossfade, a slideshow, a poster over a video, a lazy-load fade.
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
    /// that is one state of a moving layer is skipped.
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
            if op.is_finite() && op < STATE_LAYER_OPACITY && is_state_layer(dom, el) {
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
    // Whether an ancestor above the current fixed box left its containing
    // block undecided, so the box may not be a viewport layer at all.
    let mut fixed_undecided = false;
    // Where the element can be shown, as the next clipping ancestor sees it:
    // its own box until the walk passes a scroll container, whose box it
    // takes on the axis that scrolls. Scrolling brings anything in the
    // scroller's range into the scroller's box, so an ancestor above the
    // scroller hides the element only where it hides the scroller.
    let mut band = rect;
    let mut cur = dom.parent(el);
    while let Some(p) = cur {
        let display = dom.style(p, "display");
        if display == "none" || js::to_lower_case(&dom.style(p, "contentVisibility")) == "hidden" {
            return Some(Unpainted::NotRendered);
        }
        let containment = if placement == Placement::InFlow {
            Containment::DoesNot
        } else {
            contains_fixed(dom, p)
        };
        if placement == Placement::Fixed && containment == Containment::Undecided {
            fixed_undecided = true;
        }
        if placement.clipped_by(dom, p, containment) {
            // The page's own overflow propagates to the viewport, whose
            // scrolling is what brings content into view; the document test
            // below covers what it can never reach.
            let is_page = Some(p) == body || Some(p) == root;
            if !is_page && clips_contents(&display) {
                match clip_outcome(dom, p, &band, viewport_w, viewport_h) {
                    Ok(next) => band = next,
                    Err(reason) => return Some(reason),
                }
            }
            placement = Placement::of(dom, p);
            if placement == Placement::Fixed {
                fixed_rect = Some(dom.rect(p));
                fixed_undecided = false;
            }
        }
        cur = dom.parent(p);
    }

    if placement == Placement::Fixed {
        // A fixed layer no containing ancestor holds is viewport-relative and
        // never scrolled to: when the layer's own box lies outside the
        // viewport (a parked drawer), nothing in it is painted. Content that
        // runs past the edge of a layer on screen is left to the clip tests,
        // which is how a smooth-scroll viewport keeps its page. A layer that
        // an ancestor may contain is positioned against that ancestor, not
        // the viewport, and is kept.
        if fixed_undecided {
            return None;
        }
        if let Some(fr) = fixed_rect.filter(Rect::all_finite) {
            if viewport_w > 0.0 && viewport_h > 0.0 && misses(&fr, 0.0, viewport_w, 0.0, viewport_h) {
                return Some(Unpainted::OutsideDocument);
            }
        }
        return None;
    }
    outside_document(dom, &band, viewport_w)
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
    /// block (the nearest positioned or containing ancestor) up, a fixed box
    /// only from a containing ancestor up. An undecided ancestor is not taken
    /// as the containing block, which clips no more than the page does.
    fn clipped_by(self, dom: &dyn Dom, p: ElId, containment: Containment) -> bool {
        match self {
            Placement::InFlow => true,
            Placement::Absolute => is_positioned(dom, p) || containment == Containment::Contains,
            Placement::Fixed => containment == Containment::Contains,
        }
    }
}

fn is_positioned(dom: &dyn Dom, el: ElId) -> bool {
    let pos = dom.style(el, "position");
    !pos.is_empty() && pos != "static"
}

/// Whether an element is the containing block of its fixed descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Containment {
    Contains,
    DoesNot,
    /// No recorded property makes it one, and at least one property that
    /// could was not recorded.
    Undecided,
}

/// The individual transform-like properties: any value but `none` makes the
/// element the containing block of its fixed and absolute descendants.
const CONTAINING_PROPS: &[&str] = &["transform", "translate", "scale", "rotate", "perspective", "filter", "backdropFilter"];

/// The `will-change` values that do the same ahead of the change.
const CONTAINING_WILL_CHANGE: &[&str] = &["transform", "translate", "scale", "rotate", "perspective", "filter", "backdrop-filter"];

/// `transform`, `translate`, `scale`, `rotate`, `perspective`, `filter` or
/// `backdrop-filter` other than `none`, a `will-change` that names one of
/// them, and paint or layout containment (`contain: paint | layout | strict |
/// content`). A size container (`container-type`) applies size and style
/// containment only and holds nothing. A computed value always has a keyword,
/// so an empty read is a property the capture did not record.
fn contains_fixed(dom: &dyn Dom, el: ElId) -> Containment {
    let mut undecided = false;
    for prop in CONTAINING_PROPS {
        let v = dom.style(el, prop);
        if v.is_empty() {
            undecided = true;
        } else if v != "none" {
            return Containment::Contains;
        }
    }
    let will_change = dom.style(el, "willChange");
    if will_change.is_empty() {
        undecided = true;
    } else if will_change
        .split(',')
        .map(js::trim)
        .any(|v| CONTAINING_WILL_CHANGE.contains(&v))
    {
        return Containment::Contains;
    }
    let contain = dom.style(el, "contain");
    if contain.is_empty() {
        undecided = true;
    } else if contain
        .split_ascii_whitespace()
        .any(|v| matches!(v, "paint" | "layout" | "strict" | "content"))
    {
        return Containment::Contains;
    }
    if undecided {
        Containment::Undecided
    } else {
        Containment::DoesNot
    }
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

fn scrolls(value: &str) -> bool {
    matches!(value, "auto" | "scroll")
}

/// Whether a scroll container has content to scroll to. A metric the capture
/// did not record answers yes.
fn has_overflow(scroll: f64, client: f64) -> bool {
    !(scroll.is_finite() && client.is_finite()) || scroll > client + 1.0
}

/// The clip test for one ancestor `p` that clips the element, over `band`,
/// where the element can be shown as `p` sees it. `Err` names why `p` hides
/// the element; `Ok` carries the band for the ancestors above `p`.
fn clip_outcome(dom: &dyn Dom, p: ElId, band: &Rect, viewport_w: f64, viewport_h: f64) -> Result<Rect, Unpainted> {
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
        return Ok(*band);
    }
    let cr = dom.rect(p);
    if !cr.all_finite() {
        return Ok(*band);
    }
    // A clipping box with no area on a clipped axis shows nothing: a submenu
    // held at zero width, a panel at `max-height: 0`.
    if (clip_x && cr.width < 1.0) || (clip_y && cr.height < 1.0) {
        return Err(Unpainted::ClippedOut);
    }
    // Horizontally, every clipping or scrolling box hides what lies past its
    // edge at rest: carousel tracks, the columns of a scrolled table.
    if clip_x && misses_axis(band.left, band.right, band.width, cr.left, cr.right) {
        return Err(Unpainted::ClippedOut);
    }
    // Vertically only a box that hides its overflow does. A vertical scroll
    // container is how an app shell scrolls its page, and a full-viewport
    // fixed layer that hides overflow is how a smooth-scroll library does; the
    // content below their fold is reached by the ordinary scroll.
    let hides_y = matches!(oy.as_str(), "hidden" | "clip");
    let script_frame = hides_y && is_script_scroll_frame(dom, p, &cr, viewport_h);
    if hides_y
        && !is_viewport_layer(dom, p, &cr, viewport_w, viewport_h)
        && misses_axis(band.top, band.bottom, band.height, cr.top, cr.bottom)
        && !(script_frame && band.bottom > cr.bottom)
    {
        return Err(Unpainted::ClippedOut);
    }
    let (mut left, mut width) = (band.left, band.width);
    let (mut top, mut height) = (band.top, band.height);
    if scrolls(&ox) && has_overflow(dom.scroll_width(p), dom.client_width(p)) {
        left = cr.left;
        width = cr.width;
    }
    if script_frame || (scrolls(&oy) && has_overflow(dom.scroll_height(p), dom.client_height(p))) {
        top = cr.top;
        height = cr.height;
    }
    Ok(Rect::from_xywh(left, top, width, height))
}

/// Whether a box that hides its vertical overflow may be scrolled by script:
/// it is at least as tall as the fold (the viewport, or the root's layout
/// height when that is shorter) and its content runs past its bottom (or the
/// capture did not record whether it does). smooth-scrollbar and Locomotive
/// Scroll wrap the page in such a box, not always a fixed one, and move the
/// content with transforms; a capture cannot tell that from content held
/// clipped, so what lies below the box's bottom edge is kept. Smaller clips
/// (carousels, accordions, collapsed menus) and the x axis are not affected.
fn is_script_scroll_frame(dom: &dyn Dom, p: ElId, cr: &Rect, viewport_h: f64) -> bool {
    let root_h = dom
        .document_element()
        .map(|root| dom.client_height(root))
        .filter(|h| h.is_finite() && *h > 0.0);
    let fold = match root_h {
        Some(h) if viewport_h > 0.0 => js::math_min(h, viewport_h),
        Some(h) => h,
        None => viewport_h,
    };
    fold > 0.0 && cr.height >= fold - 1.0 && has_overflow(dom.scroll_height(p), dom.client_height(p))
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

/// Whether a near-transparent raster is one state of a moving layer rather
/// than an image held buried. An animation whose keyframes move opacity (or
/// cannot be read) is enough. A declared opacity transition is not: utility
/// CSS puts `opacity` in the default transition list of every element that
/// animates anything, so the transition counts only with a second marker of
/// a layer between states: an animation, a lazy-load marker on the raster or
/// its parent, or a crossfade stack (a video around it, or a sibling video or
/// raster layer over most of its box).
///
/// Even then the transition counts only while the raster is at rest at 0 (an
/// effective opacity at or below the transparent floor). A fade in or a
/// crossfade starts from 0; an image held buried sits at a faint value other
/// than 0, and Next.js images are lazy by default while Tailwind's transition
/// utilities are everywhere, so the markers alone would silence it.
///
/// A transition in progress leaves no trace in a capture, so a crossfade
/// driven by script over layers that are not siblings, or a lazy fade marked
/// only in script state, is still reported; an image genuinely held buried
/// at 0 that also carries `loading="lazy"` or sits over a sibling image is
/// skipped.
fn is_state_layer(dom: &dyn Dom, el: ElId) -> bool {
    if declares_opacity_animation(dom, el) {
        return true;
    }
    if !declares_opacity_transition(dom, el) || effective_opacity_dom(dom, el) > TRANSPARENT_FLOOR {
        return false;
    }
    declares_animation(dom, el) || marks_lazy_loading(dom, el) || in_crossfade_stack(dom, el)
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

fn animation_names(dom: &dyn Dom, el: ElId) -> Vec<String> {
    dom.style(el, "animationName")
        .split(',')
        .map(js::trim)
        .filter(|name| !name.is_empty() && *name != "none")
        .map(str::to_string)
        .collect()
}

/// Any `animation-name` other than `none`.
fn declares_animation(dom: &dyn Dom, el: ElId) -> bool {
    !animation_names(dom, el).is_empty()
}

/// An `animation-name` whose keyframes animate opacity, or whose keyframes
/// the capture could not read.
fn declares_opacity_animation(dom: &dyn Dom, el: ElId) -> bool {
    animation_names(dom, el).iter().any(|name| match dom.keyframes(name) {
        Some(frames) => frames.iter().any(|f| f.decls.iter().any(|(p, _)| p == "opacity")),
        None => true,
    })
}

/// The attributes lazy-loading libraries park a source or a load state in.
const LAZY_ATTRS: &[&str] = &[
    "data-src",
    "data-srcset",
    "data-lazy",
    "data-lazy-src",
    "data-lazy-srcset",
    "data-original",
    "data-bg",
    "data-loaded",
    "data-ll-status",
];

/// `loading="lazy"`, a lazy-loading library's source or state attribute, or a
/// class on the raster or its parent that names lazy loading or a load state
/// (`lazyload`, `owl-lazy`, `is-loading`, `preload`).
fn marks_lazy_loading(dom: &dyn Dom, el: ElId) -> bool {
    if dom
        .attr(el, "loading")
        .is_some_and(|v| js::to_lower_case(js::trim(&v)) == "lazy")
    {
        return true;
    }
    if LAZY_ATTRS.iter().any(|name| dom.attr(el, name).is_some()) {
        return true;
    }
    [Some(el), dom.parent(el)].into_iter().flatten().any(|node| {
        class_attr(dom, node).split_ascii_whitespace().any(|token| {
            let token = js::to_lower_case(token);
            token.contains("lazy") || token.contains("loading") || token.contains("preload")
        })
    })
}

const MEDIA_TAGS: &[&str] = &["img", "picture", "video", "canvas"];

/// How deep a sibling's subtree is searched for the video or raster it holds.
const LAYER_SEARCH_DEPTH: usize = 3;

/// A `<video>` parent, or a sibling that is (or holds) a video or a raster
/// covering most of the element's box: the other layers of a crossfade, or
/// the video a poster sits over.
fn in_crossfade_stack(dom: &dyn Dom, el: ElId) -> bool {
    let Some(parent) = dom.parent(el) else {
        return false;
    };
    if tag_lower(dom, parent) == "video" {
        return true;
    }
    let rect = dom.rect(el);
    dom.children(parent)
        .into_iter()
        .filter(|sibling| *sibling != el)
        .any(|sibling| holds_layer_over(dom, sibling, &rect, LAYER_SEARCH_DEPTH))
}

fn holds_layer_over(dom: &dyn Dom, node: ElId, target: &Rect, depth: usize) -> bool {
    let tag = tag_lower(dom, node);
    let raster = MEDIA_TAGS.contains(&tag.as_str()) || dom.style(node, "backgroundImage").contains("url(");
    if raster && covers_most_of(&dom.rect(node), target) {
        return true;
    }
    depth > 0
        && dom
            .children(node)
            .into_iter()
            .any(|child| holds_layer_over(dom, child, target, depth - 1))
}

/// Whether `layer` overlaps at least half of `target`'s area.
fn covers_most_of(layer: &Rect, target: &Rect) -> bool {
    if !layer.all_finite() || !target.all_finite() {
        return false;
    }
    let area = target.width * target.height;
    if area <= 0.0 {
        return false;
    }
    let w = js::math_min(layer.right, target.right) - js::math_max(layer.left, target.left);
    let h = js::math_min(layer.bottom, target.bottom) - js::math_max(layer.top, target.top);
    w > 0.0 && h > 0.0 && w * h >= 0.5 * area
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    /// The computed values a browser reports for the containing-block
    /// properties when none is set.
    const RESOLVED_INITIAL: &[(&str, &str)] = &[
        ("transform", "none"),
        ("translate", "none"),
        ("scale", "none"),
        ("rotate", "none"),
        ("perspective", "none"),
        ("filter", "none"),
        ("backdropFilter", "none"),
        ("willChange", "auto"),
        ("contain", "none"),
    ];

    fn resolved(d: &mut FakeDom, el: ElId) {
        d.set_styles(el, RESOLVED_INITIAL);
    }

    fn page() -> (FakeDom, ElId) {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 1280.0, 4000.0);
        d.set_rect(body, 0.0, 0.0, 1280.0, 4000.0);
        d.el_mut(html).scroll_width = 1280.0;
        resolved(&mut d, html);
        resolved(&mut d, body);
        (d, body)
    }

    fn why(d: &FakeDom, el: ElId) -> Option<Unpainted> {
        unpainted_at_capture(d, el, OwnOpacity::Counts)
    }

    fn raster(d: &FakeDom, el: ElId) -> Option<Unpainted> {
        unpainted_at_capture(d, el, OwnOpacity::Measured)
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

    /// The Tailwind shell: `h-screen flex overflow-hidden` around a `main`
    /// that scrolls. What lies below main's fold is reached by scrolling main,
    /// so the wrapper above it hides only what it hides of main.
    #[test]
    fn an_app_shell_keeps_what_its_inner_scroller_reaches() {
        let (mut d, body) = page();
        let shell = d.add(Some(body), "div");
        d.set_styles(shell, &[("display", "flex"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(shell, 0.0, 0.0, 1280.0, 800.0);
        let main = d.add(Some(shell), "main");
        d.set_styles(main, &[("display", "block"), ("overflowX", "auto"), ("overflowY", "auto")]);
        d.set_rect(main, 240.0, 0.0, 1040.0, 800.0);
        d.el_mut(main).client_width = 1040.0;
        d.el_mut(main).scroll_width = 1040.0;
        d.el_mut(main).client_height = 800.0;
        d.el_mut(main).scroll_height = Some(3200.0);
        let below = d.add(Some(main), "p");
        d.set_rect(below, 264.0, 2400.0, 640.0, 60.0);
        assert_eq!(why(&d, below), None);

        // A capture without scrollHeight keeps it too.
        d.el_mut(main).scroll_height = None;
        assert_eq!(why(&d, below), None);

        // When main has nothing to scroll (its height chain is broken, so it
        // is as tall as its content), a wrapper shorter than the viewport
        // hides what lies below it.
        d.set_rect(main, 240.0, 0.0, 1040.0, 3200.0);
        d.el_mut(main).client_height = 3200.0;
        d.el_mut(main).scroll_height = Some(3200.0);
        d.set_rect(shell, 0.0, 0.0, 1280.0, 600.0);
        d.el_mut(shell).client_height = 600.0;
        d.el_mut(shell).scroll_height = Some(3200.0);
        assert_eq!(why(&d, below), Some(Unpainted::ClippedOut));

        // A wrapper as tall as the viewport whose content runs past it is
        // the shape a smooth-scroll library scrolls by script, so the same
        // content is kept.
        d.set_rect(shell, 0.0, 0.0, 1280.0, 800.0);
        d.el_mut(shell).client_height = 800.0;
        assert_eq!(why(&d, below), None);
    }

    /// smooth-scrollbar and Locomotive Scroll: a viewport-tall wrapper that
    /// hides overflow, not fixed, around content moved by transforms.
    #[test]
    fn a_viewport_tall_frame_keeps_content_below_its_fold() {
        let (mut d, body) = page();
        let frame = d.add(Some(body), "div");
        d.set_styles(frame, &[("display", "block"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(frame, 0.0, 0.0, 1280.0, 800.0);
        d.el_mut(frame).client_height = 800.0;
        d.el_mut(frame).scroll_height = Some(2000.0);
        let content = d.add(Some(frame), "div");
        d.set_style(content, "transform", "matrix(1, 0, 0, 1, 0, 0)");
        d.set_rect(content, 0.0, 0.0, 1280.0, 2000.0);
        let below = d.add(Some(content), "p");
        d.set_rect(below, 40.0, 1800.0, 600.0, 40.0);
        assert_eq!(why(&d, below), None);

        // An unrecorded scrollHeight keeps it too.
        d.el_mut(frame).scroll_height = None;
        assert_eq!(why(&d, below), None);

        // The root's layout height is the fold when it is shorter than the
        // window (a horizontal scrollbar takes the rest).
        let root = d.document_element.unwrap();
        d.el_mut(root).client_height = 785.0;
        d.set_rect(frame, 0.0, 0.0, 1280.0, 785.0);
        assert_eq!(why(&d, below), None);

        // Content above the frame's top edge is not what scrolling reaches.
        let above = d.add(Some(content), "p");
        d.set_rect(above, 40.0, -400.0, 600.0, 40.0);
        assert_eq!(why(&d, above), Some(Unpainted::ClippedOut));

        // A frame whose content does not run past it holds nothing below it.
        d.el_mut(frame).scroll_height = Some(785.0);
        d.el_mut(frame).client_height = 785.0;
        assert_eq!(why(&d, below), Some(Unpainted::ClippedOut));

        // Nor does a frame shorter than the fold: an accordion panel or a
        // collapsed menu at a fixed height.
        d.el_mut(frame).scroll_height = Some(2000.0);
        d.set_rect(frame, 0.0, 0.0, 1280.0, 400.0);
        d.el_mut(frame).client_height = 400.0;
        assert_eq!(why(&d, below), Some(Unpainted::ClippedOut));

        // The x axis still clips past a viewport-tall frame's edge.
        d.set_rect(frame, 0.0, 0.0, 1280.0, 800.0);
        d.el_mut(frame).client_height = 800.0;
        d.el_mut(root).client_height = 0.0;
        let right = d.add(Some(content), "p");
        d.set_rect(right, 1400.0, 200.0, 300.0, 40.0);
        assert_eq!(why(&d, right), Some(Unpainted::ClippedOut));
    }

    #[test]
    fn a_root_scroller_under_a_hidden_page_keeps_its_content() {
        let (mut d, body) = page();
        let html = d.document_element.unwrap();
        for el in [html, body] {
            d.set_styles(el, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
            d.set_rect(el, 0.0, 0.0, 1280.0, 800.0);
        }
        let root = d.add(Some(body), "div");
        d.set_styles(root, &[("display", "block"), ("overflowX", "hidden"), ("overflowY", "auto")]);
        d.set_rect(root, 0.0, 0.0, 1280.0, 800.0);
        d.el_mut(root).client_height = 800.0;
        d.el_mut(root).scroll_height = Some(5000.0);
        let p = d.add(Some(root), "p");
        d.set_rect(p, 40.0, 4200.0, 640.0, 60.0);
        assert_eq!(why(&d, p), None);
    }

    #[test]
    fn a_scroller_wider_than_its_frame_keeps_what_it_scrolls_into_view() {
        let (mut d, body) = page();
        let frame = d.add(Some(body), "div");
        d.set_styles(frame, &[("display", "block"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(frame, 40.0, 600.0, 600.0, 300.0);
        let scroller = d.add(Some(frame), "div");
        d.set_styles(scroller, &[("display", "block"), ("overflowX", "auto"), ("overflowY", "hidden")]);
        d.set_rect(scroller, 40.0, 600.0, 900.0, 300.0);
        d.el_mut(scroller).client_width = 900.0;
        d.el_mut(scroller).scroll_width = 2400.0;
        // Inside the scroller's box, past the frame's edge: scrolling the
        // scroller brings it under the frame.
        let cell = d.add(Some(scroller), "div");
        d.set_rect(cell, 700.0, 600.0, 200.0, 100.0);
        assert_eq!(why(&d, cell), None);
        // Past the scroller's own edge at rest: dropped, as before.
        let past = d.add(Some(scroller), "div");
        d.set_rect(past, 1200.0, 600.0, 200.0, 100.0);
        assert_eq!(why(&d, past), Some(Unpainted::ClippedOut));
        // A scroller with nothing to scroll shows under the frame only what
        // already sits there.
        d.el_mut(scroller).scroll_width = 900.0;
        assert_eq!(why(&d, cell), Some(Unpainted::ClippedOut));
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

    /// A fixed badge inside a container that is its containing block sits
    /// against that container, far down the page, not against the viewport.
    #[test]
    fn a_fixed_element_inside_a_containing_block_is_not_a_viewport_layer() {
        let triggers: &[(&str, &str)] = &[
            ("transform", "matrix(1, 0, 0, 1, 0, 0)"),
            ("filter", "blur(1px)"),
            ("willChange", "transform"),
            ("willChange", "opacity, filter"),
            ("contain", "paint"),
            ("contain", "layout"),
            ("contain", "strict"),
            ("translate", "0px"),
            ("scale", "1"),
            ("rotate", "0deg"),
            ("perspective", "800px"),
            ("backdropFilter", "blur(4px)"),
        ];
        for (prop, value) in triggers {
            let (mut d, body) = page();
            let container = d.add(Some(body), "div");
            resolved(&mut d, container);
            d.set_styles(container, &[("position", "relative")]);
            d.set_rect(container, 40.0, 2000.0, 400.0, 400.0);
            let badge = d.add(Some(container), "a");
            d.set_style(badge, "position", "fixed");
            d.set_rect(badge, 60.0, 2020.0, 120.0, 14.0);
            // No trigger: a viewport layer parked below the viewport.
            assert_eq!(why(&d, badge), Some(Unpainted::OutsideDocument), "without {prop}");
            d.set_style(container, prop, value);
            assert_eq!(why(&d, badge), None, "{prop}: {value}");
        }

        // `will-change` naming something else and `contain: size` or
        // `contain: style` do not contain it.
        for (prop, value) in [("willChange", "opacity"), ("contain", "size"), ("contain", "style")] {
            let (mut d, body) = page();
            let container = d.add(Some(body), "div");
            resolved(&mut d, container);
            d.set_style(container, prop, value);
            d.set_rect(container, 40.0, 2000.0, 400.0, 400.0);
            let badge = d.add(Some(container), "a");
            d.set_style(badge, "position", "fixed");
            d.set_rect(badge, 60.0, 2020.0, 120.0, 14.0);
            assert_eq!(why(&d, badge), Some(Unpainted::OutsideDocument), "{prop}: {value}");
        }
    }

    /// A recording made before the capture read `will-change`, `contain` and
    /// the individual transforms cannot rule them out, so the badge is kept.
    #[test]
    fn an_undecided_containing_block_keeps_a_fixed_element() {
        let (mut d, body) = page();
        let container = d.add(Some(body), "div");
        d.set_styles(container, &[("transform", "none"), ("filter", "none"), ("backdropFilter", "none")]);
        d.set_rect(container, 40.0, 2000.0, 400.0, 400.0);
        let badge = d.add(Some(container), "a");
        d.set_style(badge, "position", "fixed");
        d.set_rect(badge, 60.0, 2020.0, 120.0, 14.0);
        assert_eq!(why(&d, badge), None);

        // An undecided ancestor also never makes an absolute box clip sooner.
        let strip = d.add(Some(body), "div");
        d.set_styles(strip, &[("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(strip, 40.0, 100.0, 400.0, 40.0);
        let pop = d.add(Some(strip), "div");
        d.set_style(pop, "position", "absolute");
        d.set_rect(pop, 40.0, 160.0, 200.0, 80.0);
        assert_eq!(why(&d, pop), None);
    }

    #[test]
    fn a_crossfade_layer_is_a_state_for_the_raster_rule() {
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        d.set_style(stack, "position", "relative");
        d.set_rect(stack, 40.0, 400.0, 320.0, 200.0);
        let active = d.add(Some(stack), "img");
        d.set_styles(active, &[("position", "absolute"), ("opacity", "1")]);
        d.set_rect(active, 40.0, 400.0, 320.0, 200.0);
        let poster = d.add(Some(stack), "img");
        d.set_styles(poster, &[("position", "absolute"), ("opacity", "0"), ("transitionProperty", "opacity"), ("transitionDuration", "120ms")]);
        d.set_rect(poster, 40.0, 400.0, 320.0, 200.0);
        assert_eq!(raster(&d, poster), Some(Unpainted::StateLayer));

        let all = d.add(Some(stack), "img");
        d.set_styles(all, &[("opacity", "0"), ("transitionProperty", "all"), ("transitionDuration", "0.1s")]);
        d.set_rect(all, 40.0, 400.0, 320.0, 200.0);
        assert_eq!(raster(&d, all), Some(Unpainted::StateLayer));

        // A raster held near zero with nothing moving it is what the rule is for.
        let buried = d.add(Some(body), "img");
        d.set_styles(buried, &[("opacity", "0.05"), ("transitionProperty", "all"), ("transitionDuration", "0s")]);
        d.set_rect(buried, 40.0, 700.0, 320.0, 200.0);
        assert_eq!(raster(&d, buried), None);

        // An animation that moves opacity marks a slideshow layer; one that
        // only moves transform does not.
        let slide = d.add(Some(stack), "div");
        d.set_styles(slide, &[("opacity", "0"), ("animationName", "trophy-fade"), ("backgroundImage", "url(a.png)")]);
        d.set_rect(slide, 40.0, 400.0, 320.0, 200.0);
        d.keyframes.insert(
            "trophy-fade".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("opacity".to_string(), "1".to_string())] }],
        );
        assert_eq!(raster(&d, slide), Some(Unpainted::StateLayer));
        d.keyframes.insert(
            "trophy-fade".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("transform".to_string(), "none".to_string())] }],
        );
        assert_eq!(raster(&d, slide), None);

        // A raster inside a transparent layer is not painted either way.
        let hidden_stack = d.add(Some(body), "div");
        d.set_style(hidden_stack, "opacity", "0");
        let inner = d.add(Some(hidden_stack), "img");
        d.set_style(inner, "opacity", "0.05");
        assert_eq!(raster(&d, inner), Some(Unpainted::Transparent));
    }

    const TAILWIND_TRANSITION: &[(&str, &str)] = &[
        ("transitionProperty", "color, background-color, border-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, filter, backdrop-filter"),
        ("transitionDuration", "0.15s"),
    ];

    /// Tailwind's `transition` utility lists `opacity` for every element that
    /// animates anything, so a declared transition alone does not make a
    /// buried image a state layer.
    #[test]
    fn a_transition_utility_alone_does_not_hide_a_buried_image() {
        let (mut d, body) = page();
        let hero = d.add(Some(body), "div");
        d.set_style(hero, "position", "relative");
        d.set_rect(hero, 0.0, 0.0, 1280.0, 480.0);
        let photo = d.add(Some(hero), "img");
        d.set_styles(photo, TAILWIND_TRANSITION);
        d.set_styles(photo, &[("position", "absolute"), ("opacity", "0.08")]);
        d.set_rect(photo, 0.0, 0.0, 1280.0, 480.0);
        let heading = d.add(Some(hero), "h1");
        d.set_rect(heading, 40.0, 200.0, 600.0, 60.0);
        assert_eq!(raster(&d, photo), None);

        // With a second marker, a transition at rest at 0 is a load or
        // crossfade state.
        d.set_style(photo, "opacity", "0");
        d.el_mut(photo).attrs.push(("loading".to_string(), "lazy".to_string()));
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
        d.el_mut(photo).attrs.clear();
        d.el_mut(photo).attrs.push(("class".to_string(), "owl-lazy item-img".to_string()));
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
        d.el_mut(photo).attrs.clear();
        d.el_mut(photo).attrs.push(("data-src".to_string(), "hero.jpg".to_string()));
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
        d.el_mut(photo).attrs.clear();
        d.set_style(photo, "animationName", "drift");
        d.keyframes.insert(
            "drift".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("transform".to_string(), "none".to_string())] }],
        );
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
        d.set_style(photo, "animationName", "none");
        assert_eq!(raster(&d, photo), None);
        d.set_style(photo, "opacity", "0.08");

        // A small sibling raster (a logo over the hero) is not a crossfade
        // layer.
        let logo = d.add(Some(hero), "img");
        d.set_rect(logo, 40.0, 40.0, 120.0, 40.0);
        assert_eq!(raster(&d, photo), None);
    }

    /// The Next.js Image shape: lazy by default, often with Tailwind's
    /// `transition-opacity`, held at a faint value under a dark overlay. A
    /// fade starts from 0, so the markers do not make it a state layer.
    #[test]
    fn a_lazy_image_held_at_a_faint_value_is_not_a_state_layer() {
        let (mut d, body) = page();
        let hero = d.add(Some(body), "section");
        d.set_styles(hero, &[("position", "relative"), ("overflowX", "hidden"), ("overflowY", "hidden")]);
        d.set_rect(hero, 0.0, 0.0, 1280.0, 520.0);
        let photo = d.add(Some(hero), "img");
        d.set_styles(photo, &[("position", "absolute"), ("opacity", "0.1"), ("transitionProperty", "opacity"), ("transitionDuration", "0.15s")]);
        d.el_mut(photo).attrs.push(("loading".to_string(), "lazy".to_string()));
        d.el_mut(photo).attrs.push(("data-nimg".to_string(), "fill".to_string()));
        d.set_rect(photo, 0.0, 0.0, 1280.0, 520.0);
        let overlay = d.add(Some(hero), "div");
        d.set_styles(overlay, &[("position", "absolute"), ("backgroundImage", "linear-gradient(rgba(0, 0, 0, 0.6), rgba(0, 0, 0, 0.9))")]);
        d.set_rect(overlay, 0.0, 0.0, 1280.0, 520.0);
        assert_eq!(raster(&d, photo), None);

        // Tailwind's `transition` list, the same.
        d.set_styles(photo, TAILWIND_TRANSITION);
        assert_eq!(raster(&d, photo), None);

        // A crossfade sibling over the same box does not change that.
        let next = d.add(Some(hero), "img");
        d.set_rect(next, 0.0, 0.0, 1280.0, 520.0);
        assert_eq!(raster(&d, photo), None);

        // At 0, or an effective 0 through a faint parent, it is a fade.
        d.set_style(photo, "opacity", "0.02");
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
        d.set_style(photo, "opacity", "0.1");
        d.set_style(hero, "opacity", "0.15");
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));

        // A keyframe animation that moves opacity can be caught mid-flight,
        // so it still marks a state at a faint value.
        d.set_style(hero, "opacity", "1");
        d.set_style(photo, "animationName", "pulse");
        d.keyframes.insert(
            "pulse".to_string(),
            vec![crate::browser::dom::KeyframeFrame { decls: vec![("opacity".to_string(), "0.5".to_string())] }],
        );
        assert_eq!(raster(&d, photo), Some(Unpainted::StateLayer));
    }

    /// The adant.app shape: an image poster at opacity 0 beside a wrapper that
    /// holds the playing video over the same box.
    #[test]
    fn a_poster_over_a_video_is_a_state_layer() {
        let (mut d, body) = page();
        let stack = d.add(Some(body), "div");
        d.set_style(stack, "position", "relative");
        d.set_rect(stack, 829.0, 448.0, 270.0, 428.0);
        let wrap = d.add(Some(stack), "div");
        d.set_rect(wrap, 829.0, 448.0, 270.0, 428.0);
        let video = d.add(Some(wrap), "video");
        d.set_rect(video, 829.0, 448.0, 270.0, 428.0);
        let poster = d.add(Some(stack), "img");
        d.set_styles(poster, &[("position", "absolute"), ("opacity", "0"), ("transitionProperty", "opacity"), ("transitionDuration", "0.12s")]);
        d.set_rect(poster, 829.0, 448.0, 270.0, 428.0);
        assert_eq!(raster(&d, poster), Some(Unpainted::StateLayer));

        // Moved off the video's box, the same image is reported.
        d.set_rect(video, 40.0, 1200.0, 270.0, 428.0);
        d.set_rect(wrap, 40.0, 1200.0, 270.0, 428.0);
        assert_eq!(raster(&d, poster), None);
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
