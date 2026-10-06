//! `heading-rhythm` over hand-built page geometry.
//!
//! Every page carries two plainly crowded headings, so the rule's two-heading
//! minimum is met and a shape that should pass shows up as a third finding if
//! it misfires. The shapes come from a live recapture of real sites, judged
//! once the rule pass ran after the reveal sweep and the check started seeing
//! sections that used to sit at opacity 0.

use impeccable_core::browser::fake_dom::FakeDom;
use impeccable_core::browser::page_checks::check_heading_rhythm_dom;
use impeccable_core::browser::ElId;

const W: f64 = 800.0;
const LONG: &str = "A closing paragraph of the previous block of content that runs well past eighty characters.";

struct Page {
    d: FakeDom,
    body: ElId,
    y: f64,
}

impl Page {
    fn new() -> Page {
        let mut d = FakeDom::new();
        let (_html, body) = d.with_page();
        Page { d, body, y: 0.0 }
    }

    /// An element in normal flow with neutral styles, then `styles` on top.
    fn el(&mut self, parent: ElId, tag: &str, rect: (f64, f64, f64, f64), styles: &[(&str, &str)], text: &str) -> ElId {
        let el = self.d.add(Some(parent), tag);
        self.d.set_styles(
            el,
            &[
                ("display", "block"),
                ("visibility", "visible"),
                ("opacity", "1"),
                ("position", "static"),
                ("fontSize", "16px"),
                ("backgroundColor", "rgba(0, 0, 0, 0)"),
                ("borderTopWidth", "0px"),
                ("borderBottomWidth", "0px"),
                ("boxShadow", "none"),
                ("marginBottom", "0px"),
            ],
        );
        self.d.set_styles(el, styles);
        self.d.set_rect(el, rect.0, rect.1, rect.2, rect.3);
        if !text.is_empty() {
            self.d.add_text(el, text);
        }
        el
    }

    /// A case wrapper whose top rule keeps walks from crossing into the case
    /// before it. Returns the section and the y its content starts at.
    fn case(&mut self, height: f64) -> (ElId, f64) {
        let y = self.y;
        let body = self.body;
        let sec = self.el(body, "section", (0.0, y, W, height), &[("borderTopWidth", "1px")], "");
        self.y += height + 48.0;
        (sec, y + 24.0)
    }

    /// Paragraph flush above an h2, content forty pixels below.
    fn crowded(&mut self, title: &str) {
        let (sec, y) = self.case(196.0);
        self.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
        self.el(sec, "h2", (0.0, y + 48.0, W, 36.0), &[("fontSize", "28px")], title);
        self.el(sec, "p", (0.0, y + 124.0, W, 48.0), &[], LONG);
    }

    fn flagged(&self) -> Vec<String> {
        check_heading_rhythm_dom(&self.d)
            .into_iter()
            .map(|f| f.finding.detail)
            .collect()
    }
}

fn assert_only_crowded(p: &Page) {
    let flagged = p.flagged();
    assert_eq!(flagged.len(), 2, "{flagged:#?}");
    assert!(flagged.iter().all(|s| s.contains("Crowded")), "{flagged:#?}");
}

/// The two crowded headings plus one more, the one `needle` names.
fn assert_flags(p: &Page, needle: &str) {
    let flagged = p.flagged();
    assert_eq!(flagged.len(), 3, "{flagged:#?}");
    assert!(flagged.iter().any(|s| s.contains(needle)), "{flagged:#?}");
}

fn crowded_pair() -> Page {
    let mut p = Page::new();
    p.crowded("Crowded One");
    p.crowded("Crowded Two");
    p
}

