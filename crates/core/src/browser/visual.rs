//! Visual-contrast decisions from `cli/engine/browser/injected/index.mjs`
//! (see browser/mod.rs). The async pixel sampling (Image loading, canvas
//! draws, scrollIntoView, paint waits) stays in `browser-bundle/35-visual.js`
//! and calls into these through the `vc_*` wasm exports; every threshold,
//! result string, and computation of that subsystem lives here.

#![allow(unused_imports)]

use super::dom::{
    closest_or_none, direct_text, pf0, safe_id, style_px, tag_lower, Dom, ElId, Rect,
};
use super::element_checks::{parse_rgb_or_any, DISABLED_CONTROL_SELECTOR};
use super::painted::{unpainted_for, PaintGate};
use crate::checks::rules::{
    is_emoji_only_text, is_glyph_only_text, text_fill_is_transparent, TRANSPARENT_INK_FLOOR,
};
use crate::color::{contrast_ratio, parse_gradient_colors, parse_rgb, Rgba};
use impeccable_foundation::css::measures::{data_svg_intrinsic_size, ICON_MAX_PX};
use crate::constants::{SAFE_TAGS, WCAG_LARGE_BOLD_TEXT_PX, WCAG_LARGE_TEXT_PX};
use crate::js::{self, math_max, math_min, math_round, number_to_string, parse_float, parse_int, to_fixed, WS};
use crate::js_ext_a::{num_truthy, split_ws};
use crate::js_ext_b::{slice_utf16_prefix, utf16_len};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// The plans and rects this subsystem passes around are shared; re-exported
/// so `browser::visual` stays one path.
pub use impeccable_foundation::browser::visual::*;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

// JS `/url\s*\(/i`, `/gradient/i`, `/url\((?:"([^"]+)"|'([^']+)'|([^)]*))\)/i`,
// `/taint|cross-origin|Security/i`: ASCII folding (`ci`), JS `\s` (`WS`).
re!(URL_RE, format!("{}{}*\\(", js::ci("url"), WS));
re!(GRADIENT_RE, js::ci("gradient"));
re!(WS_RUN, format!("{}+", WS));
re!(
    FIRST_CSS_URL_RE,
    format!(r#"{}\((?:"([^"]+)"|'([^']+)'|([^)]*))\)"#, js::ci("url"))
);
re!(PCT_END, "%$");
re!(PX_END, "px$");
re!(
    TAINT_RE,
    format!("{}|{}|{}", js::ci("taint"), js::ci("cross-origin"), js::ci("security"))
);

pub const OVERLAY_SELECTOR: &str =
    ".impeccable-overlay, .impeccable-label, .impeccable-banner, .impeccable-tooltip";
pub const LIVE_SELECTOR: &str = "[id^=\"impeccable-live-\"]";

/// JS `s.replace(/\s+/g, ' ')`.
fn collapse_ws(s: &str) -> String {
    WS_RUN.replace_all(s, " ").into_owned()
}

/// JS `String(v || '')` on a JSON value.
fn str_or_empty(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            if num_truthy(f) {
                number_to_string(f)
            } else {
                String::new()
            }
        }
        Some(Value::Bool(true)) => "true".into(),
        _ => String::new(),
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map_or(false, num_truthy),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn rgba_from_value(v: Option<&Value>) -> Option<Rgba> {
    match v {
        Some(Value::Object(_)) => serde_json::from_value(v.unwrap().clone()).ok(),
        _ => None,
    }
}

fn rgba_value(c: Option<&Rgba>) -> Value {
    match c {
        Some(c) => serde_json::to_value(c).unwrap_or(Value::Null),
        None => Value::Null,
    }
}

// ─── candidates ─────────────────────────────────────────────────────────────

/// JS: index.mjs#collectVisualContrastReasons(el, style)
pub fn collect_visual_contrast_reasons(dom: &dyn Dom, el: ElId) -> Vec<String> {
    let mut reasons: Vec<String> = Vec::new();
    let add = |reasons: &mut Vec<String>, r: &str| {
        if !reasons.iter().any(|x| x == r) {
            reasons.push(r.to_string());
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
    let own_bg_image = dom.style(el, "backgroundImage");
    if bg_clip == "text" && !own_bg_image.is_empty() && own_bg_image != "none" {
        add(&mut reasons, "background-clip text");
    }
    let text_shadow = dom.style(el, "textShadow");
    if !text_shadow.is_empty() && text_shadow != "none" {
        add(&mut reasons, "text shadow");
    }

    let mut current = Some(el);
    while let Some(cur) = current {
        // A `display: contents` box paints nothing (Framer's page root at
        // `#000`), so its fill neither ends the walk nor adds a reason.
        if super::background::paints_no_box(dom, cur) {
            current = dom.flat_parent(cur);
            continue;
        }
        let tag = tag_lower(dom, cur);
        let bg_image = dom.style(cur, "backgroundImage");
        let is_document_surface = tag == "body" || tag == "html";
        if !is_document_surface && !bg_image.is_empty() && bg_image != "none" {
            if URL_RE.is_match(&bg_image) {
                add(&mut reasons, "image background");
            }
            if GRADIENT_RE.is_match(&bg_image) {
                add(&mut reasons, "gradient background");
            }
        }
        if parse_float(&dom.style(cur, "opacity")) < 0.99 {
            add(&mut reasons, "opacity stack");
        }
        let mix = dom.style(cur, "mixBlendMode");
        if !mix.is_empty() && mix != "normal" {
            add(&mut reasons, "blend mode");
        }
        let filter = dom.style(cur, "filter");
        if !filter.is_empty() && filter != "none" {
            add(&mut reasons, "filter");
        }
        let solid_bg = parse_rgb_or_any(&dom.style(cur, "backgroundColor"))
            // JS `solidBg.a >= 0.95` (parseRgb always sets a).
            .filter(|bg| bg.a.unwrap_or(f64::NAN) >= 0.95 && (bg_image.is_empty() || bg_image == "none"));
        // A box whose own fill is at least 0.95 opaque shows at most a
        // twentieth of what its backdrop blur produces, and that fill is the
        // surface that ends this walk: shadcn's `bg-background/95
        // backdrop-blur` sticky header. Its backdrop filter blocks nothing,
        // so it keeps the candidate under a reason no pass refuses.
        let backdrop = dom.style(cur, "backdropFilter");
        if !backdrop.is_empty() && backdrop != "none" {
            add(
                &mut reasons,
                if solid_bg.is_some() {
                    "backdrop filter under an opaque fill"
                } else {
                    "backdrop filter"
                },
            );
        }
        if solid_bg.is_some() {
            break;
        }
        current = dom.parent(cur);
    }

    let sample_rect = dom.direct_text_rect(el).unwrap_or_else(|| dom.rect(el));
    let vw = dom.inner_width();
    let vh = dom.inner_height();
    let points = [
        (
            sample_rect.left + sample_rect.width / 2.0,
            sample_rect.top + sample_rect.height / 2.0,
        ),
        (
            sample_rect.left
                + math_min(sample_rect.width - 1.0, math_max(1.0, sample_rect.width * 0.25)),
            sample_rect.top + sample_rect.height / 2.0,
        ),
        (
            sample_rect.left
                + math_min(sample_rect.width - 1.0, math_max(1.0, sample_rect.width * 0.75)),
            sample_rect.top + sample_rect.height / 2.0,
        ),
    ];
    for (x, y) in points {
        if x < 0.0 || y < 0.0 || x > vw || y > vh {
            continue;
        }
        let stack = dom.elements_from_point(x, y);
        let self_index = stack
            .iter()
            .position(|&n| n == el || dom.contains(el, n) || dom.contains(n, el));
        let Some(self_index) = self_index else { continue };
        for &node in &stack[self_index + 1..] {
            let node_tag = tag_lower(dom, node);
            if matches!(
                node_tag.as_str(),
                "img" | "picture" | "video" | "canvas" | "svg"
            ) {
                add(&mut reasons, &format!("{node_tag} underlay"));
                break;
            }
        }
    }
    reasons
}

/// Replaced boxes that paint a picture rather than a colour.
const MEDIA_TAGS: &[&str] = &["img", "picture", "video", "canvas"];

/// Bounds on [`layer_under_text`]. The test runs only for an element the rule
/// failed whose colour pair the page has not reported yet, so a page pays for
/// a handful of them, and the node budget caps the most expensive one.
const LAYER_MAX_LEVELS: usize = 32;
const LAYER_MAX_SIBLINGS: usize = 32;
const LAYER_MAX_DEPTH: usize = 6;
const LAYER_MAX_CHILDREN: usize = 64;
const LAYER_MAX_NODES: usize = 1024;

/// The largest tile, per axis, a raster background can be drawn at and still
/// count as a texture over its element's own colour.
const TEXTURE_MAX_TILE_PX: f64 = 256.0;

/// How far apart, summed over the three channels, a surface the background
/// walk never read and the one it resolved may be and still be one surface.
const SAME_SURFACE_DISTANCE: f64 = 24.0;

/// In-flow boxes paint above negative `z-index` and below positioned boxes,
/// so they sit between the two on this coarse scale.
const FLOW_LAYER: f64 = -0.5;

/// A background colour nothing behind it shows through.
fn paints_opaque_color(dom: &dyn Dom, node: ElId) -> Option<Rgba> {
    parse_rgb_or_any(&dom.style(node, "backgroundColor")).filter(|c| c.alpha_or_one() >= 0.95)
}

/// The size a box's single background image is drawn at, where the computed
/// style says it: explicit pixel sizes, or the intrinsic size of an inline SVG
/// data URI where the size is `auto`. A remote file drawn at `auto`, `cover`,
/// `contain` or a percentage has no size this can read.
fn drawn_image_size(dom: &dyn Dom, node: ElId) -> Option<(f64, f64)> {
    let size = js::to_lower_case(&dom.style(node, "backgroundSize"));
    let first = size.split(',').next().unwrap_or("");
    let tokens: Vec<&str> = first.split_ascii_whitespace().collect();
    let intrinsic = || data_svg_intrinsic_size(&dom.style(node, "backgroundImage"));
    let px = |t: &str| {
        t.ends_with("px")
            .then(|| parse_float(t))
            .filter(|v| v.is_finite() && *v > 0.0)
    };
    match tokens.as_slice() {
        [] | ["auto"] | ["auto", "auto"] => intrinsic(),
        ["auto", h] => {
            let h = px(h)?;
            let (iw, ih) = intrinsic()?;
            Some((h * iw / ih, h))
        }
        [w] | [w, "auto"] => {
            let w = px(w)?;
            let (iw, ih) = intrinsic()?;
            Some((w, w * ih / iw))
        }
        [w, h] => Some((px(w)?, px(h)?)),
        _ => None,
    }
}

/// Whether an element's raster background is a small repeating tile: a
/// noise, grain or dot texture laid over the element's own colour. The
/// tile's pixels are not in the computed style, so "faint" cannot be
/// measured; what can be measured is the shape a photograph is almost never
/// drawn in. `no-repeat`, a size this cannot read (a remote file at `auto`,
/// `cover`, `contain`, a percentage), or a tile larger than
/// [`TEXTURE_MAX_TILE_PX`] is a picture.
fn raster_tiles_as_texture(dom: &dyn Dom, node: ElId) -> bool {
    if js::to_lower_case(&dom.style(node, "background")).contains("no-repeat") {
        return false;
    }
    drawn_image_size(dom, node)
        .map_or(false, |(w, h)| w <= TEXTURE_MAX_TILE_PX && h <= TEXTURE_MAX_TILE_PX)
}

/// Whether a box's background image is an icon rather than a picture: one
/// `no-repeat` image at most [`ICON_MAX_PX`] on both axes, at a size the
/// computed style states.
fn background_is_icon(dom: &dyn Dom, node: ElId) -> bool {
    let image = dom.style(node, "backgroundImage");
    if GRADIENT_RE.is_match(&image) || js::to_lower_case(&image).matches("url(").count() != 1 {
        return false;
    }
    if !js::to_lower_case(&dom.style(node, "background")).contains("no-repeat") {
        return false;
    }
    drawn_image_size(dom, node).map_or(false, |(w, h)| w <= ICON_MAX_PX && h <= ICON_MAX_PX)
}

/// The boxes whose background image is an icon beside this element's text:
/// the element itself (an external-link mark) and its nearest `li` (an arrow
/// bullet). The background walk and the layer test both read those images
/// as absent.
pub fn icon_hosts(dom: &dyn Dom, el: ElId) -> Vec<ElId> {
    const MAX_ANCESTORS: usize = 12;
    let mut hosts = Vec::new();
    if background_is_icon(dom, el) {
        hosts.push(el);
    }
    let mut cur = dom.parent(el);
    for _ in 0..MAX_ANCESTORS {
        let Some(c) = cur else { break };
        if tag_lower(dom, c) == "li" {
            if background_is_icon(dom, c) {
                hosts.push(c);
            }
            break;
        }
        cur = dom.parent(c);
    }
    hosts
}

/// What one box paints, in the terms the layer test needs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Paint {
    /// A raster image, or a replaced media element.
    Picture,
    /// Its own opaque background colour.
    Fill(Rgba),
    /// An opaque colour drawn by its `::before` or `::after`.
    PseudoFill(Rgba),
    /// Paint the background walk cannot turn into a colour: a translucent or
    /// gradient pseudo-element over the box, a gradient drawn larger than it.
    Unmodelled,
}

/// A `::before` or `::after` stretched over at least the text and painting
/// something. A small pseudo (an underline, a bullet, a badge dot) is not a
/// surface, and neither is one laid out inline.
fn pseudo_paint(dom: &dyn Dom, node: ElId, text: &Rect) -> Option<Paint> {
    for which in ["::before", "::after"] {
        let get = |prop: &str| dom.pseudo_style(node, which, prop).unwrap_or_default();
        let content = get("content");
        let content = js::trim(&content);
        if content.is_empty() || content == "none" || content == "normal" {
            continue;
        }
        if get("display") == "none" || get("visibility") == "hidden" {
            continue;
        }
        let opacity = get("opacity");
        if !js::trim(&opacity).is_empty() && parse_float(&opacity) < 0.1 {
            continue;
        }
        let position = get("position");
        if position != "absolute" && position != "fixed" {
            continue;
        }
        let (w, h) = (parse_float(&get("width")), parse_float(&get("height")));
        if !(w >= text.width - 4.0 && h >= text.height - 4.0) {
            continue;
        }
        let image = get("backgroundImage");
        if URL_RE.is_match(&image) {
            return Some(Paint::Picture);
        }
        if GRADIENT_RE.is_match(&image) {
            return Some(Paint::Unmodelled);
        }
        if let Some(c) = parse_rgb_or_any(&get("backgroundColor")) {
            if c.alpha_or_one() >= 0.9 {
                return Some(Paint::PseudoFill(c));
            }
            if c.alpha_or_one() > 0.1 {
                return Some(Paint::Unmodelled);
            }
        }
    }
    None
}

/// Whether a gradient background is drawn larger than its box, so the part
/// under the text is a slice of the stops and not all of them: the animated
/// button that sweeps a `200% 200%` gradient across itself shows one colour
/// at a time, and scoring every stop names colours nobody sees there.
fn gradient_drawn_larger_than_box(dom: &dyn Dom, node: ElId) -> bool {
    if !GRADIENT_RE.is_match(&dom.style(node, "backgroundImage")) {
        return false;
    }
    let rect = dom.rect(node);
    let largest = math_max(rect.width, rect.height);
    js::to_lower_case(&dom.style(node, "backgroundSize"))
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .any(|t| {
            (t.ends_with('%') && parse_float(t) > 100.5)
                || (t.ends_with("px") && parse_float(t) > largest + 1.0)
        })
}

/// What a box paints under text above it, pseudo-elements first because they
/// paint over the box's own background. `body` and `html` paint the document
/// itself, and their images are not a layer over it. `icon_host` says this
/// box's background image is an icon beside the text.
fn own_paint(
    dom: &dyn Dom,
    node: ElId,
    text: &Rect,
    document_surface: bool,
    icon_host: bool,
) -> Option<Paint> {
    if let Some(paint) = pseudo_paint(dom, node, text) {
        return Some(paint);
    }
    let fill = paints_opaque_color(dom, node);
    if !document_surface
        && URL_RE.is_match(&dom.style(node, "backgroundImage"))
        && !(icon_host && background_is_icon(dom, node))
        && !(fill.is_some() && raster_tiles_as_texture(dom, node))
    {
        return Some(Paint::Picture);
    }
    if !document_surface && gradient_drawn_larger_than_box(dom, node) {
        return Some(Paint::Unmodelled);
    }
    fill.map(Paint::Fill)
}

/// Whether `outer` covers `inner`, give or take a pixel of rounding.
fn rect_covers(outer: &Rect, inner: &Rect) -> bool {
    const SLACK: f64 = 1.0;
    outer.width > 0.0
        && outer.height > 0.0
        && inner.width > 0.0
        && inner.height > 0.0
        && outer.left <= inner.left + SLACK
        && outer.top <= inner.top + SLACK
        && outer.left + outer.width >= inner.left + inner.width - SLACK
        && outer.top + outer.height >= inner.top + inner.height - SLACK
}

/// A box's place in paint order, coarsely: `context` is the `z-index` of a
/// stacking context it opens (an opacity below 1 or a transform opens one at
/// 0), `positioned` whether it paints with the positioned boxes.
#[derive(Debug, Clone, Copy)]
struct BoxLayer {
    context: Option<f64>,
    positioned: bool,
}

fn box_layer(dom: &dyn Dom, node: ElId) -> BoxLayer {
    let position = dom.style(node, "position");
    let position = js::trim(&position);
    let positioned = !position.is_empty() && position != "static";
    let z_applies = positioned
        || dom.parent(node).map_or(false, |p| {
            let display = dom.style(p, "display");
            display.contains("flex") || display.contains("grid")
        });
    let z_raw = dom.style(node, "zIndex");
    let z_raw = js::trim(&z_raw);
    let z = (z_applies && !z_raw.is_empty() && z_raw != "auto")
        .then(|| parse_float(z_raw))
        .filter(|z| z.is_finite());
    let context = z.or_else(|| {
        let opacity = dom.style(node, "opacity");
        let translucent = !js::trim(&opacity).is_empty() && parse_float(&opacity) < 1.0;
        let transform = dom.style(node, "transform");
        let transform = js::trim(&transform);
        let transformed = !transform.is_empty() && transform != "none";
        (translucent || transformed).then_some(0.0)
    });
    BoxLayer {
        context,
        positioned,
    }
}

/// The layer the text paints in, seen one ancestor further out: the
/// outermost stacking context wins, and a positioned box lifts in-flow text
/// to the positioned layer.
fn outer_layer(inner: f64, b: BoxLayer) -> f64 {
    match b.context {
        Some(z) => z,
        None if b.positioned && inner == FLOW_LAYER => 0.0,
        None => inner,
    }
}

/// The layer a descendant of a sibling paints in, one level further in. Once
/// a stacking context is met, everything inside it paints at its `z-index`.
fn inner_layer(outer: (f64, bool), b: BoxLayer) -> (f64, bool) {
    let (layer, locked) = outer;
    if locked {
        return outer;
    }
    match b.context {
        Some(z) => (z, true),
        None if b.positioned && layer == FLOW_LAYER => (0.0, false),
        None => outer,
    }
}

/// Whether a box at `layer` paints beneath text at `text_layer`.
///
/// A later sibling has to prove it: only a strictly lower layer puts it under
/// the text, the `z-index: -1` photo after the content or the section laid
/// under a `z-index: 1` header. An earlier sibling is beneath unless it opens
/// a positive `z-index` above the text, which is what a modal or a popover
/// does. The strict order would put an earlier `position: relative;
/// z-index: 0` media box over in-flow text, and on real pages that text is
/// visible over the picture (a component's own styles the snapshot cannot
/// express), so an earlier box at layer 0 or below counts as underneath.
fn paints_beneath(layer: f64, text_layer: f64, earlier: bool) -> bool {
    if earlier {
        layer <= text_layer.max(0.0)
    } else {
        layer < text_layer
    }
}

/// Whether a box lets its children paint outside its own rect. A
/// `display: contents` element has no box, so its `overflow` clips nothing.
fn overflow_visible(dom: &dyn Dom, node: ElId) -> bool {
    if dom.style(node, "display") == "contents" {
        return true;
    }
    let overflow = dom.style(node, "overflow");
    let overflow = js::trim(&overflow);
    overflow.is_empty() || overflow == "visible"
}

/// Whether a child is worth looking inside for paint under the text: its own
/// rect covers the text, or it has no box of its own (zero size, or
/// `display: contents`) and so says nothing about where its children lie.
/// A slide parked beside the viewport, or a card elsewhere on the page, is
/// skipped without spending the budget.
fn may_reach_text(dom: &dyn Dom, child: ElId, text: &Rect) -> bool {
    let rect = dom.rect(child);
    rect_covers(&rect, text)
        || rect.width <= 0.0
        || rect.height <= 0.0
        || dom.style(child, "display") == "contents"
}

/// The paint a sibling box, or something inside it, puts under the text,
/// looked at the way a reader looks down through it: its children topmost
/// first, then its own background. Only a box that covers the text rect can
/// decide, and only where it paints beneath the text. A box that does not
/// cover it is still looked inside where nothing clips its children: a
/// zero-height wrapper around an absolutely positioned photo, a
/// `display: contents` section, a carousel track narrower than its slides.
/// Below the sibling itself, only children that may reach the text are
/// visited ([`may_reach_text`]).
#[allow(clippy::too_many_arguments)]
fn layer_in_box(
    dom: &dyn Dom,
    node: ElId,
    text: &Rect,
    text_layer: f64,
    earlier: bool,
    depth: usize,
    outer: (f64, bool),
    budget: &mut usize,
) -> Option<Paint> {
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    if dom.style(node, "visibility") == "hidden" || parse_float(&dom.style(node, "opacity")) < 0.05
    {
        return None;
    }
    let layer = inner_layer(outer, box_layer(dom, node));
    let beneath = paints_beneath(layer.0, text_layer, earlier);
    let covers = rect_covers(&dom.rect(node), text);
    if covers && beneath && MEDIA_TAGS.contains(&tag_lower(dom, node).as_str()) {
        return Some(Paint::Picture);
    }
    if depth < LAYER_MAX_DEPTH && (covers || overflow_visible(dom, node)) {
        let children = dom.children(node);
        let reaching = children
            .iter()
            .rev()
            .filter(|&&child| may_reach_text(dom, child, text))
            .take(LAYER_MAX_CHILDREN);
        for &child in reaching {
            if let Some(paint) =
                layer_in_box(dom, child, text, text_layer, earlier, depth + 1, layer, budget)
            {
                return Some(paint);
            }
        }
    }
    if covers && beneath {
        own_paint(dom, node, text, false, false)
    } else {
        None
    }
}

/// What paints under a run of text, as far as layout can say, next to the
/// answer the background walk already gave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerUnder {
    /// An image, a video, a canvas or a raster background under the whole run.
    Picture,
    /// An opaque surface that is nobody's ancestor fill: a sibling panel, a
    /// section a later box lays beneath the text, a pseudo-element's colour.
    /// The walk reads ancestor fills only, so it never saw this one.
    Detached(Rgba),
    /// Paint under the text the walk cannot turn into a colour.
    Unmodelled,
    /// The first opaque surface under the text is an ancestor's own fill,
    /// which is the surface the walk answers with.
    Ancestor,
    /// Nothing decided it.
    Undecided,
}

impl From<Paint> for LayerUnder {
    fn from(paint: Paint) -> Self {
        match paint {
            Paint::Picture => LayerUnder::Picture,
            Paint::Fill(c) | Paint::PseudoFill(c) => LayerUnder::Detached(c),
            Paint::Unmodelled => LayerUnder::Unmodelled,
        }
    }
}

/// The siblings of `node` that may paint beneath the text, topmost first:
/// the higher layer first, and at one layer the later box first.
fn sibling_layer(
    dom: &dyn Dom,
    parent: ElId,
    node: ElId,
    text: &Rect,
    text_layer: f64,
    budget: &mut usize,
) -> Option<LayerUnder> {
    let siblings = dom.children(parent);
    let index = siblings.iter().position(|&s| s == node)?;
    let earlier = (0..index).rev().take(LAYER_MAX_SIBLINGS).map(|i| (i, true));
    let later = (index + 1..siblings.len())
        .take(LAYER_MAX_SIBLINGS)
        .map(|i| (i, false));
    let mut candidates: Vec<(f64, usize, bool)> = earlier
        .chain(later)
        .map(|(i, is_earlier)| {
            let layer = inner_layer((FLOW_LAYER, false), box_layer(dom, siblings[i])).0;
            (layer, i, is_earlier)
        })
        .collect();
    candidates.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.cmp(&a.1))
    });
    for (_, i, is_earlier) in candidates {
        let outer = (FLOW_LAYER, false);
        if let Some(paint) =
            layer_in_box(dom, siblings[i], text, text_layer, is_earlier, 0, outer, budget)
        {
            return Some(paint.into());
        }
    }
    None
}

