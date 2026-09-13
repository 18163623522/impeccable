//! Port of cli/engine/rules/checks.mjs (see checks/mod.rs for the split):
//! the plain-data helpers and pure gates of Sections 4-6. Element / document
//! adapters live in the `html` crate.
//!
//! Style-reading helpers take a [`StyleMap`]: any lookup from the JS
//! camelCase computed-style property name (`borderTopWidth`, `clipPath`) to
//! its string value, so a jsdom-style map, a real cascade, and a test
//! `HashMap` all fit.

use crate::color::{self, Rgba};

use crate::js::{self, math_max, math_min3, math_round, number_to_string, to_fixed, WS, WS_CHARS};

use crate::js_ext_b::{slice_utf16_prefix, utf16_len};

use once_cell::sync::Lazy;
use regex::Regex;

/// The CSS value helpers, style traits and plain-data types these checks are
/// written against are shared; re-exported so `checks::measures` stays one path.
pub use impeccable_foundation::css::measures::*;

/// JS `\d` is ASCII only.
const D: &str = "[0-9]";

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

/// JS: checks.mjs#checkRadialSpotlight. Pure gate; `label` is a stable
/// identifier the fixture test keys on.
pub fn check_radial_spotlight(input: &RadialSpotlightInput) -> Vec<Finding> {
    let Some(stops) = parse_radial_gradient_stops(input.gradient_value) else {
        return vec![];
    };
    if stops.len() < 2 {
        return vec![];
    }
    let last = &stops[stops.len() - 1];
    let last_alpha = if last.transparent {
        0.0
    } else {
        last.color.map(|c| c.alpha_or_one()).unwrap_or(1.0)
    };
    if last_alpha > 0.05 {
        return vec![];
    }
    let colored: Vec<&GradientStop> = stops
        .iter()
        .filter(|s| !s.transparent && matches!(s.color, Some(c) if c.alpha_or_one() > 0.05))
        .collect();
    if colored.is_empty() {
        return vec![];
    }
    if colored.len() > 2 {
        return vec![];
    }
    if colored
        .iter()
        .any(|s| s.color.map(|c| c.alpha_or_one()).unwrap_or(1.0) >= 0.45)
    {
        return vec![];
    }
    let Some(chromatic) = colored
        .iter()
        .find(|s| color::has_chroma(s.color.as_ref(), Some(24.0)))
    else {
        return vec![];
    };
    if !(input.width >= 240.0 && input.height >= 160.0) {
        return vec![];
    }
    let cc = chromatic.color.expect("colored stop has a color");
    let alpha = to_fixed(cc.alpha_or_one(), 2);
    let name = match input.label {
        Some(l) if !l.is_empty() => l,
        _ => "section",
    };
    vec![Finding::new(
        "radial-spotlight-glow",
        format!(
            "radial-gradient spotlight glow \"{}\" ({} a{} → transparent) on {}x{} surface",
            name,
            color::color_to_hex(Some(&cc)),
            alpha,
            number_to_string(math_round(input.width)),
            number_to_string(math_round(input.height))
        ),
    )]
}

// ─── Cream / beige palette ──────────────────────────────────────────────────

/// JS: checks.mjs#isCreamColor. A warm, lightly-tinted off-white.
pub fn is_cream_color(rgb: Option<&Rgba>) -> bool {
    let Some(c) = rgb else { return false };
    let (r, g, b) = (c.r, c.g, c.b);
    if math_min3(r, g, b) < 209.0 {
        return false;
    }
    if !(r >= g && g >= b) {
        return false;
    }
    let warmth = r - b;
    warmth >= 6.0 && warmth <= 48.0
}

/// JS: checks.mjs#creamFromClassList. The Tailwind background token that
/// renders as a cream surface, or `None`.
pub fn cream_from_class_list(cls: Option<&str>) -> Option<String> {
    re!(ARB_RE, r"(?-u:\b)bg-\[([^\]]+)\]");
    let cls = cls?;
    if cls.is_empty() {
        return None;
    }
    if let Some(arb) = ARB_RE.captures(cls) {
        let inner = arb.get(1).map(|m| m.as_str()).unwrap_or("");
        let spaced = inner.replace('_', " ");
        if is_cream_color(color::parse_any_color(Some(&spaced)).as_ref()) {
            return Some(format!("bg-[{}]", inner));
        }
    }
    for (tok, hex) in TAILWIND_BG_HEX {
        let re = Regex::new(&format!(
            "(?:^|{ws}){}(?:$|{ws})",
            regex::escape(tok),
            ws = WS
        ))
        .expect("tailwind token regex");
        if re.is_match(cls) && is_cream_color(color::parse_any_color(Some(hex)).as_ref()) {
            return Some(tok.to_string());
        }
    }
    None
}

// ─── Oversized hero headline ────────────────────────────────────────────────
const OVERSIZED_H1_FONT_PX: f64 = 72.0;

const OVERSIZED_H1_MIN_CHARS: usize = 40;
const OVERSIZED_H1_MIN_VIEWPORT_HEIGHT_RATIO: f64 = 0.28;
const OVERSIZED_H1_MIN_VIEWPORT_AREA_RATIO: f64 = 0.25;

