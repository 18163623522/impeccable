//! What the hit-test stack says about the layers at a run of text.
//!
//! The contrast walk reads the element's ancestors and answers with the
//! first surface on that chain. Two kinds of layer never appear there, and
//! both make its verdict about something a reader does not see:
//!
//! - a layer **above** the text that hides it: a fixed cookie banner over the
//!   hero stats, a photo avatar laid over an SVG initial. The text is covered
//!   at capture and there is nothing to score.
//! - paint **under** the text that is nobody's ancestor fill: a sibling photo
//!   under a hero subline, a slideshow image under a title, an SVG circle under
//!   an initial, a dark positioned panel under a tab strip. The walk passes
//!   under it and lands on the page ground or on a card further out.
//!
//! [`layers_at_text`] asks `elementsFromPoint` at the points the occlusion
//! grid already asks for the same box ([`occlusion_probe_points`]), so a live
//! scan answers them in the same round and a recording made for that check
//! answers them on replay. The layers are not modelled: this says only whether
//! the walk's verdict is about the text a reader meets, and leaves the verdict
//! alone wherever it cannot tell.

use super::dom::{tag_lower, Dom, ElId, Rect};
use super::element_checks::{effective_opacity_dom, parse_rgb_or_any};
use super::page_checks::{
    occlusion_probe_points, occlusion_probe_rect, occlusion_viewport, rect_holds_point,
};
use crate::color::{composite_color_over, parse_gradient_colors, Rgba};
use crate::js;
use impeccable_foundation::browser::snapshot::NS_SVG;

/// What the stacks at a run of text say about the surface the walk resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextLayers {
    /// Every decided point has an opaque layer above the text.
    Covered,
    /// Every decided point has paint under the text, above the walk's
    /// surface, that the walk never read, or a layer above it.
    UnreadSurface,
    /// The stacks agree with the walk, or part of the run is visible over the
    /// surface it named.
    Consistent,
    /// Nothing can be said: a point is not answered (below the fold, a
    /// recording that never asked it), the run is not wholly inside the
    /// viewport, or too few points find the text at all.
    Undecided,
}

impl TextLayers {
    /// Whether a contrast verdict against the walk's surface stands.
    pub fn verdict_stands(self) -> bool {
        matches!(self, TextLayers::Consistent | TextLayers::Undecided)
    }
}

/// One point's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtPoint {
    Covered,
    Unread,
    Consistent,
    /// The text is not in the stack there (an inline box's gap between
    /// lines, `pointer-events: none`). The point says nothing.
    Silent,
    /// No answer: the point was never asked, or nothing is there.
    Unanswered,
}

/// The effective opacity a layer needs to hide what is under it.
const COVER_MIN_OPACITY: f64 = 0.95;

/// The alpha a fill needs to hide what is under it, as the contrast walk
/// reads an opaque fill elsewhere.
const COVER_MIN_ALPHA: f64 = 0.95;

/// Paint fainter than this (a fill's alpha times its opacity, or the opacity
/// of an image, gradient or shape) is a wash over the surface, not a surface.
const FAINT_PAINT: f64 = 0.1;

/// How far apart, summed over the channels, an unread fill and the resolved
/// surface may be and still be one surface, the tolerance the link path uses.
const SAME_SURFACE_DISTANCE: f64 = 24.0;

/// Replaced elements that paint a picture.
const MEDIA_TAGS: &[&str] = &["img", "video", "canvas"];

/// SVG elements that paint where they are hit.
const SVG_SHAPES: &[&str] = &[
    "path", "rect", "circle", "ellipse", "polygon", "polyline", "line", "image", "use",
];

fn is_document_surface(dom: &dyn Dom, node: ElId) -> bool {
    matches!(tag_lower(dom, node).as_str(), "body" | "html")
}