/// The hit-test answer, for a page the geometric climb could not decide or
/// that ends at an opaque `body` or `html`, which is the surface the walk
/// falls back to and not proof that nothing paints above it. Only points in
/// the viewport can be asked, so this runs where a live browser answers and
/// is silent below the fold and in a replayed capture that did not record the
/// point. Each point reads the stack under the text down to the first opaque
/// box. A picture needs every answered point, the same whole-run test the
/// climb makes, so a run half over a photo and half over the page is scored
/// on the page. `None` when no point was answered.
fn hit_test_layer(dom: &dyn Dom, el: ElId, text: &Rect) -> Option<LayerUnder> {
    let vw = dom.inner_width();
    let vh = dom.inner_height();
    let y = text.top + text.height / 2.0;
    let xs = [
        text.left + text.width / 2.0,
        text.left + math_min(text.width - 1.0, math_max(1.0, text.width * 0.25)),
        text.left + math_min(text.width - 1.0, math_max(1.0, text.width * 0.75)),
    ];
    let mut answers = Vec::new();
    for x in xs {
        if x < 0.0 || y < 0.0 || x > vw || y > vh {
            continue;
        }
        let stack = dom.elements_from_point(x, y);
        let Some(self_index) = stack
            .iter()
            .position(|&n| n == el || dom.contains(el, n) || dom.contains(n, el))
        else {
            continue;
        };
        let mut answer = LayerUnder::Undecided;
        for &node in &stack[self_index + 1..] {
            if dom.contains(node, el) {
                if paints_opaque_color(dom, node).is_some() {
                    answer = LayerUnder::Ancestor;
                    break;
                }
                continue;
            }
            if MEDIA_TAGS.contains(&tag_lower(dom, node).as_str()) {
                answer = LayerUnder::Picture;
                break;
            }
            if let Some(paint) = own_paint(dom, node, text, false, false) {
                answer = paint.into();
                break;
            }
        }
        answers.push(answer);
    }
    if answers.is_empty() {
        return None;
    }
    if answers.iter().all(|a| *a == LayerUnder::Picture) {
        return Some(LayerUnder::Picture);
    }
    answers
        .iter()
        .copied()
        .find(|a| matches!(a, LayerUnder::Detached(_) | LayerUnder::Unmodelled))
        .or_else(|| {
            answers
                .contains(&LayerUnder::Ancestor)
                .then_some(LayerUnder::Ancestor)
        })
}

/// What paints under this element's text, which says whether the surface
/// `resolve_background_info` returned is the one a reader sees.
///
/// The background walk reads the ancestor chain and answers with the first
/// opaque colour on it, so it is blind in several directions: an ancestor that
/// paints a raster image or a pseudo-element over its own colour, a positioned
/// sibling (hero photo, video, canvas, a dark section) that is nobody's
/// ancestor, and a gradient drawn larger than its box. Each answers with a
/// fill that is not under the text, which is how white text over a
/// photograph is reported as `1.1:1 on #f7f8f9`.
///
/// The test is geometric, so it works at any scroll position without a hit
/// test. It climbs from the element and at each level asks two things in
/// paint order: the box's own paint (pseudo-elements, then its background),
/// then its siblings that paint beneath the text, earlier ones at the text's
/// layer or below and later ones strictly below it (a `z-index: -1` photo
/// after the content, or a section laid under a `z-index: 1` header). A
/// sibling, or a box a few levels inside it, that covers the text rect decides
/// it. The first opaque ancestor fill ends the climb with
/// [`LayerUnder::Ancestor`]: the card on the hero photo is what the link on it
/// is read against. A solid colour carrying a small tiled texture of a size
/// the style states is such a surface, and an icon on the text's own element
/// or its `li` is not a picture at all. Where the climb reaches an opaque
/// `body` or `html`, or a transparent document, the hit-test stack answers
/// instead, and a page nothing decides is [`LayerUnder::Undecided`].
///
/// A gradient under the text is not a picture. The walk scores it against
/// its stops.
pub fn layer_under_text(dom: &dyn Dom, el: ElId) -> LayerUnder {
    let text = dom.direct_text_rect(el).unwrap_or_else(|| dom.rect(el));
    let hosts = icon_hosts(dom, el);
    let mut budget = LAYER_MAX_NODES;
    let mut text_layer = FLOW_LAYER;
    let mut node = el;
    for _ in 0..LAYER_MAX_LEVELS {
        let tag = tag_lower(dom, node);
        let document_surface = tag == "body" || tag == "html";
        text_layer = outer_layer(text_layer, box_layer(dom, node));
        match own_paint(dom, node, &text, document_surface, hosts.contains(&node)) {
            Some(Paint::Fill(_)) if document_surface => {
                return hit_test_layer(dom, el, &text).unwrap_or(LayerUnder::Ancestor);
            }
            Some(Paint::Fill(_)) => return LayerUnder::Ancestor,
            Some(paint) => return paint.into(),
            None => {}
        }
        let Some(parent) = dom.parent(node) else {
            break;
        };
        if let Some(layer) = sibling_layer(dom, parent, node, &text, text_layer, &mut budget) {
            return layer;
        }
        node = parent;
    }
    hit_test_layer(dom, el, &text).unwrap_or(LayerUnder::Undecided)
}