/// JS: checks.mjs#checkOversizedH1.
pub fn check_oversized_h1(input: &OversizedH1Input) -> Vec<Finding> {
    if input.tag != "h1" {
        return vec![];
    }
    let text_len = utf16_len(input.heading_text);
    if input.font_size >= OVERSIZED_H1_FONT_PX && text_len >= OVERSIZED_H1_MIN_CHARS {
        let mut viewport_detail = String::new();
        if let Some(rect) = input.rect {
            if input.viewport_width > 0.0 && input.viewport_height > 0.0 {
                let height_ratio = rect.height / input.viewport_height;
                let area_ratio =
                    (rect.width * rect.height) / (input.viewport_width * input.viewport_height);
                let dominates = height_ratio >= OVERSIZED_H1_MIN_VIEWPORT_HEIGHT_RATIO
                    || area_ratio >= OVERSIZED_H1_MIN_VIEWPORT_AREA_RATIO;
                if !dominates {
                    return vec![];
                }
                viewport_detail =
                    format!(", {}vh", number_to_string(math_round(height_ratio * 100.0)));
            }
        }
        return vec![Finding::new(
            "oversized-h1",
            format!(
                "{}px h1, {} chars{} \"{}\"",
                number_to_string(math_round(input.font_size)),
                text_len,
                viewport_detail,
                slice_utf16_prefix(input.heading_text, 60)
            ),
        )];
    }
    vec![]
}

/// How many boxes of one sibling row have to carry the hairline-and-halo pair
/// before it reads as the generated-card signature. A single popover,
/// dropdown or dialog draws an edge and an elevation on purpose, and mature
/// design systems draw both on their resting card too; what gives the
/// generated version away is the whole row wearing it at once.
pub const GPT_BORDER_SHADOW_MIN_ROW: usize = 3;

/// The blur a repeated halo has to reach. Tighter shadows are the ordinary
/// resting elevation of a card.
pub const GPT_BORDER_SHADOW_MIN_BLUR_PX: f64 = 32.0;

/// How far two boxes of a row may differ on either axis and still count as
/// the same card.
pub const GPT_BORDER_SHADOW_SIZE_TOLERANCE: f64 = 0.25;

/// How many wrapper levels a row walk climbs above the element. At depth 0
/// the row is the element's own siblings. At depth 1 each sibling of its
/// parent is a cell holding a card, the way a grid item or an `<a>` wraps
/// each card of a generated grid. At depth 2 the card sits two wrappers down,
/// as in an `<li><a>` cell or an article that holds copy beside a panel.
pub const GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH: usize = 2;

/// How many siblings one level of a row walk reads on each side of the
/// element, or of the wrapper it climbed to. A row of cards is a handful of
/// boxes; the bound keeps a list of thousands from paying for a read per
/// entry, and reading outward from the element means the boxes beside it are
/// read first.
pub const GPT_BORDER_SHADOW_MAX_SIBLINGS_PER_SIDE: usize = 24;

/// How many boxes a row walk reads inside one sibling cell while it looks
/// for the card sitting at the element's depth. Children are read one at a
/// time, first child then next sibling, so a cell holding thousands of
/// children costs no more than one holding 32.
pub const GPT_BORDER_SHADOW_MAX_CELL_NODES: usize = 32;

/// Whether two lengths are close enough to be the same card's.
pub fn gpt_border_shadow_lengths_close(x: f64, y: f64) -> bool {
    let max = math_max(x, y);
    max > 0.0 && ((x - y).abs() / max) <= GPT_BORDER_SHADOW_SIZE_TOLERANCE
}

/// Whether two boxes are close enough in size to be the same card. A box
/// with no area matches nothing, so an element that paints nothing never
/// joins a row.
pub fn gpt_border_shadow_sizes_match(a: &Rect, b: &Rect) -> bool {
    gpt_border_shadow_lengths_close(a.width, b.width)
        && gpt_border_shadow_lengths_close(a.height, b.height)
}

/// The tree a row walk reads, as one engine sees it. Navigation, plus the
/// four answers only the engine can give: whether two wrappers are the same
/// kind of cell, whether two boxes are the same card, whether a box carries
/// the hairline-and-halo pair, and whether it shows at rest.
pub trait GptBorderShadowRowTree {
    type El: Clone + PartialEq;
    fn parent(&self, el: &Self::El) -> Option<Self::El>;
    fn previous_sibling(&self, el: &Self::El) -> Option<Self::El>;
    fn next_sibling(&self, el: &Self::El) -> Option<Self::El>;
    /// The first element child, read without collecting the rest.
    fn first_child(&self, el: &Self::El) -> Option<Self::El>;
    /// Whether `other` is a cell of the same kind as `cell`, which holds the
    /// element somewhere below it.
    fn same_cell(&self, cell: &Self::El, other: &Self::El) -> bool;
    /// Whether `other` could be another card of the element's row.
    fn same_card(&self, card: &Self::El, other: &Self::El) -> bool;
    /// Whether `el` carries [`gpt_thin_border_wide_shadow_pair`].
    fn carries_pair(&self, el: &Self::El) -> bool;
    /// Whether `el` shows on the page before anyone interacts with it. A
    /// flyout laid out ahead of its hover is a popover waiting for its
    /// trigger: closed, or lifted out of the flow and transparent, hidden or
    /// parked off the page. A row of them under a nav bar is not a row of
    /// cards. Content staged in the flow for a scroll reveal still shows,
    /// since a visitor sees it by scrolling and a scan often reads the page
    /// before the reveal runs.
    fn paints_at_rest(&self, el: &Self::El) -> bool;
}

