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

/// Whether a horizontal scroller between `el` and the page root cuts `text`
/// at one of its sides: a slide in a track the visitor swipes, a paragraph in
/// a scrolled table. Scrolling brings that text into view, so what shows near
/// the viewport edge there is the track's clip, not the page's gutter.
///
/// Only a box that really scrolls on x counts: `overflow-x: auto` or `scroll`
/// with a `scrollWidth` past its `clientWidth`, the scroller the painted
/// predicate lets bring content into its box. A box that only hides its
/// overflow (a Tailwind `overflow-x-hidden` page wrapper, a section or card at
/// `overflow: hidden`) cannot tell a carousel track from text cut off by a
/// layout bug, and the text stays reported. A metric the capture did not
/// record proves no scroller either. The root and body stand for the viewport
/// and are not counted.
pub fn scrolling_ancestor_cuts(dom: &dyn Dom, el: ElId, text: &Rect) -> bool {
    let root = dom.document_element();
    let body = dom.body();
    let mut cur = dom.parent(el);
    while let Some(p) = cur {
        if Some(p) == root || Some(p) == body {
            break;
        }
        if scrolls_x(dom, p) {
            let cr = dom.rect(p);
            if cr.all_finite() && (text.left < cr.left - 1.0 || text.right > cr.right + 1.0) {
                return true;
            }
        }
        cur = dom.parent(p);
    }
    false
}

/// Whether `el` scrolls on the x axis and has content to scroll to.
fn scrolls_x(dom: &dyn Dom, el: ElId) -> bool {
    if !matches!(overflow_x(dom, el).as_str(), "auto" | "scroll") {
        return false;
    }
    let (scroll, client) = (dom.scroll_width(el), dom.client_width(el));
    scroll.is_finite() && client.is_finite() && scroll > client + 1.0
}

/// The height of one line's content area in ems: the font's ascent plus
/// descent, what a Range client rect spans for one line whatever the
/// line-height. About 1.2em for most text faces (1.1 to 1.5 across them).
const CONTENT_AREA_EM: f64 = 1.2;

/// How many line boxes a text rect `text_height` tall spans, for text set at
/// `font_size` on lines `pitch` apart. The union of a block's Range client
/// rects runs from the top of its first line's content area to the bottom of
/// its last's: one pitch per line after the first, plus one content area.
/// Dividing the height by the pitch alone reads two lines at
/// `line-height: 2.4` as one, since that height is short of 1.5 pitches.
pub fn text_line_count(text_height: f64, pitch: f64, font_size: f64) -> f64 {
    if !(text_height.is_finite() && pitch.is_finite() && pitch > 0.0) {
        return 1.0;
    }
    let content = if font_size.is_finite() && font_size > 0.0 {
        font_size * CONTENT_AREA_EM
    } else {
        0.0
    };
    let after_first = js::math_round((text_height - content) / pitch);
    1.0 + if after_first > 0.0 { after_first } else { 0.0 }
}

fn holds_break_in(dom: &dyn Dom, el: ElId) -> bool {
    dom.children(el).into_iter().any(|c| {
        !paints_nothing(dom, c)
            && is_inline_phrasing(dom, c)
            && (tag_lower(dom, c) == "br" || holds_break_in(dom, c))
    })
}