/// Whether the surface the background walk resolved is the one under this
/// element's text, so a contrast verdict against it is about something a
/// reader sees. A picture, or paint the walk cannot model, is not; a surface
/// the walk never read is only where its colour is the one the walk named.
pub fn resolved_surface_is_under_text(dom: &dyn Dom, el: ElId, resolved: Option<Rgba>) -> bool {
    match layer_under_text(dom, el) {
        LayerUnder::Picture | LayerUnder::Unmodelled => false,
        LayerUnder::Detached(surface) => resolved.map_or(false, |bg| {
            (bg.r - surface.r).abs() + (bg.g - surface.g).abs() + (bg.b - surface.b).abs()
                <= SAME_SURFACE_DISTANCE
        }),
        LayerUnder::Ancestor | LayerUnder::Undecided => true,
    }
}

/// Below this computed font size no glyph paints: a launcher button whose
/// label is set in `font-size: 0` and drawn by an icon instead.
const MIN_GLYPH_PX: f64 = 1.0;

/// Whether a candidate's text is where a reader could see and is asked to
/// read it at rest, the gates the element pass puts in front of every text
/// measurement. The pass samples pixels, and since it scrolls an off-canvas
/// candidate into view, what it measures there is whatever that scroll
/// brought into the capture, not what a visitor sees: the cells of a tab
/// strip past its clipping edge, a carousel badge parked beside the page.
///
/// - A disabled control (WCAG 1.4.3 exempts inactive components).
/// - Text in a font under a pixel, or inked in a colour at or near alpha 0,
///   paints no glyph. `-webkit-text-fill-color: transparent` counts too,
///   except on a box that clips its own background to its text: that is a
///   gradient heading, which both passes already refuse, and it keeps the
///   slot it always had.
/// - An element not painted at capture ([`unpainted_for`] with
///   [`PaintGate::Text`]): hidden, transparent, clipped out, outside the
///   document, visually hidden, or with no area.
///
/// Glyph-only and emoji-only text is refused by the caller, before this.
/// What the capture did not record keeps the candidate, as the predicate
/// does.
fn candidate_text_reads_at_rest(dom: &dyn Dom, el: ElId) -> bool {
    if closest_or_none(dom, el, DISABLED_CONTROL_SELECTOR).is_some() {
        return false;
    }
    let font_size = parse_float(&dom.style(el, "fontSize"));
    if font_size.is_finite() && font_size < MIN_GLYPH_PX {
        return false;
    }
    let clip = {
        let a = dom.style(el, "webkitBackgroundClip");
        if a.is_empty() {
            dom.style(el, "backgroundClip")
        } else {
            a
        }
    };
    if js::trim(&clip) != "text" {
        let ink_gone = parse_rgb_or_any(&dom.style(el, "color"))
            .map_or(false, |c| c.alpha_or_one() <= TRANSPARENT_INK_FLOOR);
        if ink_gone || text_fill_is_transparent(&dom.style(el, "webkitTextFillColor")) {
            return false;
        }
    }
    unpainted_for(dom, el, PaintGate::Text).is_none()
}

/// JS: index.mjs#collectVisualContrastCandidates(options)
///
/// A candidate has to pass the element pass's text gates first
/// ([`candidate_text_reads_at_rest`]), before its reasons are read (which
/// asks hit tests) and before it takes one of the `maxCandidates` slots, so a
/// page's hidden slides no longer spend the budget its visible text needs.
pub fn collect_visual_contrast_candidates(dom: &dyn Dom, options: &Value) -> Vec<Value> {
    let max_candidates = match options.get("maxCandidates") {
        Some(Value::Number(n)) if n.as_f64().map_or(false, f64::is_finite) => {
            n.as_f64().unwrap()
        }
        _ => 12.0,
    };
    let image_only = truthy(options.get("imageOnly"));
    let body = dom.body();
    let root = dom.document_element();
    let mut candidates: Vec<Value> = Vec::new();
    for el in dom.query_all(None, "*").unwrap_or_default() {
        if (candidates.len() as f64) >= max_candidates {
            break;
        }
        if closest_or_none(dom, el, OVERLAY_SELECTOR).is_some() {
            continue;
        }
        if closest_or_none(dom, el, LIVE_SELECTOR).is_some() {
            continue;
        }
        if Some(el) == body || Some(el) == root {
            continue;
        }
        if !super::element_checks::is_rendered_for_browser_rule(dom, el) {
            continue;
        }
        let tag = tag_lower(dom, el);
        if dom.style(el, "display") == "none" || dom.style(el, "visibility") == "hidden" {
            continue;
        }
        let direct = direct_text(dom, el);
        let has_direct_text = !js::trim(&direct).is_empty();
        // Text with no letter and no digit (a lone circle, a pair of braces),
        // an icon font's ligature or a close control's `x` is not read, on
        // this path as on every other.
        if !has_direct_text
            || is_emoji_only_text(&direct)
            || super::element_checks::is_icon_text(dom, el, &direct)
        {
            continue;
        }
        let bg_color = super::background::read_own_background_color(dom, el);
        let is_styled_button = (tag == "a" || tag == "button")
            && bg_color.map_or(false, |c| c.a.map_or(false, |a| a > 0.5));
        if SAFE_TAGS.contains(&tag.as_str()) && !is_styled_button {
            continue;
        }
        let rect = dom.direct_text_rect(el).unwrap_or_else(|| dom.rect(el));
        if rect.width < 4.0 || rect.height < 4.0 {
            continue;
        }
        if !candidate_text_reads_at_rest(dom, el) {
            continue;
        }
        // A box caught mid-reveal paints one frame of a fade; its pixels say
        // nothing about the contrast a visitor meets once it settles.
        if super::element_checks::caught_mid_reveal(dom, el) {
            continue;
        }
        let reasons = collect_visual_contrast_reasons(dom, el);
        if reasons.is_empty() {
            continue;
        }
        if image_only && !reasons.iter().any(|r| r == "image background") {
            continue;
        }
        // Text another layer covers at capture (a fixed consent banner, a photo
        // over an initial) is scored by no pass. The element pass stands down
        // on it, and the samples here read only what lies under the text.
        if crate::browser::text_layers::layers_at_text(dom, el, None, None)
            == crate::browser::text_layers::TextLayers::Covered
        {
            continue;
        }
        let text_color = parse_rgb_or_any(&dom.style(el, "color"));
        let font_size = {
            let v = parse_float(&dom.style(el, "fontSize"));
            if num_truthy(v) {
                v
            } else {
                16.0
            }
        };
        let font_weight = {
            let v = parse_int(&dom.style(el, "fontWeight"), 10);
            if num_truthy(v) {
                v
            } else {
                400.0
            }
        };
        let is_large_text = font_size >= WCAG_LARGE_TEXT_PX
            || (font_size >= WCAG_LARGE_BOLD_TEXT_PX && font_weight >= 700.0);
        let threshold = if is_large_text { 3.0 } else { 4.5 };
        let sx = dom.scroll_x();
        let sy = dom.scroll_y();
        let clip = json!({
            "x": math_max(0.0, (rect.left + sx - 2.0).floor()),
            "y": math_max(0.0, (rect.top + sy - 2.0).floor()),
            "width": math_max(1.0, (rect.width + 4.0).ceil()),
            "height": math_max(1.0, (rect.height + 4.0).ceil()),
        });
        let prefer_rendered = text_color.is_none()
            || text_color.map_or(false, |c| c.a.unwrap_or(f64::NAN) < 0.99)
            || reasons.iter().any(|r| {
                matches!(
                    r.as_str(),
                    "opacity stack" | "blend mode" | "filter" | "backdrop filter" | "background-clip text"
                )
            });
        let text = slice_utf16_prefix(&collapse_ws(js::trim(&direct)), 80);
        let mut m = Map::new();
        let selector = super::driver::generate_selector(dom, el);
        let identity = candidate_match(dom, &selector, el);
        m.insert("selector".into(), Value::String(selector));
        m.insert("tagName".into(), Value::String(tag));
        m.insert("text".into(), Value::String(text));
        m.insert("threshold".into(), json!(threshold));
        m.insert("reasons".into(), json!(reasons));
        m.insert("clip".into(), clip);
        m.insert("textColor".into(), rgba_value(text_color.as_ref()));
        m.insert("preferRenderedForeground".into(), Value::Bool(prefer_rendered));
        m.insert(
            "backgroundClipText".into(),
            Value::Bool(reasons.iter().any(|r| r == "background-clip text")),
        );
        if let Some(identity) = identity {
            m.insert("match".into(), identity);
        }
        candidates.push(Value::Object(m));
    }
    candidates
}

// ─── pure math ──────────────────────────────────────────────────────────────

/// JS: index.mjs#clampByte(value)
pub fn clamp_byte(value: f64) -> f64 {
    math_max(0.0, math_min(255.0, math_round(value)))
}

/// JS: index.mjs#blendRgba(fg, bg)
pub fn blend_rgba(fg: Option<&Rgba>, bg: Option<&Rgba>) -> Option<Rgba> {
    let Some(fg) = fg else { return bg.copied() };
    if bg.is_none() || fg.a.is_none() || fg.a.unwrap() >= 0.999 {
        return Some(Rgba {
            r: clamp_byte(fg.r),
            g: clamp_byte(fg.g),
            b: clamp_byte(fg.b),
            a: Some(fg.a.unwrap_or(1.0)),
        });
    }
    let bg = bg.unwrap();
    let alpha = math_max(0.0, math_min(1.0, fg.a.unwrap()));
    Some(Rgba {
        r: clamp_byte(fg.r * alpha + bg.r * (1.0 - alpha)),
        g: clamp_byte(fg.g * alpha + bg.g * (1.0 - alpha)),
        b: clamp_byte(fg.b * alpha + bg.b * (1.0 - alpha)),
        a: Some(1.0),
    })
}

/// A background-color this translucent paints nothing worth reading; the
/// walk keeps going for what is under it (`sampleCssBackground`'s own cut-off).
const MIN_PAINTED_ALPHA: f64 = 0.05;
/// How far two opaque stops of one gradient may sit apart before the answer
/// depends on where in the box the text is.
const GRADIENT_STOP_SPREAD: f64 = 2.0;

/// What the analytic pass can say about a gradient it cannot position.
#[derive(Debug, Clone, PartialEq)]
pub enum GradientVerdict {
    /// Every stop agrees: the worst of them stands for the surface.
    Color(Rgba),
    /// The stops disagree, so which one is behind the glyphs decides the
    /// answer and nothing here knows which. Rendered pixels have to say.
    Unresolved,
}

/// JS: the gradient branch of `sampleCssBackground`, made answerable. A
/// translucent stop (`rgba(171, 171, 171, 0)` is how a browser serializes the
/// transparent end of a glow, and `from-primary/10` is a wash, not a slab)
/// composites differently at every point of the box, and two opaque stops far
/// apart pick out different verdicts at each end. `None` when the value
/// carries no stop at all.
pub fn analytic_gradient_verdict(text_color: &Rgba, colors: &[Rgba]) -> Option<GradientVerdict> {
    if colors.is_empty() {
        return None;
    }
    if colors.iter().any(|c| c.a.unwrap_or(1.0) < 0.95) {
        return Some(GradientVerdict::Unresolved);
    }
    let ratios: Vec<f64> = colors.iter().map(|c| contrast_ratio(text_color, c)).collect();
    let lo = ratios.iter().copied().fold(f64::INFINITY, math_min);
    let hi = ratios.iter().copied().fold(0.0, math_max);
    if lo > 0.0 && hi >= lo * GRADIENT_STOP_SPREAD {
        return Some(GradientVerdict::Unresolved);
    }
    pick_worst_contrast_color(text_color, colors).map(GradientVerdict::Color)
}

/// JS: index.mjs#pickWorstContrastColor(textColor, colors)
pub fn pick_worst_contrast_color(text_color: &Rgba, colors: &[Rgba]) -> Option<Rgba> {
    if colors.is_empty() {
        return None;
    }
    let mut worst = colors[0];
    let mut worst_ratio = contrast_ratio(text_color, &worst);
    for c in &colors[1..] {
        let ratio = contrast_ratio(text_color, c);
        if ratio < worst_ratio {
            worst = *c;
            worst_ratio = ratio;
        }
    }
    Some(worst)
}

/// JS: index.mjs#firstCssUrl(value)
pub fn first_css_url(value: &str) -> String {
    let Some(m) = FIRST_CSS_URL_RE.captures(value) else { return String::new() };
    let pick = m
        .get(1)
        .or_else(|| m.get(2))
        .or_else(|| m.get(3))
        .map(|g| g.as_str())
        .unwrap_or("");
    // JS `(match[1] || match[2] || match[3] || '')`: an empty group falls
    // through to the next one.
    let s = if !pick.is_empty() {
        pick
    } else {
        [m.get(2), m.get(3)]
            .iter()
            .flatten()
            .map(|g| g.as_str())
            .find(|s| !s.is_empty())
            .unwrap_or("")
    };
    js::trim(s).to_string()
}

/// JS: index.mjs#getLayerValue(value, index)
pub fn get_layer_value(value: &str, index: usize) -> String {
    value
        .split(',')
        .nth(index)
        .map(|s| js::trim(s).to_string())
        .unwrap_or_default()
}

/// JS: index.mjs#parsePositionToken(token, container, painted)
pub fn parse_position_token(token: &str, container: f64, painted: f64) -> f64 {
    if token.is_empty() || token == "center" {
        return (container - painted) / 2.0;
    }
    if token == "left" || token == "top" {
        return 0.0;
    }
    if token == "right" || token == "bottom" {
        return container - painted;
    }
    if PCT_END.is_match(token) {
        let pct = parse_float(token) / 100.0;
        return (container - painted) * pct;
    }
    if PX_END.is_match(token) {
        return pf0(token);
    }
    (container - painted) / 2.0
}

/// JS: index.mjs#parsePositionPair(positionValue)
pub fn parse_position_pair(position_value: &str) -> (String, String) {
    let src = if position_value.is_empty() { "50% 50%" } else { position_value };
    let tokens: Vec<&str> = split_ws(js::trim(src))
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect();
    let first = tokens.first().copied().unwrap_or("50%");
    if tokens.len() < 2 {
        if first == "top" || first == "bottom" {
            return ("50%".into(), first.into());
        }
        return (first.into(), "50%".into());
    }
    let second = tokens[1];
    (first.into(), if second.is_empty() { "50%".into() } else { second.into() })
}

/// JS `image.naturalWidth || image.videoWidth || image.width || 1` — the JS
/// side hands the already-`||`-chained value, 0 when none.
fn intrinsic_or_one(v: f64) -> f64 {
    if num_truthy(v) {
        v
    } else {
        1.0
    }
}