/// How many boxes of `el`'s row carry the same pair as a comparable card and
/// show at rest, `el` included; 0 when `el` itself does not show. The row is
/// what the layout repeats, not only the element's DOM siblings: the walk
/// climbs up to [`GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH`] wrappers, and at each
/// level reads the wrapper's siblings of the same kind for a card at the
/// element's own depth below them. So a grid whose items each wrap a card, or
/// a stack of articles each holding one panel, is one row, the way it reads
/// on the page. Levels are counted apart, since each is one repeated
/// position, and the walk stops at the threshold.
pub fn gpt_border_shadow_row_size<T: GptBorderShadowRowTree>(tree: &T, el: &T::El) -> usize {
    if !tree.paints_at_rest(el) {
        return 0;
    }
    let mut best = 1usize;
    let mut cell = el.clone();
    for depth in 0..=GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH {
        if depth > 0 {
            match tree.parent(&cell) {
                Some(parent) => cell = parent,
                None => break,
            }
        }
        best = best.max(gpt_border_shadow_row_at_depth(tree, el, &cell, depth));
        if best >= GPT_BORDER_SHADOW_MIN_ROW {
            break;
        }
    }
    best
}

/// One level of [`gpt_border_shadow_row_size`]: `cell` is `el` or the
/// ancestor `depth` levels above it, and its siblings are read outward, one
/// from each side per step.
fn gpt_border_shadow_row_at_depth<T: GptBorderShadowRowTree>(
    tree: &T,
    el: &T::El,
    cell: &T::El,
    depth: usize,
) -> usize {
    let mut row = 1usize;
    let mut before = tree.previous_sibling(cell);
    let mut after = tree.next_sibling(cell);
    for _ in 0..GPT_BORDER_SHADOW_MAX_SIBLINGS_PER_SIDE {
        if before.is_none() && after.is_none() {
            break;
        }
        if let Some(sibling) = before.take() {
            row += gpt_border_shadow_cell_row_mates(tree, el, cell, &sibling, depth);
            if row >= GPT_BORDER_SHADOW_MIN_ROW {
                return row;
            }
            before = tree.previous_sibling(&sibling);
        }
        if let Some(sibling) = after.take() {
            row += gpt_border_shadow_cell_row_mates(tree, el, cell, &sibling, depth);
            if row >= GPT_BORDER_SHADOW_MIN_ROW {
                return row;
            }
            after = tree.next_sibling(&sibling);
        }
    }
    row
}

/// Whether `node` is another card of `el`'s row: comparable, carrying the
/// pair, and showing at rest. The checks run cheapest first.
fn gpt_border_shadow_is_row_mate<T: GptBorderShadowRowTree>(
    tree: &T,
    el: &T::El,
    node: &T::El,
) -> bool {
    tree.same_card(el, node) && tree.carries_pair(node) && tree.paints_at_rest(node)
}

/// The cards inside `sibling`, a sibling of `cell`, that stand where `el`
/// stands inside `cell`: `depth` levels down, and row-mates of `el`. At depth
/// 0 the sibling is the candidate card itself. Inside a cell the walk reads
/// at most [`GPT_BORDER_SHADOW_MAX_CELL_NODES`] boxes across all its levels,
/// one child at a time.
fn gpt_border_shadow_cell_row_mates<T: GptBorderShadowRowTree>(
    tree: &T,
    el: &T::El,
    cell: &T::El,
    sibling: &T::El,
    depth: usize,
) -> usize {
    if depth == 0 {
        return usize::from(gpt_border_shadow_is_row_mate(tree, el, sibling));
    }
    if !tree.same_cell(cell, sibling) {
        return 0;
    }
    let mut level = vec![sibling.clone()];
    let mut budget = GPT_BORDER_SHADOW_MAX_CELL_NODES;
    for _ in 0..depth {
        let mut next = Vec::new();
        'collect: for node in &level {
            let mut child = tree.first_child(node);
            while let Some(current) = child {
                if budget == 0 {
                    break 'collect;
                }
                budget -= 1;
                child = tree.next_sibling(&current);
                next.push(current);
            }
        }
        if next.is_empty() {
            return 0;
        }
        level = next;
    }
    let mut found = 0usize;
    for node in &level {
        if found + 1 >= GPT_BORDER_SHADOW_MIN_ROW {
            break;
        }
        if gpt_border_shadow_is_row_mate(tree, el, node) {
            found += 1;
        }
    }
    found
}

#[cfg(test)]
mod gpt_border_shadow_walk_tests {
    use super::*;
    use std::cell::Cell;