fn distance(a: &Rgba, b: &Rgba) -> f64 {
    (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
}

fn wholly_inside_viewport(rect: &Rect, vw: f64, vh: f64) -> bool {
    rect.left >= 0.0 && rect.top >= 0.0 && rect.right <= vw && rect.bottom <= vh
}

/// Whether a layer above the text hides it at `(x, y)`: an image, a video, a
/// canvas, or an opaque fill, at full opacity, where the capture put it.
/// Raster backgrounds are not counted, because a transparent texture drawn
/// over a hero is one too.
fn covers(dom: &dyn Dom, node: ElId, x: f64, y: f64) -> bool {
    if is_document_surface(dom, node) || !rect_holds_point(&dom.rect(node), x, y) {
        return false;
    }
    if js::trim(&dom.style(node, "visibility")) == "hidden"
        || effective_opacity_dom(dom, node) < COVER_MIN_OPACITY
    {
        return false;
    }
    MEDIA_TAGS.contains(&tag_lower(dom, node).as_str())
        || parse_rgb_or_any(&dom.style(node, "backgroundColor"))
            .map_or(false, |c| c.alpha_or_one() >= COVER_MIN_ALPHA)
}

/// What a box that is nobody's ancestor paints under the text.
enum Paint {
    /// A picture, a gradient, a shape: nothing a colour comparison can settle.
    Unmodelled,
    /// A fill, its alpha already multiplied by the box's opacity.
    Fill(Rgba),
}

/// Whether a background image is only gradients whose strongest stop, at the
/// box's opacity, is a wash: a tint laid over a hero at 5% changes no
/// verdict. A `url()` is a picture, and gradients whose stops cannot be read
/// are not known to be faint.
fn faint_gradient_wash(image: &str, opacity: f64) -> bool {
    if js::to_lower_case(image).contains("url(") {
        return false;
    }
    let stops = parse_gradient_colors(Some(image));
    !stops.is_empty()
        && stops.iter().map(|c| c.alpha_or_one()).fold(0.0, f64::max) * opacity <= FAINT_PAINT
}

fn detached_paint(dom: &dyn Dom, node: ElId) -> Option<Paint> {
    if js::trim(&dom.style(node, "visibility")) == "hidden" {
        return None;
    }
    let opacity = effective_opacity_dom(dom, node);
    if opacity <= FAINT_PAINT {
        return None;
    }
    let tag = tag_lower(dom, node);
    if MEDIA_TAGS.contains(&tag.as_str())
        || (dom.namespace_uri(node) == NS_SVG && SVG_SHAPES.contains(&tag.as_str()))
    {
        return Some(Paint::Unmodelled);
    }
    let image = dom.style(node, "backgroundImage");
    let image = js::trim(&image);
    if !image.is_empty() && image != "none" && !faint_gradient_wash(image, opacity) {
        return Some(Paint::Unmodelled);
    }
    let fill = parse_rgb_or_any(&dom.style(node, "backgroundColor"))?;
    let alpha = fill.alpha_or_one() * opacity;
    (alpha > FAINT_PAINT).then(|| Paint::Fill(Rgba { a: Some(alpha), ..fill }))
}

fn at_point(
    dom: &dyn Dom,
    el: ElId,
    x: f64,
    y: f64,
    host: Option<ElId>,
    resolved: Option<Rgba>,
) -> AtPoint {
    let stack = dom.elements_from_point(x, y);
    if stack.is_empty() {
        return AtPoint::Unanswered;
    }
    let in_text = |n: ElId| dom.contains(el, n);
    let Some(first) = stack.iter().position(|&n| in_text(n)) else {
        return AtPoint::Silent;
    };
    for &node in &stack[..first] {
        if dom.contains(node, el) {
            continue;
        }
        if covers(dom, node, x, y) {
            return AtPoint::Covered;
        }
    }
    // The element paints its own surface: nothing under it is read.
    if host == Some(el) {
        return AtPoint::Consistent;
    }
    let last = stack.iter().rposition(|&n| in_text(n)).unwrap_or(first);
    for &node in &stack[last + 1..] {
        if host.map_or(false, |h| dom.contains(node, h)) || is_document_surface(dom, node) {
            return AtPoint::Consistent;
        }
        if in_text(node) || dom.contains(node, el) {
            continue;
        }
        if !rect_holds_point(&dom.rect(node), x, y) {
            continue;
        }
        match detached_paint(dom, node) {
            None => {}
            Some(Paint::Unmodelled) => return AtPoint::Unread,
            Some(Paint::Fill(fill)) => {
                let Some(surface) = resolved else {
                    return AtPoint::Unread;
                };
                if fill.alpha_or_one() >= COVER_MIN_ALPHA {
                    return if distance(&fill, &surface) <= SAME_SURFACE_DISTANCE {
                        AtPoint::Consistent
                    } else {
                        AtPoint::Unread
                    };
                }
                let seen = composite_color_over(&fill, &surface);
                if distance(&seen, &surface) > SAME_SURFACE_DISTANCE {
                    return AtPoint::Unread;
                }
            }
        }
    }
    AtPoint::Consistent
}

/// What the hit-test stacks over this element's text say about the surface
/// the contrast walk resolved: `host` is the box that ended the walk (`None`
/// for the canvas, the element itself where it paints its own surface) and
/// `resolved` the colour it named.
///
/// Every point of the occlusion grid over the painted box is asked, so a
/// live scan asks them all in one round. The run has to lie wholly inside the
/// viewport, since only there can a point be answered, and every point has to
/// be answered. Of those, the points where the stack holds the text decide,
/// and there have to be at least half of them. The run is covered, or reads
/// against a surface the walk never read, only where every deciding point
/// says so; a run half under a banner or half over a photo keeps its verdict.
pub fn layers_at_text(
    dom: &dyn Dom,
    el: ElId,
    host: Option<ElId>,
    resolved: Option<Rgba>,
) -> TextLayers {
    let (vw, vh) = occlusion_viewport(dom);
    let Some(rect) = occlusion_probe_rect(dom, el, &dom.rect(el)) else {
        return TextLayers::Undecided;
    };
    if !wholly_inside_viewport(&rect, vw, vh) {
        return TextLayers::Undecided;
    }
    let points = occlusion_probe_points(&rect, vw, vh);
    if points.is_empty() {
        return TextLayers::Undecided;
    }
    let answers: Vec<AtPoint> = points
        .iter()
        .map(|&(x, y)| at_point(dom, el, x, y, host, resolved))
        .collect();
    if answers.contains(&AtPoint::Unanswered) {
        return TextLayers::Undecided;
    }
    let deciding: Vec<AtPoint> = answers.into_iter().filter(|a| *a != AtPoint::Silent).collect();
    if deciding.is_empty() || deciding.len() * 2 < points.len() {
        return TextLayers::Undecided;
    }
    if deciding.iter().all(|a| *a == AtPoint::Covered) {
        TextLayers::Covered
    } else if deciding.iter().all(|a| matches!(a, AtPoint::Covered | AtPoint::Unread)) {
        TextLayers::UnreadSurface
    } else {
        TextLayers::Consistent
    }
}