/// Whether a `<br>` ends a line among the text `el` sets in its own line
/// boxes, so its lines stop short of the box where the author broke them.
pub fn phrasing_holds_break(dom: &dyn Dom, el: ElId) -> bool {
    holds_break_in(dom, el)
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

    /// review of observations-20 row 31: v0-optimus-delta.vercel.app's
    /// `section.overflow-hidden` and simplybudget.framer.ai's card cut text at
    /// the viewport edge, and hid it from body-text-viewport-edge. Only a box
    /// that really scrolls on x is a track.
    #[test]
    fn only_a_real_x_scroller_cuts_text_off_the_page() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let section = d.add(Some(body), "section");
        d.set_styles(section, &[("overflow", "hidden"), ("overflowX", "hidden")]);
        d.set_rect(section, 0.0, 0.0, 390.0, 400.0);
        d.el_mut(section).client_width = 390.0;
        d.el_mut(section).scroll_width = 451.0;
        let p = d.add(Some(section), "p");
        let cut = Rect::from_xywh(56.0, 20.0, 353.0, 48.0);
        assert!(!scrolling_ancestor_cuts(&d, p, &cut), "a box that only hides overflow");
        d.set_styles(section, &[("overflow", "hidden auto"), ("overflowX", "hidden")]);
        assert!(!scrolling_ancestor_cuts(&d, p, &cut), "overflow-x-hidden");

        // nike.com's `ul.slider`: auto on x, with slides to scroll to.
        d.set_styles(section, &[("overflow", "auto"), ("overflowX", "auto")]);
        assert!(scrolling_ancestor_cuts(&d, p, &cut), "a swiped track");
        // With nothing to scroll to, it is no track.
        d.el_mut(section).scroll_width = 390.0;
        assert!(!scrolling_ancestor_cuts(&d, p, &cut));
        // Nor with an unrecorded metric.
        d.el_mut(section).scroll_width = f64::NAN;
        assert!(!scrolling_ancestor_cuts(&d, p, &cut));
        // A scroller the text sits inside does not cut it.
        d.el_mut(section).scroll_width = 948.0;
        let inside = Rect::from_xywh(24.0, 20.0, 300.0, 48.0);
        assert!(!scrolling_ancestor_cuts(&d, p, &inside));
        // The root scrolls the page, and stands for the viewport.
        d.set_styles(section, &[("overflow", "visible"), ("overflowX", "visible")]);
        let html = d.document_element().expect("root");
        d.set_styles(html, &[("overflow", "auto"), ("overflowX", "auto")]);
        d.set_rect(html, 0.0, 0.0, 390.0, 900.0);
        d.el_mut(html).client_width = 390.0;
        d.el_mut(html).scroll_width = 600.0;
        assert!(!scrolling_ancestor_cuts(&d, p, &cut));
    }

    /// A Range rect spans one content area plus a pitch per extra line.
    #[test]
    fn line_count_reads_the_content_area_and_the_pitch() {
        // 16px text on 24px lines: one line is a ~19px rect, two ~43px.
        assert_eq!(text_line_count(19.0, 24.0, 16.0), 1.0);
        assert_eq!(text_line_count(43.0, 24.0, 16.0), 2.0);
        assert_eq!(text_line_count(91.0, 24.0, 16.0), 4.0);
        // prose.html `pl-lh`: Georgia at 16px on 38.4px lines. Two lines are
        // 38.4 + 18.2 = 56.6px, under 1.5 pitches (57.6px).
        assert_eq!(text_line_count(18.2, 38.4, 16.0), 1.0);
        assert_eq!(text_line_count(56.6, 38.4, 16.0), 2.0);
        // A tall face (a 1.5em content area) on tight 16px lines is one line.
        assert_eq!(text_line_count(24.0, 16.0, 16.0), 1.0);
        // veeza.ai 106327: DM Sans at 16px on 26px lines, a 47px rect.
        assert_eq!(text_line_count(47.0, 26.0, 16.0), 2.0);
        // Nothing to divide by.
        assert_eq!(text_line_count(47.0, 0.0, 16.0), 1.0);
        assert_eq!(text_line_count(f64::NAN, 26.0, 16.0), 1.0);
    }

    #[test]
    fn a_break_among_the_phrasing_ends_a_line() {
        let mut d = FakeDom::new();
        let (_h, body) = d.with_page();
        let p = d.add(Some(body), "p");
        d.add_text(p, "A closing statement,");
        assert!(!phrasing_holds_break(&d, p));
        let strong = d.add(Some(p), "strong");
        d.set_style(strong, "display", "inline");
        let br = d.add(Some(strong), "br");
        d.set_style(br, "display", "inline");
        assert!(phrasing_holds_break(&d, p), "a break inside a bold run");
        // A break inside a block child ends that block's line, not these.
        let q = d.add(Some(body), "p");
        let card = d.add(Some(q), "div");
        d.set_style(card, "display", "block");
        let inner = d.add(Some(card), "br");
        d.set_style(inner, "display", "inline");
        assert!(!phrasing_holds_break(&d, q));
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