    /// A plain arena tree that counts every navigation read a row walk makes.
    struct CountingTree {
        parents: Vec<Option<usize>>,
        children: Vec<Vec<usize>>,
        pair: Vec<bool>,
        reads: Cell<usize>,
    }

    impl CountingTree {
        fn new() -> Self {
            CountingTree {
                parents: vec![None],
                children: vec![Vec::new()],
                pair: vec![false],
                reads: Cell::new(0),
            }
        }
        fn add(&mut self, parent: usize, pair: bool) -> usize {
            let id = self.parents.len();
            self.parents.push(Some(parent));
            self.children.push(Vec::new());
            self.pair.push(pair);
            self.children[parent].push(id);
            id
        }
        fn read(&self) {
            self.reads.set(self.reads.get() + 1);
        }
        fn position(&self, el: usize) -> Option<(usize, usize)> {
            let parent = self.parents[el]?;
            let index = self.children[parent].iter().position(|&c| c == el)?;
            Some((parent, index))
        }
    }

    impl GptBorderShadowRowTree for CountingTree {
        type El = usize;
        fn parent(&self, el: &usize) -> Option<usize> {
            self.read();
            self.parents[*el]
        }
        fn previous_sibling(&self, el: &usize) -> Option<usize> {
            self.read();
            let (parent, index) = self.position(*el)?;
            index.checked_sub(1).map(|i| self.children[parent][i])
        }
        fn next_sibling(&self, el: &usize) -> Option<usize> {
            self.read();
            let (parent, index) = self.position(*el)?;
            self.children[parent].get(index + 1).copied()
        }
        fn first_child(&self, el: &usize) -> Option<usize> {
            self.read();
            self.children[*el].first().copied()
        }
        fn same_cell(&self, _: &usize, _: &usize) -> bool {
            true
        }
        fn same_card(&self, _: &usize, _: &usize) -> bool {
            true
        }
        fn carries_pair(&self, el: &usize) -> bool {
            self.pair[*el]
        }
        fn paints_at_rest(&self, _: &usize) -> bool {
            true
        }
    }

    /// Three list items, each holding `width` plain boxes and then one card:
    /// the row size the first item's card sees, and the navigation reads it
    /// took.
    fn walk_wide_cells(width: usize) -> (usize, usize) {
        let mut tree = CountingTree::new();
        let list = tree.add(0, false);
        let mut cards = Vec::new();
        for _ in 0..3 {
            let item = tree.add(list, false);
            for _ in 0..width {
                tree.add(item, false);
            }
            cards.push(tree.add(item, true));
        }
        tree.reads.set(0);
        let size = gpt_border_shadow_row_size(&tree, &cards[0]);
        (size, tree.reads.get())
    }

    #[test]
    fn the_cell_walk_reads_a_bounded_number_of_boxes() {
        // Narrow cells: the cards in the other items sit inside the budget.
        assert_eq!(walk_wide_cells(4).0, 3);

        // Wide cells: the cards sit past the budget, and the walk reads the
        // same number of boxes whether a cell holds 200 or 20,000.
        let (narrow_size, narrow_reads) = walk_wide_cells(200);
        let (wide_size, wide_reads) = walk_wide_cells(20_000);
        assert_eq!((narrow_size, wide_size), (1, 1));
        assert_eq!(narrow_reads, wide_reads);

        // One parent read and two sibling runs per level, and inside each
        // sibling cell a first-child read plus a next-sibling read per box.
        let per_side = GPT_BORDER_SHADOW_MAX_SIBLINGS_PER_SIDE;
        let per_cell = 2 * GPT_BORDER_SHADOW_MAX_CELL_NODES + 1;
        let per_level = 1 + 2 * (per_side + 1) + 2 * per_side * per_cell;
        assert!(wide_reads <= (GPT_BORDER_SHADOW_MAX_WRAPPER_DEPTH + 1) * per_level);
    }
}

/// Every border side that reads as a hairline: at most 1.5px wide in a color
/// solid enough to draw an edge.
fn visible_hairline_widths(input: &GptBorderShadowInput) -> Vec<f64> {
    let mut visible_thin: Vec<f64> = Vec::new();
    for (index, &width) in input.border_widths.iter().enumerate() {
        let color = input
            .border_colors
            .and_then(|cs| cs.get(index))
            .and_then(|c| c.as_deref())
            .filter(|c| !c.is_empty());
        let alpha = css_color_alpha(color);
        if width > 0.0 && width <= 1.5 && alpha >= 0.28 {
            visible_thin.push(width);
        }
    }
    visible_thin
}

fn widest(widths: &[f64]) -> f64 {
    let mut max = 0.0f64;
    for &w in widths {
        max = math_max(max, w);
    }
    max
}

/// The blur of the halo an element carries, or `None` when its widest outer
/// shadow is too tight to be one. One string parse and no per-side work, so
/// the engines run it first when they walk a row of siblings.
pub fn gpt_border_shadow_halo_blur_px(box_shadow: Option<&str>) -> Option<f64> {
    let blur = shadow_max_outer_blur_px(box_shadow, Some(0.12));
    if blur < GPT_BORDER_SHADOW_MIN_BLUR_PX {
        return None;
    }
    Some(blur)
}

