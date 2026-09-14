//! Where a block's text is, for the rules that measure text geometry: the
//! extent of its glyphs, the pitch of its line boxes, and the clipping boxes
//! that cut it.
//!
//! A paragraph's border box spans its column whatever its text does: a
//! one-line note in a 1,200px box, a centred footer line, a sentence ended
//! early by a `<br>`. `line-length`, `body-text-viewport-edge`, `tight-leading`,
//! `cramped-padding` and `text-overflow` ask what a reader meets, so they read
//! the union of the Range client rects of the text ([`Dom::direct_text_rect`])
//! rather than the box. A Dom that cannot measure text answers `None` here,
//! and each rule then keeps the box it read before.

use super::dom::{tag_lower, Dom, ElId, Rect};
use crate::checks::measures::resolve_length_px;
use crate::checks::text_rules::NON_RENDERED_TAGS;
use crate::js::{self, parse_float};

/// Phrasing content: the tags whose text flows in the line boxes of the block
/// around them.
pub const PHRASING_TAGS: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "br", "cite", "code", "data", "del", "dfn", "em", "font", "i",
    "ins", "kbd", "mark", "q", "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u",
    "var", "wbr",
];

/// `overflow-x`, from the shorthand's first value when the capture recorded
/// no longhand. `overflow-x: hidden` computes the shorthand to `hidden auto`,
/// so the shorthand says nothing about the x axis on its own.
pub fn overflow_x(dom: &dyn Dom, el: ElId) -> String {
    let x = dom.style(el, "overflowX");
    if !x.is_empty() {
        return x;
    }
    dom.style(el, "overflow").split_whitespace().next().unwrap_or("").to_string()
}

/// Whether `el` clips or scrolls its content on the x axis.
pub fn clips_x(dom: &dyn Dom, el: ElId) -> bool {
    matches!(overflow_x(dom, el).as_str(), "hidden" | "clip" | "auto" | "scroll")
}

/// An element that generates no box and paints nothing: a script inside a
/// paragraph, a `display: none` twin.
fn paints_nothing(dom: &dyn Dom, el: ElId) -> bool {
    NON_RENDERED_TAGS.contains(&tag_lower(dom, el).as_str()) || dom.style(el, "display") == "none"
}

/// A phrasing tag laid out inline. A capture that recorded no display counts
/// by its tag.
fn is_inline_phrasing(dom: &dyn Dom, el: ElId) -> bool {
    PHRASING_TAGS.contains(&tag_lower(dom, el).as_str())
        && matches!(dom.style(el, "display").as_str(), "inline" | "")
}

/// Whether everything under `el` is inline phrasing content (or paints
/// nothing), so all of its text runs in `el`'s own line boxes: a paragraph
/// whose words sit in `<b>`, `<i>` or `<span>`.
pub fn holds_only_phrasing(dom: &dyn Dom, el: ElId) -> bool {
    dom.children(el)
        .into_iter()
        .all(|c| paints_nothing(dom, c) || (is_inline_phrasing(dom, c) && holds_only_phrasing(dom, c)))
}

fn usable(r: &Rect) -> bool {
    r.all_finite() && r.width > 0.0 && r.height > 0.0
}

fn union(a: Option<Rect>, b: Rect) -> Rect {
    match a {
        None => b,
        Some(a) => {
            let left = js::math_min(a.left, b.left);
            let top = js::math_min(a.top, b.top);
            let right = js::math_max(a.right, b.right);
            let bottom = js::math_max(a.bottom, b.bottom);
            Rect::from_xywh(left, top, right - left, bottom - top)
        }
    }
}

fn extend_phrasing(dom: &dyn Dom, el: ElId, acc: &mut Option<Rect>) {
    if let Some(t) = dom.direct_text_rect(el) {
        if usable(&t) {
            *acc = Some(union(*acc, t));
        }
    }
    for c in dom.children(el) {
        if !paints_nothing(dom, c) && is_inline_phrasing(dom, c) {
            extend_phrasing(dom, c, acc);
        }
    }
}

/// The extent of the text `el` sets in its own line boxes: the union of the
/// text rects of its own text and of the inline phrasing elements under it.
/// `None` when none of it can be measured.
pub fn phrasing_text_extent(dom: &dyn Dom, el: ElId) -> Option<Rect> {
    let mut acc = None;
    extend_phrasing(dom, el, &mut acc);
    acc
}