#[test]
fn a_crowded_title_in_flush_wrappers_still_flags() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(420.0);
    // A carousel ends with its controls, one of them a bordered button that
    // does not span the block, ten pixels above the title.
    let carousel = p.el(sec, "div", (0.0, y, W, 200.0), &[], "");
    p.el(carousel, "div", (0.0, y, W, 176.0), &[], "");
    p.el(carousel, "a", (340.0, y + 176.0, 120.0, 24.0), &[("borderBottomWidth", "1px")], "More");
    let outer = p.el(sec, "div", (0.0, y + 210.0, W, 36.0), &[], "");
    let inner = p.el(outer, "div", (0.0, y + 210.0, W, 36.0), &[], "");
    p.el(inner, "h2", (0.0, y + 210.0, W, 36.0), &[("fontSize", "28px")], "Wrapped Section Title");
    p.el(sec, "div", (0.0, y + 286.0, W, 120.0), &[], "Tiles the title introduces");
    let flagged = p.flagged();
    assert_eq!(flagged.len(), 3, "{flagged:#?}");
    assert!(
        flagged.iter().any(|s| s.contains("\"Wrapped Section Title\" has 10px above vs 40px below")),
        "{flagged:#?}"
    );
}

#[test]
fn an_eyebrow_in_a_wrapper_of_its_own_folds_into_the_heading() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let eyebrow_wrap = p.el(sec, "div", (0.0, y + 120.0, W, 16.0), &[], "");
    p.el(eyebrow_wrap, "p", (0.0, y + 120.0, W, 16.0), &[("fontSize", "12px")], "Introducing");
    let title_wrap = p.el(sec, "div", (0.0, y + 160.0, W, 36.0), &[], "");
    p.el(title_wrap, "h2", (0.0, y + 160.0, W, 36.0), &[("fontSize", "28px")], "Eyebrow In Wrapper");
    p.el(sec, "p", (0.0, y + 244.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // The same pair behind display: contents wrappers, as some builders emit.
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let eyebrow_wrap = p.el(sec, "div", (0.0, y + 120.0, W, 16.0), &[], "");
    p.el(eyebrow_wrap, "p", (0.0, y + 120.0, W, 16.0), &[("fontSize", "12px")], "Introducing");
    let contents = p.el(sec, "div", (0.0, 0.0, 0.0, 0.0), &[("display", "contents")], "");
    let title_wrap = p.el(contents, "div", (0.0, y + 160.0, W, 36.0), &[], "");
    p.el(title_wrap, "h2", (0.0, y + 160.0, W, 36.0), &[("fontSize", "28px")], "Eyebrow Behind Contents");
    p.el(sec, "p", (0.0, y + 244.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn a_heading_that_ends_its_row_has_nothing_below_to_introduce() {
    // Accordion triggers: each h4 is the last thing in a row padded below it.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    for (i, title) in [
        "Accordion Question That Opens The List Of Rows",
        "Accordion Question In The Middle Of The List",
        "Accordion Question On The Last Row Of The List",
    ]
    .iter()
    .enumerate()
    {
        let top = y + i as f64 * 72.0;
        let row = p.el(sec, "div", (0.0, top, W, 48.0), &[("paddingBottom", "24px")], "");
        p.el(row, "h4", (0.0, top, W, 24.0), &[("fontSize", "18px")], title);
    }
    assert_only_crowded(&p);

    // Card headlines, one per column of a row, each the last thing in its
    // column; the spacer under the row is not what they introduce.
    let mut p = crowded_pair();
    let (sec, y) = p.case(320.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let row = p.el(sec, "div", (0.0, y + 48.0, W, 156.0), &[], "");
    for (i, title) in ["Card Headline On The Right", "Card Headline On The Left"].iter().enumerate() {
        let x = i as f64 * 420.0;
        let col = p.el(row, "div", (x, y + 48.0, 380.0, 156.0), &[], "");
        p.el(col, "div", (x, y + 48.0, 380.0, 20.0), &[("fontSize", "12px")], "Culture");
        p.el(col, "h4", (x, y + 80.0, 380.0, 100.0), &[("fontSize", "18px"), ("marginBottom", "24px")], title);
    }
    p.el(sec, "div", (0.0, y + 228.0, W, 60.0), &[], "Next section");
    assert_only_crowded(&p);
}

#[test]
fn a_rule_a_photo_or_a_heading_above_is_not_content_the_heading_captions() {
    // The block above ends in a rule across its width.
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    p.el(sec, "div", (0.0, y, W, 49.0), &[("borderBottomWidth", "1px")], LONG);
    p.el(sec, "h3", (0.0, y + 73.0, W, 28.0), &[("fontSize", "20px")], "Divider Above");
    p.el(sec, "p", (0.0, y + 149.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // An hr twelve pixels above.
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "hr", (0.0, y + 72.0, W, 1.0), &[], "");
    p.el(sec, "h3", (0.0, y + 85.0, W, 28.0), &[("fontSize", "20px")], "Rule Above");
    p.el(sec, "p", (0.0, y + 153.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // A caption four pixels under its photo.
    let mut p = crowded_pair();
    let (sec, y) = p.case(220.0);
    let card = p.el(sec, "div", (0.0, y, 240.0, 204.0), &[], "");
    p.el(card, "img", (0.0, y, 240.0, 120.0), &[], "");
    p.el(card, "h3", (0.0, y + 124.0, 240.0, 28.0), &[("fontSize", "20px")], "Caption Under Photo");
    p.el(card, "p", (0.0, y + 176.0, 240.0, 28.0), &[], LONG);
    assert_only_crowded(&p);

    // An author name heading stacked over the title heading.
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "h3", (0.0, y + 72.0, W, 30.0), &[("fontSize", "26px")], "A Columnist With A Long Byline Name");
    p.el(sec, "h3", (0.0, y + 102.0, W, 30.0), &[("fontSize", "26px")], "Stacked Title");
    p.el(sec, "p", (0.0, y + 172.0, W, 24.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn a_heading_that_draws_its_own_rule_is_separated_by_it() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(
        sec,
        "h2",
        (0.0, y + 48.0, W, 53.0),
        &[("fontSize", "25px"), ("borderTopWidth", "1px"), ("borderBottomWidth", "1px")],
        "Banded Title",
    );
    p.el(sec, "p", (0.0, y + 122.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn space_held_open_by_padding_or_a_spacer_counts_as_space_above() {
    // An empty spacer box between the previous block and the heading.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "div", (0.0, y + 48.0, W, 80.0), &[], "");
    p.el(sec, "h2", (0.0, y + 128.0, W, 36.0), &[("fontSize", "28px")], "Heading After A Spacer");
    p.el(sec, "p", (0.0, y + 188.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // The previous panel's bottom padding: its framed screenshot ends fifty
    // pixels before the panel's box does.
    let mut p = crowded_pair();
    let (sec, y) = p.case(420.0);
    let panel = p.el(sec, "div", (0.0, y, W, 266.0), &[("paddingBottom", "50px")], "");
    p.el(panel, "div", (22.0, y, 345.0, 216.0), &[("backgroundColor", "rgb(19, 23, 27)"), ("borderBottomWidth", "1px")], "");
    p.el(sec, "h3", (0.0, y + 266.0, W, 28.0), &[("fontSize", "24px")], "Heading Under A Padded Panel");
    p.el(sec, "p", (0.0, y + 306.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // The heading's own top padding sets its first line well below its box.
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "h2", (0.0, y + 48.0, W, 56.0), &[("fontSize", "15px"), ("paddingTop", "36px")], "Footer Group Heading");
    p.el(sec, "ul", (0.0, y + 116.0, W, 48.0), &[], "Links under the footer heading");
    assert_only_crowded(&p);
}

#[test]
fn short_spacers_are_never_folded_in_as_an_eyebrow() {
    // Two 48px spacer boxes between the previous block and the heading: short
    // enough for the eyebrow fold's size test, but a label has words.
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "div", (0.0, y + 48.0, W, 48.0), &[], "");
    p.el(sec, "div", (0.0, y + 96.0, W, 48.0), &[], "");
    p.el(sec, "h2", (0.0, y + 144.0, W, 36.0), &[("fontSize", "28px")], "Heading After Short Spacers");
    p.el(sec, "p", (0.0, y + 204.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn an_icon_badge_above_is_a_picture_the_heading_captions() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(200.0);
    let badge = p.el(sec, "div", (368.0, y, 64.0, 64.0), &[("backgroundColor", "rgba(20, 160, 90, 0.1)")], "");
    p.el(badge, "svg", (384.0, y + 16.0, 32.0, 32.0), &[], "");
    p.el(sec, "h3", (0.0, y + 80.0, W, 28.0), &[("fontSize", "18px")], "Feature Under Its Icon");
    p.el(sec, "p", (0.0, y + 148.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn the_nearest_blocks_are_measured_whatever_the_source_order() {
    // A flex column orders the button last on screen but first in source.
    let mut p = crowded_pair();
    let (sec, y) = p.case(300.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "h2", (0.0, y + 68.0, W, 36.0), &[("fontSize", "28px")], "Heading In A Flex Column");
    p.el(sec, "div", (300.0, y + 254.0, 200.0, 44.0), &[], "Read more");
    p.el(sec, "div", (0.0, y + 128.0, W, 108.0), &[], LONG);
    assert_only_crowded(&p);

    // A stat figure's inline run draws past its paragraph's box; the gap is
    // measured from the paragraph, where the run's block ends.
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    let stat = p.el(sec, "p", (0.0, y + 18.0, W, 82.0), &[("fontSize", "90px")], "");
    p.el(stat, "span", (50.0, y, 80.0, 117.0), &[("display", "inline")], "60% reduction");
    p.el(sec, "h3", (0.0, y + 119.0, W, 40.0), &[("fontSize", "38px")], "Heading Under A Stat");
    p.el(sec, "p", (0.0, y + 175.0, W, 74.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn a_standfirst_behind_display_contents_is_what_sits_below() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "h2", (0.0, y + 58.0, W, 36.0), &[("fontSize", "28px")], "Standfirst Behind Contents");
    let contents = p.el(sec, "div", (0.0, 0.0, 0.0, 0.0), &[("display", "contents")], "");
    p.el(contents, "p", (0.0, y + 106.0, W, 24.0), &[], "A short standfirst under the heading.");
    p.el(sec, "div", (0.0, y + 178.0, W, 60.0), &[], "Plans");
    assert_only_crowded(&p);
}

#[test]
fn a_title_whose_wrapper_holds_the_spacing_measures_past_the_wrapper() {
    // A section header whose padding holds sixteen pixels above the title and
    // forty-eight below it.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let header = p.el(
        sec,
        "header",
        (0.0, y + 48.0, W, 104.0),
        &[("paddingTop", "16px"), ("paddingBottom", "48px")],
        "",
    );
    p.el(header, "h2", (0.0, y + 64.0, W, 40.0), &[("fontSize", "32px")], "Header Padding");
    p.el(sec, "p", (0.0, y + 152.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Header Padding\" has 16px above vs 48px below");

    // Six pixels of bottom padding, the rest a margin.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let wrap = p.el(
        sec,
        "div",
        (0.0, y + 64.0, W, 46.0),
        &[("paddingBottom", "6px"), ("marginBottom", "42px")],
        "",
    );
    p.el(wrap, "h2", (0.0, y + 64.0, W, 40.0), &[("fontSize", "32px")], "Small Padding");
    p.el(sec, "p", (0.0, y + 152.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Small Padding\" has 16px above vs 48px below");
}

#[test]
fn a_title_row_stretched_by_an_icon_or_a_button_measures_past_the_row() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let row = p.el(sec, "div", (0.0, y + 64.0, W, 48.0), &[("display", "flex"), ("marginBottom", "48px")], "");
    p.el(row, "span", (0.0, y + 64.0, 48.0, 48.0), &[("backgroundColor", "rgb(228, 224, 245)")], "");
    p.el(row, "h2", (60.0, y + 68.0, 740.0, 40.0), &[("fontSize", "32px")], "Icon Beside Heading");
    p.el(sec, "p", (0.0, y + 160.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Icon Beside Heading\" has 20px above vs 52px below");

    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let row = p.el(sec, "div", (0.0, y + 64.0, W, 56.0), &[("display", "flex"), ("marginBottom", "48px")], "");
    p.el(row, "h2", (0.0, y + 72.0, 560.0, 40.0), &[("fontSize", "32px")], "Pricing Title Row");
    p.el(row, "a", (600.0, y + 64.0, 200.0, 56.0), &[("backgroundColor", "rgb(34, 34, 34)")], "View all plans");
    p.el(sec, "p", (0.0, y + 168.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Pricing Title Row\" has 24px above vs 56px below");
}

#[test]
fn a_heading_last_in_one_column_measures_to_the_content_under_the_row() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    let grid = p.el(sec, "div", (0.0, y, W, 136.0), &[("display", "grid"), ("marginBottom", "48px")], "");
    let left = p.el(grid, "div", (0.0, y, 380.0, 136.0), &[], "");
    p.el(left, "p", (0.0, y, 380.0, 96.0), &[], LONG);
    p.el(left, "h3", (0.0, y + 108.0, 380.0, 28.0), &[("fontSize", "22px")], "Column Heading");
    let right = p.el(grid, "div", (420.0, y, 380.0, 96.0), &[], "");
    p.el(right, "p", (420.0, y, 380.0, 96.0), &[], LONG);
    p.el(sec, "p", (0.0, y + 184.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Column Heading\" has 12px above vs 48px below");
}

#[test]
fn spacer_boxes_below_count_as_space_below() {
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "h2", (0.0, y + 56.0, W, 40.0), &[("fontSize", "32px")], "Heading Over A Spacer");
    p.el(sec, "div", (0.0, y + 96.0, W, 48.0), &[], "");
    p.el(sec, "p", (0.0, y + 144.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Heading Over A Spacer\" has 8px above vs 48px below");

    // A builder's flex stack: every text in a wrapper, every gap a spacer box.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    let above = p.el(sec, "div", (0.0, y, W, 48.0), &[], "");
    p.el(above, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "div", (0.0, y + 48.0, W, 16.0), &[], "");
    let title = p.el(sec, "div", (0.0, y + 64.0, W, 40.0), &[], "");
    p.el(title, "h2", (0.0, y + 64.0, W, 40.0), &[("fontSize", "32px")], "Stack Heading");
    p.el(sec, "div", (0.0, y + 104.0, W, 48.0), &[], "");
    let below = p.el(sec, "div", (0.0, y + 152.0, W, 48.0), &[], "");
    p.el(below, "p", (0.0, y + 152.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Stack Heading\" has 16px above vs 48px below");
}

#[test]
fn only_a_line_set_as_a_label_folds_into_the_heading() {
    // A date line in its own wrapper, set like the body copy: content of its
    // own, so the gap above is the ten pixels to it.
    let mut p = crowded_pair();
    let (sec, y) = p.case(300.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let line = p.el(sec, "div", (0.0, y + 120.0, W, 24.0), &[], "");
    p.el(line, "p", (0.0, y + 120.0, W, 24.0), &[], "Updated March 2026");
    let title = p.el(sec, "div", (0.0, y + 154.0, W, 40.0), &[], "");
    p.el(title, "h2", (0.0, y + 154.0, W, 40.0), &[("fontSize", "32px")], "Wrapped Title");
    p.el(sec, "p", (0.0, y + 242.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Wrapped Title\" has 10px above vs 48px below");

    // The same place, a line set in tracked capitals: the title's label.
    let mut p = crowded_pair();
    let (sec, y) = p.case(300.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let line = p.el(sec, "div", (0.0, y + 120.0, W, 24.0), &[], "");
    p.el(
        line,
        "p",
        (0.0, y + 120.0, W, 24.0),
        &[("textTransform", "uppercase"), ("letterSpacing", "1.6px")],
        "Introducing",
    );
    let title = p.el(sec, "div", (0.0, y + 154.0, W, 40.0), &[], "");
    p.el(title, "h2", (0.0, y + 154.0, W, 40.0), &[("fontSize", "32px")], "Labelled Title");
    p.el(sec, "p", (0.0, y + 242.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn a_heading_that_ends_a_box_with_a_visible_edge_has_nothing_below() {
    // The heading ends a box that draws a rule under it.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let boxed = p.el(
        sec,
        "div",
        (0.0, y + 56.0, W, 72.0),
        &[("borderBottomWidth", "1px"), ("paddingBottom", "31px")],
        "",
    );
    p.el(boxed, "h3", (0.0, y + 56.0, W, 40.0), &[("fontSize", "24px")], "Heading Ending A Ruled Box");
    p.el(sec, "p", (0.0, y + 152.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);

    // The heading ends a band painted a color of its own.
    let band = |color: &str| {
        let mut p = crowded_pair();
        let (sec, y) = p.case(340.0);
        let band = p.el(
            sec,
            "div",
            (0.0, y, W, 280.0),
            &[("backgroundColor", color), ("paddingBottom", "32px")],
            "",
        );
        p.el(band, "p", (0.0, y, W, 200.0), &[], LONG);
        p.el(band, "h3", (0.0, y + 208.0, W, 40.0), &[("fontSize", "24px")], "Heading Ending A Band");
        p.el(sec, "p", (0.0, y + 300.0, W, 48.0), &[], LONG);
        p
    };
    assert_only_crowded(&band("rgb(236, 236, 236)"));
    // A white box on the white page draws no edge, so its padding is space.
    assert_flags(&band("rgb(255, 255, 255)"), "\"Heading Ending A Band\" has 8px above vs 52px below");
}

#[test]
fn a_box_framed_on_its_other_sides_above_is_content_not_a_rule() {
    let block_above = |styles: &[(&str, &str)]| {
        let mut p = crowded_pair();
        let (sec, y) = p.case(240.0);
        p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
        p.el(sec, "pre", (0.0, y + 48.0, W, 54.0), styles, "npx impeccable detect src/");
        p.el(sec, "h2", (0.0, y + 114.0, W, 32.0), &[("fontSize", "24px")], "Heading Under A Framed Block");
        p.el(sec, "p", (0.0, y + 182.0, W, 48.0), &[], LONG);
        p
    };
    let framed = [
        ("borderTopWidth", "1px"),
        ("borderRightWidth", "1px"),
        ("borderBottomWidth", "1px"),
        ("borderLeftWidth", "1px"),
        ("backgroundColor", "rgb(246, 248, 250)"),
    ];
    // A code block bordered on every side.
    assert_flags(&block_above(&framed), "\"Heading Under A Framed Block\" has 12px above vs 36px below");
    // A callout with a left border only.
    assert_flags(
        &block_above(&[("borderLeftWidth", "4px"), ("backgroundColor", "rgb(238, 246, 255)")]),
        "\"Heading Under A Framed Block\" has 12px above vs 36px below",
    );
    // The same block drawing only its bottom edge is a rule.
    assert_only_crowded(&block_above(&[("borderBottomWidth", "1px")]));

    // A panel framed on every side whose last row draws a divider: the line
    // sits inside the frame, and the frame is the edge.
    let mut p = crowded_pair();
    let (sec, y) = p.case(240.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let panel = p.el(sec, "div", (0.0, y + 48.0, W, 54.0), &framed, "");
    p.el(panel, "div", (1.0, y + 49.0, W - 2.0, 52.0), &[("borderBottomWidth", "1px")], "Last settings row");
    p.el(sec, "h2", (0.0, y + 114.0, W, 32.0), &[("fontSize", "24px")], "Heading Under A Panel");
    p.el(sec, "p", (0.0, y + 182.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Heading Under A Panel\" has 12px above vs 36px below");
}

#[test]
fn a_layout_row_that_shares_a_class_but_holds_other_content_does_not_repeat() {
    // Bootstrap rows: the title alone in its own `.row > .col-12`, the next
    // `.row` a row of feature columns headed by h3s.
    let mut p = crowded_pair();
    let (sec, y) = p.case(300.0);
    let row = p.el(sec, "div", (0.0, y, W, 48.0), &[("display", "flex")], "");
    p.d.set_attr(row, "class", "row");
    let col = p.el(row, "div", (0.0, y, W, 48.0), &[], "");
    p.d.set_attr(col, "class", "col-12");
    p.el(col, "p", (0.0, y, W, 48.0), &[], LONG);
    let row = p.el(sec, "div", (0.0, y + 48.0, W, 102.0), &[("display", "flex")], "");
    p.d.set_attr(row, "class", "row");
    let col = p.el(row, "div", (0.0, y + 48.0, W, 102.0), &[], "");
    p.d.set_attr(col, "class", "col-12");
    let title = p.el(col, "h2", (0.0, y + 64.0, W, 38.0), &[("fontSize", "32px")], "Title Alone In A Row");
    p.d.set_attr(title, "class", "mt-3 mb-5");
    let row = p.el(sec, "div", (0.0, y + 150.0, W, 100.0), &[("display", "flex")], "");
    p.d.set_attr(row, "class", "row");
    for i in 0..3 {
        let x = i as f64 * 267.0;
        let col = p.el(row, "div", (x, y + 150.0, 266.0, 100.0), &[], "");
        p.d.set_attr(col, "class", "col-md-4");
        p.el(col, "h3", (x, y + 150.0, 266.0, 29.0), &[("fontSize", "24px")], "Feature");
        p.el(col, "p", (x, y + 179.0, 266.0, 71.0), &[], LONG);
    }
    assert_flags(&p, "\"Title Alone In A Row\" has 16px above vs 48px below");

    // Block-editor groups: every block a `.wp-block-group`, the title alone in
    // a padded group between groups that open with h3s.
    let mut p = crowded_pair();
    let (sec, y) = p.case(300.0);
    let group = |p: &mut Page, top: f64, h: f64, styles: &[(&str, &str)]| {
        let g = p.el(sec, "div", (0.0, top, W, h), styles, "");
        p.d.set_attr(g, "class", "wp-block-group is-layout-constrained");
        g
    };
    let above = group(&mut p, y, 76.0, &[]);
    p.el(above, "h3", (0.0, y, W, 28.0), &[("fontSize", "20px")], "Sub");
    p.el(above, "p", (0.0, y + 28.0, W, 48.0), &[], LONG);
    let title = group(&mut p, y + 76.0, 102.0, &[("paddingTop", "16px"), ("paddingBottom", "48px")]);
    p.el(title, "h2", (0.0, y + 92.0, W, 38.0), &[("fontSize", "30px")], "Group Title");
    let below = group(&mut p, y + 178.0, 76.0, &[]);
    p.el(below, "h3", (0.0, y + 178.0, W, 28.0), &[("fontSize", "20px")], "Detail");
    p.el(below, "p", (0.0, y + 206.0, W, 48.0), &[], LONG);
    assert_flags(&p, "\"Group Title\" has 16px above vs 48px below");
}

#[test]
fn cards_that_repeat_the_headline_in_the_same_place_still_repeat() {
    // Two cards in a row, each ending in its headline. One carries a label
    // above the headline and the other does not, so the child paths differ;
    // the class chain down to the headline is the same.
    let mut p = crowded_pair();
    let (sec, y) = p.case(320.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    let row = p.el(sec, "div", (0.0, y + 48.0, W, 156.0), &[], "");
    let card = p.el(row, "div", (0.0, y + 48.0, 380.0, 156.0), &[], "");
    p.d.set_attr(card, "class", "card");
    p.el(card, "div", (0.0, y + 48.0, 380.0, 20.0), &[("fontSize", "12px")], "Culture");
    let title = p.el(card, "h4", (0.0, y + 80.0, 380.0, 100.0), &[("fontSize", "18px"), ("marginBottom", "24px")], "Labelled Card Headline");
    p.d.set_attr(title, "class", "card-title");
    let card = p.el(row, "div", (420.0, y + 48.0, 380.0, 156.0), &[("paddingTop", "32px")], "");
    p.d.set_attr(card, "class", "card");
    let title = p.el(card, "h4", (420.0, y + 80.0, 380.0, 100.0), &[("fontSize", "18px"), ("marginBottom", "24px")], "Unlabelled Card Headline");
    p.d.set_attr(title, "class", "card-title");
    p.el(sec, "div", (0.0, y + 228.0, W, 60.0), &[], "Next section");
    assert_only_crowded(&p);
}

#[test]
fn a_date_set_smaller_than_the_body_text_folds_into_a_card_title() {
    // A news card: a thumbnail, a 12px date four pixels above the title, and
    // a tag row set as small as the date. The date is the title's label, and
    // the pair sits under the card's own picture.
    let mut p = crowded_pair();
    let (sec, y) = p.case(420.0);
    let card = p.el(sec, "article", (0.0, y, 266.0, 380.0), &[("borderBottomWidth", "1px")], "");
    let thumb = p.el(card, "div", (0.0, y, 266.0, 150.0), &[], "");
    p.el(thumb, "img", (0.0, y, 266.0, 150.0), &[], "");
    let text = p.el(
        card,
        "div",
        (0.0, y + 150.0, 266.0, 186.0),
        &[("paddingTop", "16px"), ("paddingBottom", "16px")],
        "",
    );
    p.el(text, "time", (0.0, y + 166.0, 266.0, 18.0), &[("fontSize", "12px")], "August 27, 2026");
    p.el(text, "h3", (0.0, y + 188.0, 266.0, 103.0), &[("fontSize", "18px")], "A News Card Title Under Its Date");
    let tags = p.el(card, "ul", (0.0, y + 336.0, 266.0, 36.0), &[], "");
    p.el(tags, "li", (0.0, y + 336.0, 80.0, 24.0), &[("fontSize", "12px")], "Raytheon");
    assert_only_crowded(&p);
}

#[test]
fn a_spacer_between_an_eyebrow_and_its_heading_keeps_the_fold() {
    // Builders put a spacer box between the label and the title. The walk
    // above treats it as gap, so the fold steps over it too and the title is
    // not measured against its own label.
    let mut p = crowded_pair();
    let (sec, y) = p.case(260.0);
    p.el(sec, "p", (0.0, y, W, 48.0), &[], LONG);
    p.el(sec, "p", (0.0, y + 120.0, W, 16.0), &[("fontSize", "12px")], "Introducing");
    p.el(sec, "div", (0.0, y + 136.0, W, 12.0), &[], "");
    p.el(sec, "h2", (0.0, y + 148.0, W, 36.0), &[("fontSize", "28px")], "Eyebrow Over A Spacer");
    p.el(sec, "p", (0.0, y + 232.0, W, 48.0), &[], LONG);
    assert_only_crowded(&p);
}

#[test]
fn a_zero_shadow_is_not_an_edge() {
    // Tailwind's ring and shadow variables compute to all-zero layers; they
    // draw nothing, so the box's padding is still space below the heading.
    let band = |shadow: &str| {
        let mut p = crowded_pair();
        let (sec, y) = p.case(340.0);
        let band = p.el(
            sec,
            "div",
            (0.0, y, W, 280.0),
            &[("backgroundColor", "rgb(255, 255, 255)"), ("paddingBottom", "32px"), ("boxShadow", shadow)],
            "",
        );
        p.el(band, "p", (0.0, y, W, 200.0), &[], LONG);
        p.el(band, "h3", (0.0, y + 208.0, W, 40.0), &[("fontSize", "24px")], "Heading Ending A Band");
        p.el(sec, "p", (0.0, y + 300.0, W, 48.0), &[], LONG);
        p
    };
    assert_flags(
        &band("rgba(0, 0, 0, 0) 0px 0px 0px 0px, rgba(0, 0, 0, 0) 0px 0px 0px 0px"),
        "\"Heading Ending A Band\" has 8px above vs 52px below",
    );
    assert_only_crowded(&band("rgba(0, 0, 0, 0.1) 0px 1px 0px 0px"));
}