/// The hairline-and-halo pair on one element: the widest visible hairline and
/// the widest blur drawn outside the box, or `None` when only one of the two
/// is there.
pub fn gpt_thin_border_wide_shadow_pair(input: &GptBorderShadowInput) -> Option<(f64, f64)> {
    let blur = gpt_border_shadow_halo_blur_px(input.box_shadow)?;
    let visible_thin = visible_hairline_widths(input);
    if visible_thin.len() < 2 {
        return None;
    }
    Some((widest(&visible_thin), blur))
}

/// The finding the engines report for a pair that repeats: a
/// [`gpt_thin_border_wide_shadow_pair`] on at least
/// [`GPT_BORDER_SHADOW_MIN_ROW`] boxes of one row, as
/// [`gpt_border_shadow_row_size`] counts it. `row_size` counts the element
/// itself.
pub fn gpt_border_shadow_row_finding(pair: (f64, f64), row_size: usize) -> Vec<Finding> {
    if row_size < GPT_BORDER_SHADOW_MIN_ROW {
        return vec![];
    }
    let (max_border, blur) = pair;
    vec![Finding::new(
        "gpt-thin-border-wide-shadow",
        format!(
            "{}px border + {}px shadow blur, repeated across the row",
            number_to_string(max_border),
            number_to_string(math_round(blur))
        ),
    )]
}

/// JS: checks.mjs#checkGptThinBorderWideShadow. The recorded call vectors pin
/// this signature-on-one-element form; the engines report the narrowed rule
/// through [`gpt_thin_border_wide_shadow_pair`],
/// [`gpt_border_shadow_row_size`] and [`gpt_border_shadow_row_finding`].
pub fn check_gpt_thin_border_wide_shadow(input: &GptBorderShadowInput) -> Vec<Finding> {
    let visible_thin = visible_hairline_widths(input);
    let max_border = widest(&visible_thin);
    let blur = shadow_max_blur_px(input.box_shadow, Some(0.12));
    if visible_thin.len() >= 2 && blur >= 16.0 {
        return vec![Finding::new(
            "gpt-thin-border-wide-shadow",
            format!(
                "{}px border + {}px shadow blur",
                number_to_string(max_border),
                number_to_string(math_round(blur))
            ),
        )];
    }
    vec![]
}

// ─── Clipped overflow / screen-reader-only text ─────────────────────────────

/// JS: checks.mjs#positionedStyleImpliesEscape. A positioned child's inset
/// declarations read as pushing it outside its clipping parent (negative
/// offset or a full 100% offset).
pub fn positioned_style_implies_escape(style: &dyn StyleMap) -> bool {
    re!(
        NEG_RE,
        format!(r"(?:^|[{ws}(])-+(?:{d}|\.)", ws = WS_CHARS, d = D)
    );
    re!(
        FULL_RE,
        format!(r"(?:^|[{ws}(])100(?:\.0+)?%", ws = WS_CHARS)
    );
    const PROPS: [&str; 11] = [
        "top",
        "right",
        "bottom",
        "left",
        "inset",
        "insetBlock",
        "insetInline",
        "insetBlockStart",
        "insetBlockEnd",
        "insetInlineStart",
        "insetInlineEnd",
    ];
    for prop in PROPS {
        let Some(v) = style.prop(prop) else { continue };
        if v.is_empty() {
            continue;
        }
        let value = js::to_lower_case(js::trim(&v));
        if NEG_RE.is_match(&value) {
            return true;
        }
        if FULL_RE.is_match(&value) {
            return true;
        }
    }
    false
}

/// JS: checks.mjs#checkContentHiddenAtRest. Pure threshold check over a
/// `measureHiddenTextDOM()` result.
pub fn check_content_hidden_at_rest(input: &ContentHiddenInput) -> Vec<Finding> {
    if input.total_chars < 200.0 || input.hidden_chars < 150.0 {
        return vec![];
    }
    let share = input.hidden_chars / input.total_chars;
    if share <= 0.3 {
        return vec![];
    }
    let sample = match input.hidden_samples.first() {
        Some(s) => format!(" (e.g. \"{}\")", s),
        None => String::new(),
    };
    vec![Finding::new(
        "content-hidden-at-rest",
        format!(
            "{}% of page text ({} of {} chars) stays at opacity 0 / visibility hidden after reveal handlers ran{}",
            number_to_string(math_round(share * 100.0)),
            number_to_string(input.hidden_chars),
            number_to_string(input.total_chars),
            sample
        ),
    )]
}

// ─── Text occlusion helper ──────────────────────────────────────────────────