/// JS: index.mjs#resolvePaintedImageRect(containerRect, image, sizeValue, positionValue)
pub fn resolve_painted_image_rect(
    container: &Box4,
    intrinsic_w: f64,
    intrinsic_h: f64,
    size_value: &str,
    position_value: &str,
) -> PaintedRect {
    let iw = intrinsic_or_one(intrinsic_w);
    let ih = intrinsic_or_one(intrinsic_h);
    let mut painted_w = iw;
    let mut painted_h = ih;
    let size = js::trim(if size_value.is_empty() { "auto" } else { size_value });
    if size == "cover" || size == "contain" {
        let scale = if size == "cover" {
            math_max(container.width / iw, container.height / ih)
        } else {
            math_min(container.width / iw, container.height / ih)
        };
        painted_w = iw * scale;
        painted_h = ih * scale;
    } else if !size.is_empty() && size != "auto" {
        let parts = split_ws(size);
        let width_token = parts.first().copied().unwrap_or("");
        let height_token = parts.get(1).copied().filter(|s| !s.is_empty()).unwrap_or("auto");
        if PCT_END.is_match(width_token) {
            painted_w = container.width * (parse_float(width_token) / 100.0);
        } else if PX_END.is_match(width_token) {
            let v = parse_float(width_token);
            if num_truthy(v) {
                painted_w = v;
            }
        }
        if height_token == "auto" {
            painted_h = painted_w * (ih / iw);
        } else if PCT_END.is_match(height_token) {
            painted_h = container.height * (parse_float(height_token) / 100.0);
        } else if PX_END.is_match(height_token) {
            let v = parse_float(height_token);
            if num_truthy(v) {
                painted_h = v;
            }
        }
    }
    let (x_token, y_token) = parse_position_pair(position_value);
    let position_x = parse_position_token(&x_token, container.width, painted_w);
    let position_y = parse_position_token(&y_token, container.height, painted_h);
    PaintedRect {
        left: container.left + position_x,
        top: container.top + position_y,
        width: painted_w,
        height: painted_h,
        intrinsic_width: iw,
        intrinsic_height: ih,
    }
}

/// JS: index.mjs#resolveObjectImageRect(containerRect, image, style)
pub fn resolve_object_image_rect(
    container: &Box4,
    intrinsic_w: f64,
    intrinsic_h: f64,
    object_fit: &str,
    object_position: &str,
) -> PaintedRect {
    let iw = intrinsic_or_one(intrinsic_w);
    let ih = intrinsic_or_one(intrinsic_h);
    let fit = if object_fit.is_empty() { "fill" } else { object_fit };
    let mut painted_w = container.width;
    let mut painted_h = container.height;
    if fit == "contain" || fit == "cover" {
        let scale = if fit == "cover" {
            math_max(container.width / iw, container.height / ih)
        } else {
            math_min(container.width / iw, container.height / ih)
        };
        painted_w = iw * scale;
        painted_h = ih * scale;
    } else if fit == "none" {
        painted_w = iw;
        painted_h = ih;
    } else if fit == "scale-down" {
        let contain_scale = math_min(math_min(container.width / iw, container.height / ih), 1.0);
        painted_w = iw * contain_scale;
        painted_h = ih * contain_scale;
    }
    let (x_token, y_token) = parse_position_pair(object_position);
    PaintedRect {
        left: container.left + parse_position_token(&x_token, container.width, painted_w),
        top: container.top + parse_position_token(&y_token, container.height, painted_h),
        width: painted_w,
        height: painted_h,
        intrinsic_width: iw,
        intrinsic_height: ih,
    }
}

/// JS: index.mjs#pointToImageSource(point, paintedRect)
pub fn point_to_image_source(x: f64, y: f64, painted: &PaintedRect) -> Option<(f64, f64)> {
    if x < painted.left
        || y < painted.top
        || x > painted.left + painted.width
        || y > painted.top + painted.height
    {
        return None;
    }
    Some((
        math_max(
            0.0,
            math_min(
                painted.intrinsic_width - 1.0,
                ((x - painted.left) / painted.width) * painted.intrinsic_width,
            ),
        ),
        math_max(
            0.0,
            math_min(
                painted.intrinsic_height - 1.0,
                ((y - painted.top) / painted.height) * painted.intrinsic_height,
            ),
        ),
    ))
}

/// JS: index.mjs#textSamplePoints(rect)
pub fn text_sample_points(rect: &Rect, inner_width: f64, inner_height: f64) -> Vec<(f64, f64)> {
    let inset_x = math_min(12.0, math_max(1.0, rect.width * 0.12));
    let inset_y = math_min(8.0, math_max(1.0, rect.height * 0.22));
    let xs: Vec<f64> = if rect.width < 28.0 {
        vec![rect.left + rect.width / 2.0]
    } else {
        vec![
            rect.left + inset_x,
            rect.left + rect.width / 2.0,
            rect.right - inset_x,
        ]
    };
    let ys: Vec<f64> = if rect.height < 22.0 {
        vec![rect.top + rect.height / 2.0]
    } else {
        vec![
            rect.top + inset_y,
            rect.top + rect.height / 2.0,
            rect.bottom - inset_y,
        ]
    };
    let mut points = Vec::new();
    for &y in &ys {
        for &x in &xs {
            if x >= 0.0 && y >= 0.0 && x <= inner_width && y <= inner_height {
                points.push((x, y));
            }
        }
    }
    points
}

// ─── raster sampling helpers (sampleDrawablePixel) ─────────────────────────

/// JS: index.mjs#sampleDrawablePixel (canvas sizing)
pub fn raster_plan(intrinsic_w: f64, intrinsic_h: f64) -> RasterPlan {
    let iw = intrinsic_or_one(intrinsic_w);
    let ih = intrinsic_or_one(intrinsic_h);
    let max_raster_side = 640.0;
    let scale = math_min(1.0, max_raster_side / math_max(iw, ih));
    let width = math_max(1.0, math_round(iw * scale));
    let height = math_max(1.0, math_round(ih * scale));
    RasterPlan {
        width,
        height,
        scale_x: width / iw,
        scale_y: height / ih,
    }
}

/// JS: index.mjs#sampleDrawablePixel (source point → raster pixel)
pub fn raster_pixel(plan: &RasterPlan, source_x: f64, source_y: f64) -> (f64, f64) {
    (
        math_max(0.0, math_min(plan.width - 1.0, (source_x * plan.scale_x).floor())),
        math_max(0.0, math_min(plan.height - 1.0, (source_y * plan.scale_y).floor())),
    )
}

/// JS: `{ status: 'sampled', color: { r, g, b, a: data[3] / 255 } }`.
pub fn pixel_sample(r: f64, g: f64, b: f64, a255: f64) -> Value {
    json!({ "status": "sampled", "color": { "r": r, "g": g, "b": b, "a": a255 / 255.0 } })
}

/// JS: the canvas draw / getImageData error classification.
pub fn raster_error_reason(message: &str) -> String {
    if TAINT_RE.is_match(message) {
        "tainted image".to_string()
    } else {
        "image sample failed".to_string()
    }
}

/// JS: `{ status: 'unresolved', reason: cached?.reason || 'image sample failed' }`.
pub fn raster_failure_sample(reason: &str) -> Value {
    let reason = if reason.is_empty() { "image sample failed" } else { reason };
    json!({ "status": "unresolved", "reason": reason })
}

/// JS: `{ status: 'unresolved', reason: 'canvas unavailable' }`.
pub fn raster_no_context_sample() -> Value {
    json!({ "status": "unresolved", "reason": "canvas unavailable" })
}

// ─── the background stack walk (sampleVisualBackgroundAtPoint) ─────────────

/// JS: index.mjs#sampleVisualBackgroundAtPoint — the depth cap and the node
/// list (`elementsFromPoint` stack from the element down, overlay chrome
/// skipped). `Err` carries the early-unresolved sample.
pub fn stack_nodes(dom: &dyn Dom, el: ElId, x: f64, y: f64, depth: f64) -> Result<Vec<StackNode>, Value> {
    if depth > 8.0 {
        return Err(json!({ "status": "unresolved", "reason": "background stack too deep" }));
    }
    let stack = dom.elements_from_point(x, y);
    let self_index = stack.iter().position(|&n| n == el || dom.contains(el, n));
    let nodes: Vec<ElId> = match self_index {
        Some(i) => stack[i..].to_vec(),
        None => {
            let mut v = vec![el];
            v.extend(stack);
            v
        }
    };
    Ok(nodes
        .into_iter()
        .filter(|&n| closest_or_none(dom, n, OVERLAY_SELECTOR).is_none())
        .map(|n| {
            let tag = tag_lower(dom, n);
            let kind = if tag == "img" {
                "img"
            } else if tag == "canvas" || tag == "video" {
                "raster"
            } else if tag == "svg" {
                // Vector paint (an avatar circle, an inline illustration) is
                // opaque to this walk: its fills live on children no CSS
                // background read can see. Reading through it would report the
                // surface behind the artwork as the text's background.
                "unreadable"
            } else {
                "css"
            };
            StackNode {
                el: n,
                kind: kind.to_string(),
            }
        })
        .collect())
}

/// The walk's stop when it reaches paint it cannot read (`unreadable` stack
/// nodes). `stop` ends the walk: what is under this node is not what the text
/// sits on, so the nodes below it must not answer for it.
pub fn unreadable_stack_sample(dom: &dyn Dom, node: ElId) -> Value {
    json!({ "status": "unresolved", "reason": format!("{} paint", tag_lower(dom, node)), "stop": true })
}

/// An unresolved sample that ends the walk rather than passing it down.
pub fn sample_ends_walk(sample: &Value) -> bool {
    sample.get("stop").and_then(Value::as_bool) == Some(true)
}

/// The walk's compositing fold: `pending` is the translucent surfaces it
/// passed through, topmost first, and `ground` the opaque sample under them.
/// Each is blended into the one below, so the topmost surface names the
/// method (`...+alpha`), as the old pairwise recursion did.
pub fn composite_stack(pending: &[Value], ground: &Value) -> Value {
    let mut out = ground.clone();
    for sample in pending.iter().rev() {
        out = alpha_composite(sample.clone(), &out);
    }
    out
}

/// JS: index.mjs#sampleImageElement — painted rect + source point for the
/// `<img>` at `node` (`intrinsic_*` are the JS `naturalWidth || videoWidth ||
/// width` chain, 0 when none). `Err` is the unresolved sample.
pub fn img_source_point(
    dom: &dyn Dom,
    node: ElId,
    intrinsic_w: f64,
    intrinsic_h: f64,
    x: f64,
    y: f64,
) -> Result<(PaintedRect, (f64, f64)), Value> {
    let rect: Box4 = dom.rect(node).into();
    let painted = resolve_object_image_rect(
        &rect,
        intrinsic_w,
        intrinsic_h,
        &dom.style(node, "objectFit"),
        &dom.style(node, "objectPosition"),
    );
    match point_to_image_source(x, y, &painted) {
        Some(p) => Ok((painted, p)),
        None => Err(json!({ "status": "unresolved", "reason": "point outside image" })),
    }
}

/// JS: index.mjs#sampleImageElement — the retry with the separately loaded
/// image: the painted box stays, only the intrinsic size changes
/// (`loaded.naturalWidth || loaded.width || paintedRect.intrinsicWidth`).
pub fn img_loaded_source_point(
    painted: &PaintedRect,
    loaded_w: f64,
    loaded_h: f64,
    x: f64,
    y: f64,
) -> Option<(f64, f64)> {
    let loaded_rect = PaintedRect {
        intrinsic_width: if num_truthy(loaded_w) { loaded_w } else { painted.intrinsic_width },
        intrinsic_height: if num_truthy(loaded_h) { loaded_h } else { painted.intrinsic_height },
        ..*painted
    };
    point_to_image_source(x, y, &loaded_rect)
}

/// JS: `{ ...sample, method: 'canvas-img-underlay' }` when sampled, else the sample.
pub fn img_finish(sample: Value) -> Value {
    with_method(sample, "canvas-img-underlay")
}

/// JS: canvas/video underlay source point (`intrinsic_*` are
/// `node.width || node.videoWidth`, 0 when none → the rect's size).
pub fn raster_source_point(dom: &dyn Dom, node: ElId, intrinsic_w: f64, intrinsic_h: f64, x: f64, y: f64) -> Option<(f64, f64)> {
    let rect = dom.rect(node);
    let painted = PaintedRect {
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
        intrinsic_width: if num_truthy(intrinsic_w) { intrinsic_w } else { rect.width },
        intrinsic_height: if num_truthy(intrinsic_h) { intrinsic_h } else { rect.height },
    };
    point_to_image_source(x, y, &painted)
}

/// JS: `{ ...sample, method: \`canvas-${tag}-underlay\` }` when sampled.
pub fn raster_finish(dom: &dyn Dom, node: ElId, sample: Value) -> Value {
    let tag = tag_lower(dom, node);
    with_method(sample, &format!("canvas-{tag}-underlay"))
}

fn with_method(sample: Value, method: &str) -> Value {
    if sample.get("status").and_then(Value::as_str) == Some("sampled") {
        let mut m = sample.as_object().cloned().unwrap_or_default();
        m.insert("method".into(), Value::String(method.to_string()));
        Value::Object(m)
    } else {
        sample
    }
}

/// JS: index.mjs#sampleCssBackground — every decision except the image
/// load and the canvas sample.
pub fn css_plan(dom: &dyn Dom, node: ElId, text_color: Option<&Rgba>) -> CssPlan {
    let bg_image = dom.style(node, "backgroundImage");
    if !bg_image.is_empty() && bg_image != "none" {
        if GRADIENT_RE.is_match(&bg_image) {
            if let Some(tc) = text_color {
                let colors = parse_gradient_colors(Some(&bg_image));
                match analytic_gradient_verdict(tc, &colors) {
                    Some(GradientVerdict::Color(color)) => {
                        return CssPlan::Sample {
                            sample: json!({ "status": "sampled", "color": color, "method": "analytic-gradient" }),
                        };
                    }
                    Some(GradientVerdict::Unresolved) => {
                        return CssPlan::Sample {
                            sample: json!({ "status": "unresolved", "reason": "gradient stops disagree", "stop": true }),
                        };
                    }
                    None => {}
                }
            } else {
                // JS-PARITY: contrastRatio(null, c) throws in the JS when
                // textColor is null; analyzeVisualContrastCandidate never
                // reaches here without one, so this branch is unreachable.
            }
        }
        if URL_RE.is_match(&bg_image) {
            let size = {
                let v = get_layer_value(&dom.style(node, "backgroundSize"), 0);
                if v.is_empty() { "auto".to_string() } else { v }
            };
            let position = {
                let v = get_layer_value(&dom.style(node, "backgroundPosition"), 0);
                if v.is_empty() { "50% 50%".to_string() } else { v }
            };
            return CssPlan::Url {
                url: first_css_url(&bg_image),
                size,
                position,
            };
        }
    }
    let bg = parse_rgb_or_any(&dom.style(node, "backgroundColor"));
    if let Some(bg) = bg {
        if bg.a.unwrap_or(f64::NAN) > MIN_PAINTED_ALPHA {
            return CssPlan::Sample {
                sample: json!({ "status": "sampled", "color": bg, "method": "solid-background" }),
            };
        }
    }
    CssPlan::Sample {
        sample: json!({ "status": "unresolved", "reason": "no readable background" }),
    }
}

/// JS: sampleCssBackground url path — `if (!img) return { status:
/// 'unresolved', reason: 'image unavailable' }`.
///
/// An image this pass could not load (a cross-origin file served without
/// CORS) is still painted on the page. Where its placement is known without
/// its pixels (a size stated in pixels or percentages, or `cover`) and it
/// covers neither the candidate's text nor its own box, it is not the surface
/// either, and the walk ends there as it does once the image loads
/// ([`css_url_source_point`]). Otherwise the walk goes on, as before.
pub fn css_url_no_image(dom: &dyn Dom, node: ElId, el: ElId, size: &str, position: &str) -> Value {
    let rect: Box4 = dom.rect(node).into();
    let painted = resolve_painted_image_rect(&rect, 0.0, 0.0, size, position);
    if image_leaves_text_uncovered(dom, node, el, &painted, 0.0, 0.0, size) {
        return uncovered_image_sample();
    }
    json!({ "status": "unresolved", "reason": "image unavailable" })
}