/// The line-height `el`'s text is set at, for an element with its own
/// resolved `own` line-height. An inline element's line boxes belong to the
/// block around it, whose strut sets the pitch when it is taller than the
/// inline's own: an 11px link run at `line-height: 11px` inside a 14px block
/// lands on 14px lines. A block whose line-height cannot be resolved
/// (`normal`) leaves the element's own value, as before.
pub fn line_pitch_px(dom: &dyn Dom, el: ElId, own: f64) -> f64 {
    if dom.style(el, "display") != "inline" {
        return own;
    }
    let mut cur = dom.parent(el);
    while let Some(p) = cur {
        let display = dom.style(p, "display");
        if display != "inline" && display != "contents" {
            let font_size = parse_float(&dom.style(p, "fontSize"));
            if !(font_size.is_finite() && font_size > 0.0) {
                return own;
            }
            return match resolve_length_px(Some(&dom.style(p, "lineHeight")), font_size) {
                Some(block) if block.is_finite() && block > own => block,
                _ => own,
            };
        }
        cur = dom.parent(p);
    }
    own
}

/// Whether a box that clips on the x axis, between `el` and the page root,
/// cuts `text` at one of its sides: a slide in a carousel track, a paragraph
/// in a horizontal scroller. What shows near the viewport edge there is the
/// track's clip, not the page's gutter. The root and body stand for the
/// viewport and are not counted.
pub fn clipping_ancestor_cuts(dom: &dyn Dom, el: ElId, text: &Rect) -> bool {
    let root = dom.document_element();
    let body = dom.body();
    let mut cur = dom.parent(el);
    while let Some(p) = cur {
        if Some(p) == root || Some(p) == body {
            break;
        }
        if clips_x(dom, p) {
            let cr = dom.rect(p);
            if cr.all_finite() && (text.left < cr.left - 1.0 || text.right > cr.right + 1.0) {
                return true;
            }
        }
        cur = dom.parent(p);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::fake_dom::FakeDom;

    #[test]
    fn phrasing_extent_unions_inline_children_and_stops_at_blocks() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = d.add(Some(body), "p");
        d.set_style(p, "display", "block");
        let b = d.add(Some(p), "b");
        d.set_style(b, "display", "inline");
        d.add_text(b, "Bold opening words");
        d.set_text_rect(b, 40.0, 100.0, 300.0, 18.0);
        let i = d.add(Some(p), "i");
        d.set_style(i, "display", "inline");
        d.add_text(i, "and an italic run that wraps");
        d.set_text_rect(i, 20.0, 100.0, 600.0, 42.0);
        assert!(holds_only_phrasing(&d, p));
        let t = phrasing_text_extent(&d, p).expect("extent");
        assert_eq!((t.left, t.top, t.right, t.bottom), (20.0, 100.0, 620.0, 142.0));

        // A block child is not phrasing: the paragraph holds a component.
        let card = d.add(Some(p), "div");
        d.set_style(card, "display", "block");
        d.add_text(card, "card body");
        d.set_text_rect(card, 0.0, 200.0, 1000.0, 18.0);
        assert!(!holds_only_phrasing(&d, p));
        let t = phrasing_text_extent(&d, p).expect("extent");
        assert_eq!(t.right, 620.0, "the block child's text is not the paragraph's line");

        // Nothing measurable.
        let bare = d.add(Some(body), "p");
        let s = d.add(Some(bare), "span");
        d.add_text(s, "unmeasured");
        assert!(phrasing_text_extent(&d, bare).is_none());
    }

    #[test]
    fn overflow_x_reads_the_longhand_first() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let main = d.add(Some(body), "main");
        d.set_styles(main, &[("overflow", "hidden auto"), ("overflowX", "hidden")]);
        assert_eq!(overflow_x(&d, main), "hidden");
        let legacy = d.add(Some(body), "div");
        d.set_style(legacy, "overflow", "auto scroll");
        assert_eq!(overflow_x(&d, legacy), "auto");
    }

    #[test]
    fn an_inline_run_sits_on_the_block_strut() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let block = d.add(Some(body), "div");
        d.set_styles(block, &[("display", "inline-block"), ("fontSize", "14px"), ("lineHeight", "14px")]);
        let span = d.add(Some(block), "span");
        d.set_styles(span, &[("display", "inline"), ("fontSize", "11px"), ("lineHeight", "11px")]);
        assert_eq!(line_pitch_px(&d, span, 11.0), 14.0);
        // A taller own line-height sets the pitch itself.
        assert_eq!(line_pitch_px(&d, span, 20.0), 20.0);
        // An unresolvable block strut leaves the own value.
        d.set_style(block, "lineHeight", "normal");
        assert_eq!(line_pitch_px(&d, span, 11.0), 11.0);
        // A block element is its own strut.
        d.set_style(span, "display", "block");
        assert_eq!(line_pitch_px(&d, span, 11.0), 11.0);
    }
}