/// JS: checks.mjs#isOpaqueDecoratedBox. A near-solid background fill or
/// two-plus visible borders make a box hide whatever sits behind it.
pub fn is_opaque_decorated_box(cs: Option<&dyn StyleMap>) -> bool {
    let Some(cs) = cs else { return false };
    let bg_raw = prop_or_empty(cs, "backgroundColor");
    if let Some(bg) = color::parse_any_color(Some(&bg_raw)) {
        if bg.alpha_or_one() > 0.6 {
            return true;
        }
    }
    let mut border_sides = 0usize;
    for side in ["Top", "Right", "Bottom", "Left"] {
        let w = parse_float_or_zero(cs.prop(&format!("border{}Width", side)).as_deref());
        if w <= 0.0 {
            continue;
        }
        let bc_raw = prop_or_empty(cs, &format!("border{}Color", side));
        if let Some(bc) = color::parse_any_color(Some(&bc_raw)) {
            if bc.alpha_or_one() > 0.3 {
                border_sides += 1;
            }
        }
    }
    border_sides >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn style(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // Expected values below were produced by running the JS functions in Node.

    #[test]
    fn css_color_is_transparent_cases() {
        assert!(css_color_is_transparent(None));
        assert!(css_color_is_transparent(Some("")));
        assert!(css_color_is_transparent(Some("  Transparent ")));
        assert!(css_color_is_transparent(Some("rgba(0, 0, 0, 0)")));
        assert!(css_color_is_transparent(Some("rgba(10,20,30,0.04)")));
        assert!(!css_color_is_transparent(Some("rgba(10,20,30,0.5)")));
        assert!(!css_color_is_transparent(Some("#fff")));
        assert!(css_color_is_transparent(Some("rgba(1, 2, 3, 0.00)")));
        assert!(!css_color_is_transparent(Some("notacolor")));
    }

    #[test]
    fn colors_nearly_match_cases() {
        assert!(colors_nearly_match(
            Some("#fff"),
            Some("rgb(254, 255, 253)")
        ));
        assert!(!colors_nearly_match(
            Some("#fff"),
            Some("rgb(250, 255, 255)")
        ));
        assert!(!colors_nearly_match(Some("#fff"), Some("nope")));
        assert!(!colors_nearly_match(
            Some("rgba(0,0,0,0.5)"),
            Some("rgba(0,0,0,0.6)")
        ));
        assert!(colors_nearly_match(
            Some("rgba(0,0,0,0.5)"),
            Some("rgba(0,0,0,0.52)")
        ));
    }

    #[test]
    fn parse_radial_gradient_stops_cases() {
        assert_eq!(parse_radial_gradient_stops(None), None);
        assert_eq!(
            parse_radial_gradient_stops(Some("linear-gradient(red, blue)")),
            None
        );
        assert_eq!(
            parse_radial_gradient_stops(Some(
                "repeating-radial-gradient(circle, #000 0 2px, transparent 2px 4px)"
            )),
            None
        );
        let stops = parse_radial_gradient_stops(Some(
            "radial-gradient(circle at 50% 50%, rgba(80,111,255,0.26), transparent 44%)",
        ))
        .unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].color, Some(Rgba::new(80.0, 111.0, 255.0, 0.26)));
        assert!(!stops[0].transparent);
        assert_eq!(stops[1].color, None);
        assert!(stops[1].transparent);
        assert_eq!(
            parse_radial_gradient_stops(Some("radial-gradient(#fff, #000")),
            None
        );
        assert_eq!(
            parse_radial_gradient_stops(Some("radial-gradient(circle, #fff)")),
            None
        );
    }

    #[test]
    fn shadow_layer_alpha_cases() {
        assert_eq!(shadow_layer_alpha("0 0 40px rgba(0,0,0,.5)"), 0.5);
        assert_eq!(shadow_layer_alpha("0 0 40px"), 1.0);
        assert_eq!(shadow_layer_alpha("0 0 40px transparent"), 0.0);
        assert_eq!(shadow_layer_alpha("0 0 4px #00000080"), 0.5019607843137255);
        assert_eq!(shadow_layer_alpha("inset 0 1px black"), 1.0);
        assert_eq!(shadow_layer_alpha("0 0 4px currentcolor"), 1.0);
    }

    #[test]
    fn shadow_max_blur_px_defaults() {
        assert_eq!(shadow_max_blur_px(None, None), 0.0);
        assert_eq!(
            shadow_max_blur_px(Some("0 0 20px rgba(0,0,0,0.05)"), None),
            20.0
        );
        assert_eq!(
            shadow_max_blur_px(Some("0 0 20px rgba(0,0,0,0.05)"), Some(0.12)),
            0.0
        );
        assert_eq!(
            shadow_max_blur_px(Some("0 1px 2px black, 0 0 30px hsl(200, 50%, 50%)"), None),
            30.0
        );
        assert_eq!(shadow_max_blur_px(Some("0px 0px 10px"), None), 10.0);
    }

    #[test]
    fn shadow_max_outer_blur_px_cases() {
        let px = |s: &str| shadow_max_outer_blur_px(Some(s), Some(0.12));
        assert_eq!(shadow_max_outer_blur_px(None, None), 0.0);
        assert_eq!(px("none"), 0.0);
        assert_eq!(px("0 0 40px rgba(15,23,42,0.18)"), 40.0);
        assert_eq!(px("rgba(15, 23, 42, 0.18) 0px 0px 40px 0px"), 40.0);
        // Lit from above is still a shadow drawn outside the box.
        assert_eq!(px("0 8px 40px rgba(15,23,42,0.22)"), 40.0);
        assert_eq!(px("-12px 0 36px -16px rgba(0,0,0,0.7)"), 36.0);
        // Drawn inside the box.
        assert_eq!(px("inset 0 0 40px rgba(15,23,42,0.18)"), 0.0);
        assert_eq!(px("rgba(15, 23, 42, 0.18) 0px 0px 40px 0px inset"), 0.0);
        // Too faint to paint.
        assert_eq!(px("0 0 40px rgba(0,0,0,0.05)"), 0.0);
        // The widest layer of a stack is the one reported; an inset layer
        // beside it is skipped.
        assert_eq!(
            px("0 8px 60px rgba(0,0,0,0.3), 0 0 36px rgba(0,0,0,0.2)"),
            60.0
        );
        assert_eq!(
            px("inset 0 0 80px rgba(0,0,0,0.3), 0 0 36px rgba(0,0,0,0.2)"),
            36.0
        );
    }

    #[test]
    fn css_color_alpha_cases() {
        assert_eq!(css_color_alpha(None), 0.0);
        assert_eq!(css_color_alpha(Some("transparent")), 0.0);
        assert_eq!(css_color_alpha(Some("rgba(0,0,0,0.5)")), 0.5);
        assert_eq!(css_color_alpha(Some("#fff")), 1.0);
        assert_eq!(css_color_alpha(Some("garbage")), 1.0);
    }

    #[test]
    fn border_from_style_cases() {
        let s = style(&[
            ("borderTopWidth", "1px"),
            ("borderRightWidth", "0px"),
            ("borderBottomWidth", "abc"),
            ("borderTopColor", "red"),
        ]);
        assert_eq!(border_widths_from_style(&s), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            border_colors_from_style(&s),
            [
                "red".to_string(),
                String::new(),
                String::new(),
                String::new()
            ]
        );
    }

    #[test]
    fn positioned_style_implies_escape_cases() {
        assert!(positioned_style_implies_escape(&style(&[("top", "-10px")])));
        assert!(positioned_style_implies_escape(&style(&[("left", "100%")])));
        assert!(positioned_style_implies_escape(&style(&[(
            "inset",
            "auto calc(100% + 4px)"
        )])));
        assert!(!positioned_style_implies_escape(&style(&[("top", "10px")])));
        assert!(!positioned_style_implies_escape(&style(&[("left", "50%")])));
        assert!(!positioned_style_implies_escape(&style(&[("left", "")])));
        assert!(!positioned_style_implies_escape(&style(&[])));
        assert!(!positioned_style_implies_escape(&style(&[(
            "left",
            "calc(50%-1px)"
        )])));
    }

    #[test]
    fn metric_length_cases() {
        assert_eq!(metric_length_px(LengthInput::Number(3.5), 16.0), Some(3.5));
        assert_eq!(metric_length_px(LengthInput::Number(f64::NAN), 16.0), None);
        assert_eq!(
            metric_length_px(LengthInput::Number(f64::INFINITY), 16.0),
            None
        );
        assert_eq!(metric_length_px(LengthInput::Text("2em"), 10.0), Some(20.0));
        assert_eq!(metric_length_px(LengthInput::Text("auto"), 10.0), None);
        assert_eq!(metric_length_px(LengthInput::Missing, 10.0), None);
        assert_eq!(
            first_metric_length_px(
                16.0,
                &[
                    LengthInput::Missing,
                    LengthInput::Text("auto"),
                    LengthInput::Text("1px"),
                    LengthInput::Number(9.0)
                ]
            ),
            Some(1.0)
        );
        assert_eq!(first_metric_length_px(16.0, &[]), None);
    }

    #[test]
    fn expand_box_shorthand_cases() {
        assert_eq!(expand_box_shorthand(&["a"]), vec!["a", "a", "a", "a"]);
        assert_eq!(expand_box_shorthand(&["a", "b"]), vec!["a", "b", "a", "b"]);
        assert_eq!(
            expand_box_shorthand(&["a", "b", "c"]),
            vec!["a", "b", "c", "b"]
        );
        assert_eq!(
            expand_box_shorthand(&["a", "b", "c", "d", "e"]),
            vec!["a", "b", "c", "d"]
        );
    }

    #[test]
    fn clipped_by_inset_cases() {
        assert!(clipped_by_inset(Some("inset(50%)")));
        assert!(clipped_by_inset(Some("inset(0% 50% 0% 50%)")));
        assert!(clipped_by_inset(Some("inset(50% 0%)")));
        assert!(clipped_by_inset(Some("INSET(0% 50% 0% 50% round 4px)")));
        assert!(clipped_by_inset(Some("inset(49.5% 0% 50.5%)")));
        // Every value must be a percentage: a unitless 0 fails the whole gate.
        assert!(!clipped_by_inset(Some("inset(100% 0 0 0)")));
        assert!(!clipped_by_inset(Some("inset(50% 0)")));
        assert!(!clipped_by_inset(Some("inset(10% 20%)")));
        assert!(!clipped_by_inset(Some("inset(50% 0% 49%)")));
        assert!(!clipped_by_inset(Some("inset(50px)")));
        assert!(!clipped_by_inset(Some("inset()")));
        assert!(!clipped_by_inset(Some("circle(0)")));
        assert!(!clipped_by_inset(None));
    }

    #[test]
    fn clipped_by_rect_cases() {
        assert!(clipped_by_rect(Some("rect(0 0 0 0)")));
        assert!(clipped_by_rect(Some("rect(0, 0, 0, 0)")));
        assert!(clipped_by_rect(Some("rect(1px, 1px, 1px, 1px)")));
        assert!(!clipped_by_rect(Some("rect(0 10px 10px 0)")));
        assert!(!clipped_by_rect(Some("rect(0 auto auto 0)")));
        assert!(!clipped_by_rect(Some("rect(0 0 0)")));
        assert!(!clipped_by_rect(Some("auto")));
        assert!(!clipped_by_rect(None));
        assert!(clipped_by_rect(Some("rect(0 1em 0 2em)")));
    }

    #[test]
    fn is_screen_reader_only_text_style_cases() {
        assert!(!is_screen_reader_only_text_style(
            None,
            &SrOnlyMetrics::default()
        ));
        let sr = style(&[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
            ("overflow", "hidden"),
        ]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr),
            &SrOnlyMetrics::default()
        ));
        let sr_no_clip = style(&[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
        ]);
        assert!(!is_screen_reader_only_text_style(
            Some(&sr_no_clip),
            &SrOnlyMetrics::default()
        ));
        let sr_clip = style(&[("clip", "rect(0 0 0 0)")]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr_clip),
            &SrOnlyMetrics::default()
        ));
        let sr_clip_path = style(&[("webkitClipPath", "inset(50%)")]);
        assert!(is_screen_reader_only_text_style(
            Some(&sr_clip_path),
            &SrOnlyMetrics::default()
        ));
        let big = style(&[
            ("position", "absolute"),
            ("width", "100px"),
            ("height", "1px"),
            ("overflow", "hidden"),
        ]);
        assert!(!is_screen_reader_only_text_style(
            Some(&big),
            &SrOnlyMetrics::default()
        ));
        // Metrics win over the style widths.
        assert!(is_screen_reader_only_text_style(
            Some(&big),
            &SrOnlyMetrics {
                width: Some(1.0),
                height: Some(1.0),
                ..Default::default()
            }
        ));
        // A 0 font size falls back to 16 for em math.
        let em = style(&[
            ("position", "absolute"),
            ("fontSize", "0px"),
            ("width", "0.1em"),
            ("height", "0.1em"),
            ("overflowY", "clip"),
        ]);
        assert!(is_screen_reader_only_text_style(
            Some(&em),
            &SrOnlyMetrics::default()
        ));
    }

    #[test]
    fn cream_from_class_list_cases() {
        assert_eq!(cream_from_class_list(None), None);
        assert_eq!(cream_from_class_list(Some("")), None);
        assert_eq!(
            cream_from_class_list(Some("min-h-screen bg-amber-50 text-stone-900")),
            Some("bg-amber-50".to_string())
        );
        assert_eq!(cream_from_class_list(Some("bg-stone-50")), None);
        assert_eq!(
            cream_from_class_list(Some("p-4 bg-[#f5f0e6]")),
            Some("bg-[#f5f0e6]".to_string())
        );
        assert_eq!(
            cream_from_class_list(Some("bg-[rgb(245_240_230)]")),
            Some("bg-[rgb(245_240_230)]".to_string())
        );
        assert_eq!(cream_from_class_list(Some("bg-[#ffffff]")), None);
        assert_eq!(cream_from_class_list(Some("bg-amber-500")), None);
        assert_eq!(
            cream_from_class_list(Some("bg-[#ffffff] bg-orange-50")),
            Some("bg-orange-50".to_string())
        );
        assert_eq!(cream_from_class_list(Some("xbg-amber-50")), None);
    }

    #[test]
    fn is_opaque_decorated_box_cases() {
        assert!(!is_opaque_decorated_box(None));
        assert!(is_opaque_decorated_box(Some(&style(&[(
            "backgroundColor",
            "rgb(255, 255, 255)"
        )]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[(
            "backgroundColor",
            "rgba(255, 255, 255, 0.5)"
        )]))));
        assert!(is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgb(0, 0, 0)"),
            ("borderBottomWidth", "1px"),
            ("borderBottomColor", "rgb(0, 0, 0)"),
        ]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgb(0, 0, 0)"),
            ("borderBottomWidth", "0px"),
            ("borderBottomColor", "rgb(0, 0, 0)"),
        ]))));
        assert!(!is_opaque_decorated_box(Some(&style(&[
            ("borderTopWidth", "1px"),
            ("borderTopColor", "rgba(0, 0, 0, 0.2)"),
            ("borderBottomWidth", "1px"),
            ("borderBottomColor", "rgba(0, 0, 0, 0.2)"),
        ]))));
    }
}