/// The sample that ends the walk at an image that is not under the text.
fn uncovered_image_sample() -> Value {
    json!({
        "status": "unresolved",
        "reason": "background image does not cover the text",
        "stop": true,
    })
}

/// JS: sampleCssBackground url path — painted rect of the loaded image over
/// the node's box and the source point; `Err` is the unresolved sample.
///
/// An image is the surface under the text only where it covers that text.
/// One drawn `no-repeat` at a size and position that leave the candidate's
/// text box uncovered (a mark beside a button label), or that leave its own
/// box uncovered (an icon sprite framing a count, a picture parked at one
/// side of a panel), is a mark laid on a surface this walk does not read:
/// the box's own `background-color` is never read once it carries an image,
/// and the image's transparent pixels composite over whatever lies further
/// down the stack. That is how a white count on a dark badge read 1.1:1
/// against the page's own fill. Such an image ends the walk unresolved,
/// and the candidate is left to the pixel pass. `el` is the candidate.
///
/// Where the placement cannot be measured (a size that needs the image's
/// intrinsic size and none was loaded, a size in other units, a repeat the
/// capture did not record), the image is read as before. `body` and `html`
/// paint the document, so only the text box is asked of them.
#[allow(clippy::too_many_arguments)]
pub fn css_url_source_point(
    dom: &dyn Dom,
    node: ElId,
    el: ElId,
    intrinsic_w: f64,
    intrinsic_h: f64,
    size: &str,
    position: &str,
    x: f64,
    y: f64,
) -> Result<(f64, f64), Value> {
    let rect: Box4 = dom.rect(node).into();
    let painted = resolve_painted_image_rect(&rect, intrinsic_w, intrinsic_h, size, position);
    if image_leaves_text_uncovered(dom, node, el, &painted, intrinsic_w, intrinsic_h, size) {
        return Err(uncovered_image_sample());
    }
    point_to_image_source(x, y, &painted)
        .ok_or_else(|| json!({ "status": "unresolved", "reason": "point outside background image" }))
}

/// How far a no-repeat image may fall short of a box and still cover it:
/// the rounding of positions and sizes.
const COVER_SLACK_PX: f64 = 1.0;

/// See [`css_url_source_point`].
#[allow(clippy::too_many_arguments)]
fn image_leaves_text_uncovered(
    dom: &dyn Dom,
    node: ElId,
    el: ElId,
    painted: &PaintedRect,
    intrinsic_w: f64,
    intrinsic_h: f64,
    size: &str,
) -> bool {
    let Some((repeat_x, repeat_y)) = background_repeat_axes(&dom.style(node, "background")) else {
        return false;
    };
    if (repeat_x && repeat_y) || !placed_size_is_known(size, intrinsic_w, intrinsic_h) {
        return false;
    }
    let covers = |target: &Rect| {
        if !target.all_finite() || target.width <= 0.0 || target.height <= 0.0 {
            return true;
        }
        let x = repeat_x
            || (painted.left <= target.left + COVER_SLACK_PX
                && painted.left + painted.width >= target.right - COVER_SLACK_PX);
        let y = repeat_y
            || (painted.top <= target.top + COVER_SLACK_PX
                && painted.top + painted.height >= target.bottom - COVER_SLACK_PX);
        x && y
    };
    let text = dom.direct_text_rect(el).unwrap_or_else(|| dom.rect(el));
    if !covers(&text) {
        return true;
    }
    let tag = tag_lower(dom, node);
    tag != "body" && tag != "html" && !covers(&dom.rect(node))
}

/// Which axes a box's first background layer repeats on, read from the
/// computed `background` shorthand (`rgba(0, 0, 0, 0.6) url("…") no-repeat
/// scroll 50% 50% / 22px 16px padding-box border-box`). `None` when the
/// shorthand was not recorded or names no repeat keyword.
fn background_repeat_axes(shorthand: &str) -> Option<(bool, bool)> {
    let lower = js::to_lower_case(shorthand);
    // The first layer, with every function (`url(…)`, `rgba(…)`) removed so
    // neither its commas nor its contents are read as keywords.
    let mut layer = String::new();
    let mut depth = 0usize;
    for c in lower.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => break,
            _ if depth == 0 => layer.push(c),
            _ => {}
        }
    }
    let words: Vec<&str> = layer
        .split_ascii_whitespace()
        .filter(|w| matches!(*w, "repeat" | "no-repeat" | "repeat-x" | "repeat-y" | "space" | "round"))
        .collect();
    match words.as_slice() {
        [] => None,
        ["repeat-x"] => Some((true, false)),
        ["repeat-y"] => Some((false, true)),
        [one] => Some((*one != "no-repeat", *one != "no-repeat")),
        [x, y, ..] => Some((*x != "no-repeat", *y != "no-repeat")),
    }
}

/// Whether [`resolve_painted_image_rect`] places an image of this computed
/// `background-size` where the browser does: `cover`, or pixel and
/// percentage sizes on both axes, or any size once the image's intrinsic
/// size is known. Another unit, or a `calc()` mixing two, is not modelled.
fn placed_size_is_known(size: &str, intrinsic_w: f64, intrinsic_h: f64) -> bool {
    let size = js::to_lower_case(js::trim(size));
    if size.contains('(') {
        return false;
    }
    let tokens: Vec<&str> = size.split_ascii_whitespace().collect();
    let modelled = |t: &str| t == "auto" || t.ends_with("px") || t.ends_with('%');
    match tokens.as_slice() {
        ["cover"] => true,
        ["contain"] | [] => num_truthy(intrinsic_w) && num_truthy(intrinsic_h),
        [w] if modelled(w) => num_truthy(intrinsic_w) && num_truthy(intrinsic_h),
        [w, h] if modelled(w) && modelled(h) => {
            (*w != "auto" && *h != "auto") || (num_truthy(intrinsic_w) && num_truthy(intrinsic_h))
        }
        _ => false,
    }
}

/// JS: `{ ...sample, method: 'canvas-background-image' }` when sampled.
pub fn css_url_finish(sample: Value) -> Value {
    with_method(sample, "canvas-background-image")
}

/// JS: `!sample.color || sample.color.a == null || sample.color.a >= 0.95` —
/// a sampled color that ends the walk (no compositing over what is beneath).
pub fn sample_is_opaque(sample: &Value) -> bool {
    let Some(color) = sample.get("color") else { return true };
    if color.is_null() {
        return true;
    }
    match color.get("a") {
        None | Some(Value::Null) => true,
        Some(Value::Number(n)) => n.as_f64().map_or(false, |a| a >= 0.95),
        // JS `>=` on a non-number coerces; a non-numeric alpha never occurs.
        Some(_) => false,
    }
}

/// JS: the alpha compositing step — `under` sampled → blended color with
/// `${sample.method}+alpha`, else the translucent sample itself.
pub fn alpha_composite(sample: Value, under: &Value) -> Value {
    if under.get("status").and_then(Value::as_str) == Some("sampled") {
        let top = rgba_from_value(sample.get("color"));
        let base = rgba_from_value(under.get("color"));
        let blended = blend_rgba(top.as_ref(), base.as_ref());
        let method = str_or_empty(sample.get("method"));
        return json!({
            "status": "sampled",
            "color": rgba_value(blended.as_ref()),
            "method": format!("{method}+alpha"),
        });
    }
    sample
}

/// JS: the walk's final `{ status: 'unresolved', reason }` from the collected
/// per-node reasons (deduped, first three, comma-joined; fallback text).
pub fn unresolved_from_reasons(reasons: &[String]) -> Value {
    let mut uniq: Vec<&str> = Vec::new();
    for r in reasons {
        if r.is_empty() {
            continue;
        }
        if !uniq.contains(&r.as_str()) {
            uniq.push(r);
        }
    }
    let joined = uniq.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    let reason = if joined.is_empty() { "no readable visual background".to_string() } else { joined };
    json!({ "status": "unresolved", "reason": reason })
}

// ─── analyzeVisualContrastCandidate ─────────────────────────────────────────

/// `{ ...candidate, ...extra }` with JS spread semantics (existing keys keep
/// their position; new keys append).
fn spread(candidate: &Value, extra: Vec<(&str, Value)>) -> Value {
    let mut m = candidate.as_object().cloned().unwrap_or_default();
    for (k, v) in extra {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}

fn unresolved(candidate: &Value, reason: &str) -> Value {
    spread(
        candidate,
        vec![
            ("status", json!("unresolved")),
            ("confidence", json!("none")),
            ("reason", json!(reason)),
        ],
    )
}

/// JS: index.mjs#analyzeVisualContrastCandidate — everything before the
/// sampling loop.
/// `[n, count]` when `selector` matches `count` elements and `el` is the
/// `n`th, in document order. A generated selector names one element unless
/// the page repeats the id that anchors it (a search box rendered once per
/// breakpoint), and then the first match can be a collapsed copy the
/// candidate never came from. `None` for a selector that names one element.
fn candidate_match(dom: &dyn Dom, selector: &str, el: ElId) -> Option<Value> {
    if !selector.contains('#') {
        return None;
    }
    let matches = dom.query_all(None, selector).ok()?;
    if matches.len() < 2 {
        return None;
    }
    let n = matches.iter().position(|m| *m == el)?;
    Some(json!([n, matches.len()]))
}

/// The element a candidate names: the `n`th match its `match` field records
/// while the page still has that many, the first match otherwise.
pub fn candidate_element(dom: &dyn Dom, candidate: &Value) -> Result<Option<ElId>, ()> {
    let selector = str_or_empty(candidate.get("selector"));
    let identity = candidate.get("match").and_then(Value::as_array).and_then(|a| {
        Some((a.first()?.as_u64()? as usize, a.get(1)?.as_u64()? as usize))
    });
    match identity {
        Some((n, count)) => {
            let matches = dom.query_all(None, &selector).map_err(|_| ())?;
            Ok(if matches.len() == count { matches.get(n).copied() } else { matches.first().copied() })
        }
        None => dom.query_one(None, &selector).map_err(|_| ()),
    }
}

pub fn prepare_analysis(dom: &dyn Dom, candidate: &Value) -> Prepared {
    let el = match candidate_element(dom, candidate) {
        Err(_) => return Prepared::Early { early: unresolved(candidate, "stale selector") },
        Ok(None) => return Prepared::Early { early: unresolved(candidate, "missing element") },
        Ok(Some(el)) => el,
    };
    if !super::element_checks::is_rendered_for_browser_rule(dom, el) {
        return Prepared::Early { early: unresolved(candidate, "hidden element") };
    }
    let reasons: Vec<String> = candidate
        .get("reasons")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| str_or_empty(Some(v))).collect())
        .unwrap_or_default();
    let blocking = reasons.iter().find(|r| {
        matches!(
            r.as_str(),
            "background-clip text" | "blend mode" | "filter" | "backdrop filter" | "opacity stack" | "text shadow"
        )
    });
    if let Some(b) = blocking {
        return Prepared::Early { early: unresolved(candidate, &format!("{b} needs screenshot pixels")) };
    }
    let text_color = parse_rgb_or_any(&dom.style(el, "color"))
        .or_else(|| rgba_from_value(candidate.get("textColor")));
    let Some(text_color) = text_color else {
        return Prepared::Early { early: unresolved(candidate, "unreadable text color") };
    };
    let rect = dom.direct_text_rect(el).unwrap_or_else(|| dom.rect(el));
    if rect.width < 4.0 || rect.height < 4.0 {
        return Prepared::Early { early: unresolved(candidate, "missing text rect") };
    }
    let points = text_sample_points(&rect, dom.inner_width(), dom.inner_height());
    if points.is_empty() {
        return Prepared::Early { early: unresolved(candidate, "text outside viewport") };
    }
    Prepared::Ready {
        el,
        points: points.iter().map(|(x, y)| json!({ "x": x, "y": y })).collect(),
        text_color,
    }
}

/// The ratio the sampled pass stands behind, over its sorted per-point
/// ratios. It is their median, the way the pixel pass reads its glyph cores:
/// the 10th percentile took one point over a bright patch of a photo, or a
/// sample off the image's edge, as the verdict for the whole line (vorelios.com
/// printed 4.0:1 against a median of 8.3:1, climatempo.com.br 4.4:1 against
/// 10.0:1). Where the points read two surfaces, the median more than
/// [`VERDICT_MEDIAN_DIVERGENCE`] times the 10th percentile, the words over the
/// light part are really there and the line is scored at that percentile as
/// before.
pub fn sampled_verdict(sorted: &[f64]) -> f64 {
    let low = percentile(sorted, 10.0);
    let median = percentile(sorted, 50.0);
    if median > low * VERDICT_MEDIAN_DIVERGENCE {
        low
    } else {
        median
    }
}

/// JS: index.mjs#analyzeVisualContrastCandidate — after the sampling loop:
/// `samples` is one `{ status, color?, method?, reason? }` per point.
pub fn finish_analysis(candidate: &Value, text_color: &Rgba, samples: &[Value], points_len: usize) -> Value {
    let mut ratios: Vec<f64> = Vec::new();
    let mut methods: Vec<String> = Vec::new();
    let mut unresolved_reasons: Vec<String> = Vec::new();
    for sample in samples {
        let sampled = sample.get("status").and_then(Value::as_str) == Some("sampled");
        let color = rgba_from_value(sample.get("color"));
        if !sampled || color.is_none() {
            unresolved_reasons.push(str_or_empty(sample.get("reason")));
            continue;
        }
        let bg = color.unwrap();
        let fg = blend_rgba(Some(text_color), Some(&bg)).unwrap();
        ratios.push(contrast_ratio(&fg, &bg));
        let method = str_or_empty(sample.get("method"));
        if !method.is_empty() && !methods.contains(&method) {
            methods.push(method);
        }
    }
    if ratios.len() < math_min(3.0, points_len as f64) as usize {
        let mut uniq: Vec<&str> = Vec::new();
        for r in &unresolved_reasons {
            if !r.is_empty() && !uniq.contains(&r.as_str()) {
                uniq.push(r);
            }
        }
        let joined = uniq.iter().take(3).copied().collect::<Vec<_>>().join(", ");
        let reason = if joined.is_empty() { "not enough readable samples".to_string() } else { joined };
        return spread(
            candidate,
            vec![
                ("status", json!("unresolved")),
                ("confidence", json!("none")),
                ("samples", json!(ratios.len())),
                ("reason", json!(reason)),
            ],
        );
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = ratios.len();
    let median = percentile(&ratios, 50.0);
    let measured = sampled_verdict(&ratios);
    let threshold = candidate.get("threshold").and_then(Value::as_f64).unwrap_or(f64::NAN);
    let status = if measured < threshold { "fail" } else { "pass" };
    let mut sorted_methods = methods.clone();
    sorted_methods.sort();
    let method = {
        let j = sorted_methods.join(", ");
        if j.is_empty() { "browser-visual".to_string() } else { j }
    };
    let text = str_or_empty(candidate.get("text"));
    let text_label = if text.is_empty() { String::new() } else { format!(" \"{text}\"") };
    let detail = format!(
        "browser contrast {}:1 median {}:1 (need {}:1) via {}{}",
        crate::color::ratio_label(measured, threshold),
        crate::color::ratio_label(median, threshold),
        number_to_string(threshold),
        method,
        text_label
    );
    let finding = if status == "fail" {
        json!({ "id": "low-contrast", "snippet": detail })
    } else {
        Value::Null
    };
    spread(
        candidate,
        vec![
            ("status", json!(status)),
            ("confidence", json!(if method.contains("canvas-") { "high" } else { "medium" })),
            ("method", json!(method)),
            ("ratio", json!(measured)),
            ("medianRatio", json!(median)),
            ("samples", json!(n)),
            ("finding", finding),
        ],
    )
}

// ─── the screenshot pixel pass ──────────────────────────────────────────────
//
// `screenshot-contrast` diffs two clipped screenshots of the same box, one
// with the text painted and one with it transparent, and measures every pixel
// the text changed. The decisions below say which of those pixels describe the
// text's own background and when the set is too unlike text to answer at all.

/// One pixel the text paints on: `delta` is the summed channel change between
/// the two frames (how much of the pixel the glyph covers), `ratio` the WCAG
/// ratio measured there, `ground` the luminance left when the text is hidden,
/// `darkened` says the text made that pixel darker than that ground, and
/// `off_color` that what the text painted there is not the color the text
/// declares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphPixel {
    pub delta: f64,
    pub ratio: f64,
    pub ground: f64,
    pub darkened: bool,
    pub off_color: bool,
}

/// How far a well covered pixel may drift from the declared text color,
/// relative to the distance between that color and the ground: a pixel two
/// thirds covered by the glyph sits a third of the way back toward the ground.
const PAINTED_COLOR_DRIFT: f64 = 0.5;

/// Whether the pixel the text painted looks like the color the text declares.
/// It should, wherever the glyph covers most of the pixel: `painted` is that
/// pixel in the frame with the text, `ground` the same pixel in the frame
/// without it. Only worth asking where the pass claims the CSS color as the
/// foreground — `preferRenderedForeground` makes the painted value the
/// foreground, and then there is nothing to check.
pub fn painted_is_text_color(painted: [f64; 3], text: [f64; 3], ground: [f64; 3]) -> bool {
    let drift: f64 = (0..3).map(|i| (painted[i] - text[i]).abs()).sum();
    let span: f64 = (0..3).map(|i| (text[i] - ground[i]).abs()).sum();
    drift <= span * PAINTED_COLOR_DRIFT
}

/// What the pixel pass concluded: a ratio it stands behind, or the reason it
/// could not read the text.
#[derive(Debug, Clone, PartialEq)]
pub enum PixelContrastOutcome {
    Verdict {
        measured: f64,
        median: f64,
        core_pixels: usize,
    },
    Unresolved(&'static str),
}

/// Below this many pixels the diff is noise, not a glyph.
pub const GLYPH_MIN_PIXELS: usize = 8;
/// A pixel is a glyph core at this share of the strongest change in the box:
/// the glyph covers nearly all of it, so what it painted is the text's own
/// color. Anything below it is partly background. An antialiased edge tends
/// to 1:1 no matter how legible the text is, and even a pixel three quarters
/// covered reads well under the color the visitor sees on a dark ground:
/// landio.framer.website's dates measured 3.5:1 over the pixels from 75% up,
/// where the crops read about 4.5:1.
const GLYPH_CORE_COVERAGE: f64 = 0.9;
/// Small or thin text can leave fewer than [`GLYPH_MIN_PIXELS`] cores; the
/// pass then reads the well covered pixels from this share up, as it did
/// before it sampled cores, rather than reporting nothing.
const GLYPH_BODY_COVERAGE: f64 = 0.75;
/// Text covers a fraction of its own box. When most of the clip changed, the
/// page repainted between the two screenshots (a video, a carousel, a reveal
/// animation) and no pixel pair is a glyph over its background.
const GLYPH_CHURN_SHARE: f64 = 0.55;
/// Hiding text moves every pixel it painted the same way: toward the surface
/// under it. A box where a large minority moved the other way holds something
/// that arrived or left between the captures — text mid-animation leaves its
/// old position and its new one in the same diff — so no pair is a glyph over
/// its background.
const GLYPH_DIRECTION_MINORITY: f64 = 0.25;
/// How far the verdict may sit below the median of every measured pixel
/// before the sample is judged bimodal rather than a reading of one surface.
const VERDICT_MEDIAN_DIVERGENCE: f64 = 3.0;
/// How far the surface the glyphs sit on may stand apart from the surface
/// beside them in the same box. A page that paints its own text twice (a
/// duplicate layer behind it for a glow) leaves the second copy where the
/// glyphs were, and the pass would read that copy as the background.
const GROUND_DISAGREEMENT: f64 = 2.0;

/// WCAG ratio between two luminances.
fn luminance_ratio(a: f64, b: f64) -> f64 {
    (math_max(a, b) + 0.05) / (math_min(a, b) + 0.05)
}

/// The sorted-array percentile the visual pass uses everywhere.
fn percentile(sorted: &[f64], pct: f64) -> f64 {
    let n = sorted.len();
    let idx = ((pct / 100.0) * n as f64).floor();
    sorted[math_min((n - 1) as f64, math_max(0.0, idx)) as usize]
}

/// Reasons whose rendered pixels do not say what the text sits on. A filter or
/// backdrop-filter paints the surface from something the diff cannot attribute
/// (the video or image under a glass panel reads as the panel's own ground),
/// and `background-clip: text` paints the glyphs from the background the
/// pass would score them against. Both resolve to unresolved, never to a
/// failure.
pub fn pixel_contrast_blocked(reasons: &[String]) -> Option<String> {
    reasons
        .iter()
        .find(|r| {
            matches!(
                r.as_str(),
                "background-clip text" | "filter" | "backdrop filter"
            )
        })
        .cloned()
}

/// The verdict over the pixels the text painted on. `clip_pixels` is the size
/// of the compared box and `surround_ground` the mean luminance of the pixels
/// in it the text did not touch, when there are enough of them to mean
/// anything.
pub fn pixel_contrast_verdict(
    pixels: &[GlyphPixel],
    clip_pixels: usize,
    surround_ground: Option<f64>,
) -> PixelContrastOutcome {
    if pixels.len() < GLYPH_MIN_PIXELS {
        return PixelContrastOutcome::Unresolved("too few glyph pixels");
    }
    if clip_pixels > 0 && pixels.len() as f64 > clip_pixels as f64 * GLYPH_CHURN_SHARE {
        return PixelContrastOutcome::Unresolved("box repainted between captures");
    }
    let darkened = pixels.iter().filter(|p| p.darkened).count();
    let minority = darkened.min(pixels.len() - darkened);
    if minority as f64 > pixels.len() as f64 * GLYPH_DIRECTION_MINORITY {
        return PixelContrastOutcome::Unresolved("text moved between captures");
    }
    let strongest = pixels.iter().fold(0.0f64, |m, p| math_max(m, p.delta));
    let covered = |share: f64| -> Vec<&GlyphPixel> {
        let floor = strongest * share;
        pixels.iter().filter(|p| p.delta >= floor).collect()
    };
    let mut core_pixels = covered(GLYPH_CORE_COVERAGE);
    if core_pixels.len() < GLYPH_MIN_PIXELS {
        core_pixels = covered(GLYPH_BODY_COVERAGE);
    }
    if core_pixels.len() < GLYPH_MIN_PIXELS {
        return PixelContrastOutcome::Unresolved("too few fully painted glyph pixels");
    }
    if core_pixels.iter().filter(|p| p.off_color).count() * 2 > core_pixels.len() {
        return PixelContrastOutcome::Unresolved("the text is painted by something else");
    }
    if let Some(surround) = surround_ground {
        let under: f64 =
            core_pixels.iter().map(|p| p.ground).sum::<f64>() / core_pixels.len() as f64;
        if luminance_ratio(under, surround) >= GROUND_DISAGREEMENT {
            return PixelContrastOutcome::Unresolved("the hidden text is still painted");
        }
    }
    let mut core: Vec<f64> = core_pixels.iter().map(|p| p.ratio).collect();
    let mut all: Vec<f64> = pixels.iter().map(|p| p.ratio).collect();
    core.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    all.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // The verdict is the median of the glyph cores, so the median the snippet
    // prints is that same number over that same set. It used to be the median
    // over every changed pixel, edges included, which printed a verdict above
    // its own median (`pixel contrast 3.5:1 median 1.7:1`).
    let measured = percentile(&core, 50.0);
    if !measured.is_finite() || measured <= 0.0 {
        return PixelContrastOutcome::Unresolved("no readable glyph pixels");
    }
    // Every changed pixel still answers one question: whether the cores are a
    // reading of one surface or a second population (a repaint, a video band).
    if percentile(&all, 50.0) > measured * VERDICT_MEDIAN_DIVERGENCE {
        return PixelContrastOutcome::Unresolved("verdict disagrees with its own median");
    }
    PixelContrastOutcome::Verdict {
        measured,
        median: measured,
        core_pixels: core.len(),
    }
}

/// JS: analyzeVisualContrast — retry a candidate after scrolling it into view
/// only when the first pass failed for being outside the viewport.
pub fn needs_scroll_retry(result: &Value) -> bool {
    result.get("status").and_then(Value::as_str) == Some("unresolved")
        && result.get("reason").and_then(Value::as_str) == Some("text outside viewport")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    fn rgba(r: f64, g: f64, b: f64, a: f64) -> Rgba {
        Rgba::new(r, g, b, a)
    }

    #[test]
    fn blend_and_worst_color() {
        let fg = rgba(0.0, 0.0, 0.0, 0.5);
        let bg = rgba(255.0, 255.0, 255.0, 1.0);
        let out = blend_rgba(Some(&fg), Some(&bg)).unwrap();
        assert_eq!((out.r, out.g, out.b, out.a), (128.0, 128.0, 128.0, Some(1.0)));
        assert_eq!(blend_rgba(None, Some(&bg)), Some(bg));
        let worst = pick_worst_contrast_color(&rgba(0.0, 0.0, 0.0, 1.0), &[bg, rgba(20.0, 20.0, 20.0, 1.0)]).unwrap();
        assert_eq!(worst.r, 20.0);
        assert!(pick_worst_contrast_color(&bg, &[]).is_none());
    }

    #[test]
    fn position_and_painted_rects() {
        assert_eq!(parse_position_pair(""), ("50%".into(), "50%".into()));
        assert_eq!(parse_position_pair("top"), ("50%".into(), "top".into()));
        assert_eq!(parse_position_pair("left 20px"), ("left".into(), "20px".into()));
        assert_eq!(parse_position_token("right", 100.0, 40.0), 60.0);
        assert_eq!(parse_position_token("25%", 100.0, 40.0), 15.0);
        let c = Box4 { left: 10.0, top: 20.0, width: 200.0, height: 100.0 };
        let p = resolve_painted_image_rect(&c, 400.0, 100.0, "cover", "center");
        assert_eq!((p.width, p.height), (400.0, 100.0));
        assert_eq!(p.left, 10.0 + (200.0 - 400.0) / 2.0);
        let o = resolve_object_image_rect(&c, 50.0, 50.0, "contain", "");
        assert_eq!((o.width, o.height), (100.0, 100.0));
        assert_eq!(point_to_image_source(0.0, 0.0, &p), None);
        assert_eq!(point_to_image_source(110.0, 70.0, &p), Some((200.0, 50.0)));
        assert_eq!(first_css_url("url(\"a b.png\"), url(c.png)"), "a b.png");
        assert_eq!(first_css_url("url( x.png )"), "x.png");
        assert_eq!(get_layer_value("cover, auto", 1), "auto");
    }

    #[test]
    fn sample_points_and_raster() {
        let r = Rect::from_xywh(0.0, 0.0, 100.0, 40.0);
        assert_eq!(text_sample_points(&r, 1280.0, 800.0).len(), 9);
        let r2 = Rect::from_xywh(-50.0, 0.0, 20.0, 10.0);
        assert!(text_sample_points(&r2, 1280.0, 800.0).is_empty());
        let plan = raster_plan(1280.0, 640.0);
        assert_eq!((plan.width, plan.height, plan.scale_x), (640.0, 320.0, 0.5));
        assert_eq!(raster_pixel(&plan, 1279.0, 5.0), (639.0, 2.0));
        assert_eq!(raster_error_reason("Failed: canvas is tainted"), "tainted image");
        assert_eq!(pixel_sample(1.0, 2.0, 3.0, 255.0)["color"]["a"], json!(1.0));
    }

    #[test]
    fn finish_analysis_formats_detail() {
        let candidate = json!({ "selector": "p", "text": "Hello", "threshold": 4.5 });
        let tc = rgba(120.0, 120.0, 120.0, 1.0);
        let samples: Vec<Value> = (0..3)
            .map(|_| json!({ "status": "sampled", "color": { "r": 255, "g": 255, "b": 255, "a": 1 }, "method": "solid-background" }))
            .collect();
        let out = finish_analysis(&candidate, &tc, &samples, 3);
        assert_eq!(out["status"], "fail");
        assert_eq!(out["confidence"], "medium");
        assert_eq!(out["finding"]["snippet"], "browser contrast 4.4:1 median 4.4:1 (need 4.5:1) via solid-background \"Hello\"");
        let out2 = finish_analysis(&candidate, &tc, &samples[..1], 3);
        assert_eq!(out2["status"], "unresolved");
        assert_eq!(out2["reason"], "not enough readable samples");
        assert_eq!(out2["samples"], json!(1));
    }

    #[test]
    fn finish_analysis_never_prints_a_failing_ratio_as_the_bar() {
        // #777777 over #070707 samples 4.498:1. One decimal read 4.5 and two
        // read 4.50, under a `need 4.5:1` it fails.
        let candidate = json!({ "selector": "p", "text": "Plans", "threshold": 4.5 });
        let tc = rgba(119.0, 119.0, 119.0, 1.0);
        let dark: Vec<Value> = (0..3)
            .map(|_| json!({ "status": "sampled", "color": { "r": 7, "g": 7, "b": 7, "a": 1 }, "method": "solid-background" }))
            .collect();
        let out = finish_analysis(&candidate, &tc, &dark, 3);
        assert_eq!(out["status"], "fail");
        assert_eq!(out["finding"]["snippet"], "browser contrast 4.49:1 median 4.49:1 (need 4.5:1) via solid-background \"Plans\"");
        // Four points over black beside three over #070707: one surface, and
        // its median passes where the 10th percentile used to fail.
        let mut mixed = dark.clone();
        for _ in 0..4 {
            mixed.push(json!({ "status": "sampled", "color": { "r": 0, "g": 0, "b": 0, "a": 1 }, "method": "solid-background" }));
        }
        let out = finish_analysis(&candidate, &tc, &mixed, 7);
        assert_eq!(out["status"], "pass", "{out}");
        assert!(out["finding"].is_null(), "{out}");
        // White over a grey band and over black reads two surfaces; the words
        // over the grey keep the low reading, and a median above the bar
        // prints one decimal.
        let white = rgba(255.0, 255.0, 255.0, 1.0);
        let grey = json!({ "status": "sampled", "color": { "r": 119, "g": 119, "b": 119, "a": 1 }, "method": "canvas-img-underlay" });
        let black = json!({ "status": "sampled", "color": { "r": 0, "g": 0, "b": 0, "a": 1 }, "method": "canvas-img-underlay" });
        let banded: Vec<Value> = vec![grey.clone(), grey.clone(), grey, black.clone(), black.clone(), black.clone(), black];
        let out = finish_analysis(&candidate, &white, &banded, 7);
        assert_eq!(out["status"], "fail", "{out}");
        let snippet = out["finding"]["snippet"].as_str().unwrap();
        assert!(snippet.starts_with("browser contrast 4.48:1 median 21.0:1 (need 4.5:1)"), "{snippet}");
    }

    #[test]
    fn the_sampled_verdict_is_the_median_unless_the_points_read_two_surfaces() {
        // vorelios.com: one bright patch under a hero line.
        assert_eq!(sampled_verdict(&[4.0, 7.9, 8.1, 8.3, 8.4, 8.6, 9.0]), 8.3);
        // A line half over a light band and half over a dark one.
        assert_eq!(sampled_verdict(&[2.0, 2.1, 2.2, 12.0, 12.5, 13.0, 13.5]), 2.0);
        assert_eq!(sampled_verdict(&[3.0]), 3.0);
    }

    #[test]
    fn a_frosted_header_whose_own_fill_is_opaque_blocks_nothing() {
        // bookerapp.replit.app: shadcn's `bg-background/95 backdrop-blur`.
        let mut d = FakeDom::new();
        let (_html, body) = d.with_page();
        let wrap = d.add(Some(body), "div");
        d.set_style(wrap, "backgroundImage", "url(\"/booker_bg.jpg\")");
        let header = d.add(Some(wrap), "header");
        d.set_styles(header, &[("backgroundColor", "rgba(22, 24, 29, 0.95)"), ("backdropFilter", "blur(8px)")]);
        let p = d.add(Some(header), "p");
        d.set_style(p, "backgroundColor", "rgba(0, 0, 0, 0)");
        let reasons = collect_visual_contrast_reasons(&d, p);
        assert_eq!(reasons, vec!["backdrop filter under an opaque fill".to_string()]);
        assert!(pixel_contrast_blocked(&reasons).is_none());
        d.set_style(header, "backgroundColor", "rgba(22, 24, 29, 0.5)");
        let reasons = collect_visual_contrast_reasons(&d, p);
        assert!(reasons.iter().any(|r| r == "backdrop filter"), "{reasons:?}");
        assert!(reasons.iter().any(|r| r == "image background"), "{reasons:?}");
        // A contents box's fill ends nothing.
        d.set_styles(header, &[("backgroundColor", "rgb(0, 0, 0)"), ("backdropFilter", "none"), ("display", "contents")]);
        let reasons = collect_visual_contrast_reasons(&d, p);
        assert!(reasons.iter().any(|r| r == "image background"), "{reasons:?}");
    }

    #[test]
    fn stack_walk_pieces() {
        let s = json!({ "status": "sampled", "color": { "r": 1, "g": 2, "b": 3, "a": 0.5 }, "method": "solid-background" });
        assert!(!sample_is_opaque(&s));
        let under = json!({ "status": "sampled", "color": { "r": 255, "g": 255, "b": 255, "a": 1 } });
        let out = alpha_composite(s.clone(), &under);
        assert_eq!(out["method"], "solid-background+alpha");
        assert_eq!(out["color"]["r"].as_f64(), Some(128.0));
        assert_eq!(unresolved_from_reasons(&["a".into(), "".into(), "a".into(), "b".into(), "c".into(), "d".into()])["reason"], "a, b, c");
        assert_eq!(unresolved_from_reasons(&[])["reason"], "no readable visual background");
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = d.add(Some(body), "p");
        d.set_rect(p, 0.0, 0.0, 100.0, 50.0);
        assert!(stack_nodes(&d, p, 10.0, 10.0, 9.0).is_err());
        let nodes = stack_nodes(&d, p, 10.0, 10.0, 0.0).unwrap();
        assert_eq!(nodes[0].el, p);
        assert_eq!(nodes[0].kind, "css");
    }

    /// `n` pixels at one coverage / ratio pair, all darkening their ground.
    fn px(n: usize, delta: f64, ratio: f64) -> Vec<GlyphPixel> {
        vec![GlyphPixel { delta, ratio, ground: 1.0, darkened: true, off_color: false }; n]
    }

    #[test]
    fn pixel_verdict_reads_the_painted_glyph_not_its_edges() {
        // kraflio.com "LinkedIn": white bold 16px on a near-black card. The
        // painted pixels read 18:1; the antialiased edges read near 1:1 and
        // outnumber them, which is how the old tenth percentile reported 1.3:1
        // under a 10.2:1 median.
        let mut pixels = px(40, 705.0, 18.4);
        pixels.extend(px(120, 70.0, 1.1));
        match pixel_contrast_verdict(&pixels, 4000, None) {
            PixelContrastOutcome::Verdict { measured, median, core_pixels } => {
                assert_eq!(core_pixels, 40);
                assert!((measured - 18.4).abs() < 1e-9);
                // The printed median reads the same painted pixels, not the
                // edges the verdict set aside.
                assert!((median - 18.4).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        // aisupply.framer.website collapsed accordion trigger: pale grey on
        // white through an opacity stack. Painted pixels really are 2.2:1.
        let mut faint = px(40, 240.0, 2.2);
        faint.extend(px(90, 60.0, 1.3));
        match pixel_contrast_verdict(&faint, 4000, None) {
            PixelContrastOutcome::Verdict { measured, .. } => assert!((measured - 2.2).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pixel_verdict_reads_glyph_cores_and_never_prints_a_verdict_above_its_median() {
        // landio.framer.website "Access accurate, real-time data": light text
        // through an opacity stack on a near-black card, measured live. The
        // pixels from 90% of the strongest change up read 5.4:1, the band
        // under them 4.2:1 and 3.6:1, and the edges fall toward 1:1. Over the
        // pixels from 75% up the verdict was 4.2:1 (a failure) and the snippet
        // printed a median of 2.3:1 from every changed pixel.
        let mut pixels = px(322, 370.0, 5.4);
        pixels.extend(px(333, 320.0, 4.2));
        pixels.extend(px(220, 290.0, 3.6));
        pixels.extend(px(198, 250.0, 2.8));
        pixels.extend(px(253, 210.0, 2.3));
        pixels.extend(px(186, 160.0, 2.0));
        pixels.extend(px(900, 60.0, 1.3));
        match pixel_contrast_verdict(&pixels, 40000, None) {
            PixelContrastOutcome::Verdict { measured, median, core_pixels } => {
                assert_eq!(core_pixels, 322);
                assert!((measured - 5.4).abs() < 1e-9, "{measured}");
                assert!(measured <= median);
                assert!((median - 5.4).abs() < 1e-9, "{median}");
            }
            other => panic!("{other:?}"),
        }
        // A thin label with only a handful of cores keeps the verdict it had
        // over the well covered pixels instead of going silent.
        let mut thin = px(5, 400.0, 2.4);
        thin.extend(px(30, 320.0, 2.1));
        thin.extend(px(60, 100.0, 1.2));
        match pixel_contrast_verdict(&thin, 4000, None) {
            PixelContrastOutcome::Verdict { measured, median, core_pixels } => {
                assert_eq!(core_pixels, 35);
                assert!((measured - 2.1).abs() < 1e-9, "{measured}");
                assert!(measured <= median);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pixel_verdict_refuses_what_it_cannot_read() {
        assert_eq!(
            pixel_contrast_verdict(&px(4, 700.0, 1.2), 4000, None),
            PixelContrastOutcome::Unresolved("too few glyph pixels")
        );
        // adant.ai "60%": a video band inside the clip changes between the two
        // captures harder than the glyphs do, so the fully painted set is the
        // churn and the median of everything measured disagrees with it.
        let mut churn = px(30, 700.0, 1.4);
        churn.extend(px(200, 300.0, 16.2));
        assert_eq!(
            pixel_contrast_verdict(&churn, 4000, None),
            PixelContrastOutcome::Unresolved("verdict disagrees with its own median")
        );
        // A scroll-reveal mid-fade repaints the whole box, not a glyph.
        assert_eq!(
            pixel_contrast_verdict(&px(900, 120.0, 2.0), 1000, None),
            PixelContrastOutcome::Unresolved("box repainted between captures")
        );
        // Text mid-animation leaves its old position and its new one in the
        // same diff, so half the changed pixels moved the other way.
        let mut moved = px(60, 700.0, 1.2);
        moved.extend(px(60, 700.0, 16.0).into_iter().map(|p| GlyphPixel { darkened: false, ..p }));
        assert_eq!(
            pixel_contrast_verdict(&moved, 4000, None),
            PixelContrastOutcome::Unresolved("text moved between captures")
        );
        // paymentkit.com's hero: the frame without the text still paints it
        // (an animation left a copy behind), so what the glyphs painted is not
        // the color they declare and the diff is text over text.
        let mut ghost = px(40, 120.0, 1.1).into_iter().map(|p| GlyphPixel { off_color: true, ..p }).collect::<Vec<_>>();
        ghost.extend(px(90, 30.0, 1.4));
        assert_eq!(
            pixel_contrast_verdict(&ghost, 4000, None),
            PixelContrastOutcome::Unresolved("the text is painted by something else")
        );
        // paymentkit.com's headline: the frame without the text still holds a
        // dim copy of it, so the surface under the glyphs is nothing like the
        // surface beside them in the same box.
        assert_eq!(
            pixel_contrast_verdict(&px(40, 700.0, 2.6), 4000, Some(0.02)),
            PixelContrastOutcome::Unresolved("the hidden text is still painted")
        );
        // The surface under the glyphs matching the one beside them answers.
        assert!(matches!(
            pixel_contrast_verdict(&px(40, 700.0, 2.6), 4000, Some(0.9)),
            PixelContrastOutcome::Verdict { .. }
        ));
        // The same reading from a glyph that really is its declared color.
        assert!(painted_is_text_color([252.0, 252.0, 252.0], [255.0, 255.0, 255.0], [20.0, 22.0, 28.0]));
        assert!(!painted_is_text_color([52.0, 211.0, 153.0], [205.0, 205.0, 205.0], [160.0, 160.0, 160.0]));
        // One stray full-coverage pixel over a wash of edges: nothing to stand
        // behind.
        let mut sparse = px(1, 700.0, 1.2);
        sparse.extend(px(60, 60.0, 1.1));
        assert_eq!(
            pixel_contrast_verdict(&sparse, 4000, None),
            PixelContrastOutcome::Unresolved("too few fully painted glyph pixels")
        );
    }

    #[test]
    fn blocked_reasons_and_gradient_verdicts() {
        let blocked = ["opacity stack".to_string(), "backdrop filter".to_string()];
        assert_eq!(pixel_contrast_blocked(&blocked).as_deref(), Some("backdrop filter"));
        assert_eq!(
            pixel_contrast_blocked(&["background-clip text".to_string()]).as_deref(),
            Some("background-clip text")
        );
        assert_eq!(pixel_contrast_blocked(&["opacity stack".to_string()]), None);
        let white = rgba(255.0, 255.0, 255.0, 1.0);
        // framai.framer.website glow: the transparent end of the radial
        // gradient is the stop with the worst contrast, and it is not a light
        // grey surface. Nothing here knows where in the box the text sits.
        let glow = [
            rgba(133.0, 38.0, 254.0, 0.82),
            rgba(171.0, 171.0, 171.0, 0.0),
        ];
        assert_eq!(pick_worst_contrast_color(&white, &glow).unwrap(), glow[1]);
        assert_eq!(
            analytic_gradient_verdict(&white, &glow),
            Some(GradientVerdict::Unresolved)
        );
        // overdrive.health `from-primary/10`: a wash of the text's own color
        // over the card, not a slab of it.
        let wash = [rgba(55.0, 65.0, 81.0, 0.1), rgba(55.0, 65.0, 81.0, 0.1)];
        assert_eq!(
            analytic_gradient_verdict(&rgba(55.0, 65.0, 81.0, 1.0), &wash),
            Some(GradientVerdict::Unresolved)
        );
        // A solid color written as a gradient still answers.
        let solid = [rgba(62.0, 69.0, 204.0, 1.0), rgba(62.0, 69.0, 204.0, 1.0)];
        assert_eq!(
            analytic_gradient_verdict(&white, &solid),
            Some(GradientVerdict::Color(solid[0]))
        );
        // Opaque ends far apart: which one is behind the glyphs decides.
        let sweep = [rgba(20.0, 20.0, 24.0, 1.0), rgba(240.0, 240.0, 244.0, 1.0)];
        assert_eq!(
            analytic_gradient_verdict(&white, &sweep),
            Some(GradientVerdict::Unresolved)
        );
        assert_eq!(analytic_gradient_verdict(&white, &[]), None);
    }

    #[test]
    fn the_walk_composites_down_the_stack() {
        let glass = json!({ "status": "sampled", "color": { "r": 255, "g": 255, "b": 255, "a": 0.1 }, "method": "solid-background" });
        let scrim = json!({ "status": "sampled", "color": { "r": 0, "g": 0, "b": 0, "a": 0.5 }, "method": "analytic-gradient" });
        let ground = json!({ "status": "sampled", "color": { "r": 200, "g": 200, "b": 200, "a": 1 }, "method": "canvas-video-underlay" });
        let out = composite_stack(&[glass.clone(), scrim], &ground);
        // 200 under a half-black scrim is 100, and a tenth of white over that
        // is 116 — not the 255 the old self-compositing walk converged on.
        assert_eq!(out["color"]["r"].as_f64(), Some(116.0));
        assert_eq!(out["method"], "solid-background+alpha");
        // Nothing translucent above it leaves the ground as it was.
        assert_eq!(composite_stack(&[], &ground), ground);
        assert!(sample_ends_walk(&json!({ "status": "unresolved", "stop": true })));
        assert!(!sample_ends_walk(&json!({ "status": "unresolved" })));
    }

    #[test]
    fn vector_paint_stops_the_walk() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let avatar = d.add(Some(body), "svg");
        d.set_rect(avatar, 0.0, 0.0, 40.0, 40.0);
        let nodes = stack_nodes(&d, avatar, 10.0, 10.0, 0.0).unwrap();
        assert_eq!(nodes[0].kind, "unreadable");
        assert_eq!(unreadable_stack_sample(&d, avatar)["reason"], "svg paint");
    }

    /// A paragraph in a faded row: a candidate while the row is at rest, and
    /// none while the row is caught mid-reveal, since the pixels would read a
    /// frame of the fade.
    #[test]
    fn a_box_caught_mid_reveal_is_not_read_from_pixels() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        d.set_styles(body, &[("backgroundColor", "rgb(255, 255, 255)"), ("backgroundImage", "none")]);
        let row = d.add(Some(body), "div");
        d.set_styles(row, &[("backgroundColor", "rgba(0, 0, 0, 0)"), ("backgroundImage", "none"), ("opacity", "0.5")]);
        d.set_rect(row, 0.0, 0.0, 400.0, 40.0);
        let p = d.add(Some(row), "p");
        d.add_text(p, "Pick a template:");
        d.set_styles(p, &[("color", "rgb(10, 16, 21)"), ("fontSize", "16px"), ("fontWeight", "400"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("backgroundImage", "none"), ("opacity", "1")]);
        d.set_rect(p, 10.0, 10.0, 200.0, 20.0);
        let cands = collect_visual_contrast_candidates(&d, &json!({}));
        assert_eq!(cands.len(), 1, "{cands:?}");
        assert_eq!(cands[0]["reasons"], json!(["opacity stack"]));

        // Sliding in from under 0.1 opacity.
        d.set_styles(row, &[("opacity", "0.0283007"), ("transform", "matrix(1, 0, 0, 1, -19.1319, 0)")]);
        assert!(collect_visual_contrast_candidates(&d, &json!({})).is_empty());
        // Faded by a running animation.
        d.set_styles(row, &[("opacity", "0.5"), ("transform", "none")]);
        d.set_running_animations(row, &["opacity"]);
        assert!(collect_visual_contrast_candidates(&d, &json!({})).is_empty());
        // The animation settled.
        d.set_running_animations(row, &[]);
        assert_eq!(collect_visual_contrast_candidates(&d, &json!({})).len(), 1);
    }

    #[test]
    fn candidates_and_prepare() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let sec = d.add(Some(body), "section");
        d.set_styles(sec, &[("backgroundImage", "linear-gradient(red, blue)"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("opacity", "1")]);
        d.set_rect(sec, 0.0, 0.0, 400.0, 200.0);
        let p = d.add(Some(sec), "p");
        d.add_text(p, "Hello world");
        d.set_styles(p, &[("color", "rgb(10, 10, 10)"), ("fontSize", "16px"), ("fontWeight", "400"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("backgroundImage", "none"), ("opacity", "1")]);
        d.set_rect(p, 10.0, 10.0, 200.0, 20.0);
        let cands = collect_visual_contrast_candidates(&d, &json!({}));
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        assert_eq!(c["reasons"], json!(["gradient background"]));
        assert_eq!(c["threshold"], json!(4.5));
        assert_eq!(c["clip"], json!({ "x": 8.0, "y": 8.0, "width": 204.0, "height": 24.0 }));
        assert_eq!(c["preferRenderedForeground"], json!(false));
        assert!(collect_visual_contrast_candidates(&d, &json!({ "imageOnly": true })).is_empty());
        let keys: Vec<&String> = c.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["selector", "tagName", "text", "threshold", "reasons", "clip", "textColor", "preferRenderedForeground", "backgroundClipText"]);
        match prepare_analysis(&d, c) {
            Prepared::Ready { el, points, .. } => {
                assert_eq!(el, p);
                assert_eq!(points.len(), 3);
            }
            other => panic!("{other:?}"),
        }
        let blocked = json!({ "selector": "p", "reasons": ["opacity stack"] });
        match prepare_analysis(&d, &blocked) {
            Prepared::Early { early } => assert_eq!(early["reason"], "opacity stack needs screenshot pixels"),
            _ => panic!(),
        }
    }

    fn text_run(d: &mut FakeDom, parent: ElId, tag: &str, text: &str, rect: (f64, f64, f64, f64)) -> ElId {
        let el = d.add(Some(parent), tag);
        d.add_text(el, text);
        d.set_styles(
            el,
            &[
                ("color", "rgb(250, 250, 250)"),
                ("fontSize", "16px"),
                ("fontWeight", "400"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("backgroundImage", "none"),
                ("opacity", "1"),
            ],
        );
        d.set_rect(el, rect.0, rect.1, rect.2, rect.3);
        el
    }

    fn candidate_texts(d: &FakeDom) -> Vec<String> {
        collect_visual_contrast_candidates(d, &json!({ "maxCandidates": 50 }))
            .iter()
            .map(|c| c["text"].as_str().unwrap_or("").to_string())
            .collect()
    }

    #[test]
    fn the_pass_takes_only_text_a_reader_sees() {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 390.0, 4000.0);
        d.set_rect(body, 0.0, 0.0, 390.0, 4000.0);
        d.el_mut(html).scroll_width = 390.0;
        let section = d.add(Some(body), "section");
        d.set_styles(
            section,
            &[
                ("backgroundImage", "linear-gradient(rgb(230, 226, 216), rgb(217, 212, 200))"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("opacity", "1"),
            ],
        );
        d.set_rect(section, 0.0, 0.0, 390.0, 900.0);
        text_run(&mut d, section, "p", "Visible copy", (10.0, 10.0, 200.0, 20.0));

        // A tab strip that clips its row at 380px: the second cell sits at
        // x 600, which only a scroll the page never makes would show.
        let strip = d.add(Some(section), "div");
        d.set_styles(
            strip,
            &[
                ("overflowX", "hidden"),
                ("overflowY", "hidden"),
                ("backgroundImage", "none"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("opacity", "1"),
            ],
        );
        d.set_rect(strip, 10.0, 100.0, 370.0, 60.0);
        text_run(&mut d, strip, "p", "First cell", (36.0, 110.0, 248.0, 40.0));
        text_run(&mut d, strip, "p", "Past the strip", (600.0, 110.0, 248.0, 40.0));

        // Two faded controls with a fill of their own; the disabled one is
        // exempt, as it is in the element pass.
        for (label, top, disabled) in [("Continue", 200.0, true), ("Submit", 260.0, false)] {
            let button = text_run(&mut d, section, "button", label, (10.0, top, 200.0, 40.0));
            d.set_styles(button, &[("backgroundColor", "rgb(55, 65, 81)"), ("opacity", "0.5")]);
            if disabled {
                d.add_selector(button, "[disabled]");
            }
        }

        // A launcher whose label is set in 0px transparent ink over its own
        // gradient.
        let launcher = text_run(&mut d, section, "button", "ASK FEDEX", (284.0, 320.0, 80.0, 56.0));
        d.set_styles(
            launcher,
            &[
                ("backgroundColor", "rgb(77, 20, 140)"),
                ("backgroundImage", "linear-gradient(270deg, rgb(77, 20, 140) 0%, rgb(77, 20, 140) 100%)"),
                ("fontSize", "0px"),
                ("color", "rgba(0, 0, 0, 0)"),
            ],
        );

        // A lone circle, and copy filled with nothing.
        text_run(&mut d, section, "div", "\u{25EF}", (10.0, 400.0, 52.0, 84.0));
        let unfilled = text_run(&mut d, section, "p", "Filled with nothing", (10.0, 500.0, 200.0, 20.0));
        d.set_style(unfilled, "webkitTextFillColor", "rgba(0, 0, 0, 0)");

        // A heading that paints its own gradient into its glyphs keeps its
        // slot: both passes refuse it later, as before.
        let heading = text_run(&mut d, section, "h2", "Gradient heading", (10.0, 560.0, 300.0, 40.0));
        d.set_styles(
            heading,
            &[
                ("webkitBackgroundClip", "text"),
                ("backgroundImage", "linear-gradient(90deg, rgb(62, 69, 204), rgb(133, 38, 254))"),
                ("color", "rgba(0, 0, 0, 0)"),
                ("webkitTextFillColor", "rgba(0, 0, 0, 0)"),
            ],
        );

        assert_eq!(
            candidate_texts(&d),
            vec!["Visible copy", "First cell", "Submit", "Gradient heading"]
        );
    }

    #[test]
    fn hidden_candidates_no_longer_spend_the_budget() {
        let mut d = FakeDom::new();
        let (html, body) = d.with_page();
        d.set_rect(html, 0.0, 0.0, 1280.0, 2000.0);
        d.el_mut(html).scroll_width = 1280.0;
        let track = d.add(Some(body), "div");
        d.set_styles(
            track,
            &[
                ("overflowX", "hidden"),
                ("overflowY", "hidden"),
                ("backgroundImage", "url(\"https://example.com/photo.jpg\")"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("opacity", "1"),
            ],
        );
        d.set_rect(track, 0.0, 0.0, 600.0, 300.0);
        for i in 0..12 {
            text_run(&mut d, track, "p", "Parked slide", (700.0 + 600.0 * i as f64, 20.0, 200.0, 20.0));
        }
        text_run(&mut d, track, "p", "Active slide", (20.0, 20.0, 200.0, 20.0));
        assert_eq!(
            collect_visual_contrast_candidates(&d, &json!({}))
                .iter()
                .map(|c| c["text"].as_str().unwrap_or(""))
                .collect::<Vec<_>>(),
            vec!["Active slide"]
        );
    }

    #[test]
    fn an_image_that_covers_neither_the_text_nor_its_box_is_not_the_surface() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();

        // A 32px badge carrying a 22x16 frame sprite, the count centred on
        // it: the sprite covers the glyphs, not the badge.
        let badge = d.add(Some(body), "span");
        d.set_rect(badge, 200.0, 100.0, 32.0, 32.0);
        d.set_style(
            badge,
            "background",
            "rgba(0, 0, 0, 0.6) url(\"https://example.com/frame.svg\") no-repeat scroll 50% 50% / 22px 16px padding-box border-box",
        );
        let count = d.add(Some(badge), "i");
        d.add_text(count, "12");
        d.set_rect(count, 210.0, 107.5, 12.0, 17.0);
        d.set_text_rect(count, 211.0, 108.5, 10.0, 12.0);
        let sample =
            css_url_source_point(&d, badge, count, 30.0, 22.0, "22px 16px", "50% 50%", 216.0, 114.0)
                .unwrap_err();
        assert_eq!(sample["reason"], "background image does not cover the text");
        assert!(sample_ends_walk(&sample));
        // An image this pass could not load is still painted on the page, and
        // its stated placement says the same without its pixels.
        assert!(sample_ends_walk(&css_url_no_image(&d, badge, count, "22px 16px", "50% 50%")));

        // A 16px mark beside a button's label: the label is not over it.
        let button = d.add(Some(body), "button");
        d.set_rect(button, 0.0, 500.0, 200.0, 40.0);
        d.set_style(
            button,
            "background",
            "rgb(31, 41, 55) url(\"data:image/svg+xml,x\") no-repeat scroll 12px 50% / 16px 16px padding-box border-box",
        );
        d.add_text(button, "Download");
        d.set_text_rect(button, 40.0, 510.0, 80.0, 20.0);
        let sample =
            css_url_source_point(&d, button, button, 16.0, 16.0, "16px 16px", "12px 50%", 60.0, 520.0)
                .unwrap_err();
        assert!(sample_ends_walk(&sample));

        // A photo drawn to cover its card is the surface.
        let card = d.add(Some(body), "div");
        d.set_rect(card, 0.0, 200.0, 400.0, 240.0);
        d.set_style(
            card,
            "background",
            "rgba(0, 0, 0, 0) url(\"https://example.com/photo.jpg\") no-repeat scroll 50% 50% / cover padding-box border-box",
        );
        let caption = d.add(Some(card), "p");
        d.add_text(caption, "Caption");
        d.set_rect(caption, 20.0, 380.0, 200.0, 24.0);
        assert!(css_url_source_point(&d, card, caption, 1600.0, 900.0, "cover", "50% 50%", 60.0, 390.0).is_ok());
        assert_eq!(css_url_no_image(&d, card, caption, "cover", "50% 50%")["reason"], "image unavailable");

        // A repeating tile covers what it paints; outside its first tile the
        // point is unresolved and the walk goes on, as before.
        let tiled = d.add(Some(body), "div");
        d.set_rect(tiled, 0.0, 600.0, 400.0, 200.0);
        d.set_style(
            tiled,
            "background",
            "rgb(20, 20, 20) url(\"https://example.com/noise.png\") repeat scroll 0% 0% / 64px 64px padding-box border-box",
        );
        let words = d.add(Some(tiled), "p");
        d.add_text(words, "Words");
        d.set_rect(words, 20.0, 620.0, 300.0, 24.0);
        assert!(css_url_source_point(&d, tiled, words, 64.0, 64.0, "64px 64px", "0% 0%", 30.0, 630.0).is_ok());
        let outside =
            css_url_source_point(&d, tiled, words, 64.0, 64.0, "64px 64px", "0% 0%", 200.0, 630.0)
                .unwrap_err();
        assert!(!sample_ends_walk(&outside));

        // What the capture cannot place is read as before: no shorthand
        // recorded, or a `contain` size with no intrinsic size loaded.
        let bare = d.add(Some(body), "span");
        d.set_rect(bare, 200.0, 100.0, 32.0, 32.0);
        let unplaced = css_url_source_point(&d, bare, count, 30.0, 22.0, "22px 16px", "50% 50%", 216.0, 114.0);
        assert!(unplaced.map_or_else(|s| !sample_ends_walk(&s), |_| true));
        d.set_style(bare, "background", "rgba(0, 0, 0, 0) url(\"x.svg\") no-repeat scroll 50% 50% / contain padding-box border-box");
        let no_size = css_url_source_point(&d, bare, count, 0.0, 0.0, "contain", "50% 50%", 216.0, 114.0);
        assert!(no_size.map_or_else(|s| !sample_ends_walk(&s), |_| true));
    }

    #[test]
    fn repeat_and_size_are_read_where_the_capture_states_them() {
        assert_eq!(
            background_repeat_axes("rgba(0, 0, 0, 0.6) url(\"a,no-repeat.svg\") no-repeat scroll 50% 50% / 22px 16px padding-box border-box"),
            Some((false, false))
        );
        assert_eq!(
            background_repeat_axes("url(\"a.png\") repeat-x scroll 0% 0% / auto padding-box border-box, rgb(255, 255, 255) url(\"b.png\") no-repeat scroll 0% 0% / auto padding-box border-box"),
            Some((true, false))
        );
        assert_eq!(
            background_repeat_axes("rgb(255, 255, 255) none repeat scroll 0% 0% / auto padding-box border-box"),
            Some((true, true))
        );
        assert_eq!(background_repeat_axes("url(x.png) space no-repeat"), Some((true, false)));
        assert_eq!(background_repeat_axes(""), None);
        assert!(placed_size_is_known("22px 16px", 0.0, 0.0));
        assert!(placed_size_is_known("cover", 0.0, 0.0));
        assert!(!placed_size_is_known("contain", 0.0, 0.0));
        assert!(placed_size_is_known("contain", 300.0, 200.0));
        assert!(!placed_size_is_known("auto 16px", 0.0, 0.0));
        assert!(!placed_size_is_known("calc(50% + 10px) auto", 300.0, 200.0));
        assert!(!placed_size_is_known("10em 2em", 300.0, 200.0));
    }

    /// Text a fixed layer covers at capture is no candidate: the element pass
    /// stands down on it, and a sampled or pixel verdict would score it anyway.
    #[test]
    fn covered_text_is_no_candidate() {
        let build = |banner_opacity: &str| {
            let mut d = FakeDom::new();
            let (_h, body) = d.with_page();
            let sec = d.add(Some(body), "section");
            d.set_styles(sec, &[("backgroundImage", "linear-gradient(red, blue)"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("opacity", "1")]);
            d.set_rect(sec, 0.0, 600.0, 1280.0, 200.0);
            let p = d.add(Some(sec), "p");
            d.add_text(p, "Team size");
            d.set_styles(p, &[("color", "rgb(250, 240, 250)"), ("fontSize", "14px"), ("fontWeight", "400"), ("backgroundColor", "rgba(0, 0, 0, 0)"), ("backgroundImage", "none"), ("opacity", "1")]);
            d.set_rect(p, 16.0, 735.0, 184.0, 20.0);
            let banner = d.add(Some(body), "div");
            d.set_styles(banner, &[("position", "fixed"), ("backgroundColor", "rgba(255, 255, 255, 0.95)"), ("backgroundImage", "none"), ("opacity", banner_opacity)]);
            d.set_rect(banner, 0.0, 645.0, 1280.0, 155.0);
            collect_visual_contrast_candidates(&d, &json!({}))
        };
        assert!(build("1").is_empty(), "{:?}", build("1"));
        assert_eq!(build("0.5").len(), 1, "a translucent banner shows the text");
    }
}
