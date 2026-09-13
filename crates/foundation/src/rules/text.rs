//! The selector lists, tag lists and thresholds the text rules are written
//! against, the plain-data inputs of the kicker and numbered-label checks,
//! and the two text parsers they share. The candidate gates and the checks
//! themselves live in the detector.

use crate::js::{self, parse_int, WS, WS_CHARS};
use crate::js_ext_b::utf16_len;
use crate::rules::types::D;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: Lazy<Regex> = Lazy::new(|| Regex::new(&$pat).expect(stringify!($name)));
    };
}

// ─── Shared constants (selectors, tag sets, regexes) ────────────────────────

/// JS: checks.mjs#HEADING_TAGS.
pub const HEADING_TAGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

/// JS: checks.mjs#KICKER_SKIP_SELECTOR.
pub const KICKER_SKIP_SELECTOR: &str = "nav,form,table,thead,tbody,tfoot,figure,figcaption,ol,ul,li,[role=\"navigation\"],[aria-label*=\"breadcrumb\" i],[class*=\"breadcrumb\" i],[aria-hidden=\"true\"],[data-impeccable-allow-kickers]";

/// JS: checks.mjs#KICKER_CARD_CONTEXT_SELECTOR.
pub const KICKER_CARD_CONTEXT_SELECTOR: &str =
    "article,button,a,li,[role=\"listitem\"],[role=\"option\"]";

/// JS: checks.mjs#NUMBERED_LABEL_TAGS.
pub const NUMBERED_LABEL_TAGS: &[&str] = &["span", "p", "div", "small", "em", "strong", "b"];

/// JS: checks.mjs#REPEATED_TEXT_SKIP_SELECTOR.
pub const REPEATED_TEXT_SKIP_SELECTOR: &str = "table,select,datalist,nav,menu,[role=\"navigation\"],[role=\"menu\"],[role=\"menubar\"],[role=\"listbox\"],[role=\"grid\"],[role=\"tablist\"],[role=\"radiogroup\"],[aria-hidden=\"true\"]";

/// JS: checks.mjs#REPEATED_TEXT_CONTAINER_TAGS.
pub const REPEATED_TEXT_CONTAINER_TAGS: &[&str] = &[
    "div", "section", "article", "aside", "main", "figure", "form", "fieldset", "details", "li",
];

/// JS: checks.mjs#QUALITY_TEXT_TAGS.
pub const QUALITY_TEXT_TAGS: &[&str] = &["p", "li", "td", "th", "dd", "blockquote", "figcaption"];

/// Where display type starts, for the tight-leading floor: 1.5x the 16px
/// default body size. Type at or above it sets its own leading, and a ratio
/// under 1.3 there is craft rather than crowding. Calibrated on the review
/// corpus: every reading block reviewers confirmed as harmfully crowded
/// measured 22px or smaller, while the display-type false positives (section
/// statements and card headlines set on `p` / `div` / `span` by page
/// builders) started at 24px and ran to 51px.
pub const LEADING_DISPLAY_TYPE_PX: f64 = 24.0;

/// How much of a second line box the tight-leading floor needs to see before
/// it judges leading. Text that renders as one line has no gap between lines
/// to crowd; wrapped text is two line boxes or more, so the midpoint
/// separates them.
pub const LEADING_MIN_LINE_BOXES: f64 = 1.5;

/// Heading context for the tight-leading floor. Headings carry their own
/// leading scale, and their text sits in a child `<a>` or `<span>` as often
/// as in the heading element, so the exemption is an ancestor test rather
/// than a tag test, and it honors the ARIA role a card title uses in place of
/// a heading tag.
pub const LEADING_HEADING_CONTEXT: &str = "h1, h2, h3, h4, h5, h6, [role=\"heading\"]";

/// The tags that carry a heading's own text when it is not in the heading
/// element itself. The ancestor exemption is limited to these, because a
/// heading can also contain a block of body copy: a page builder that nests a
/// card's description paragraph inside the card's `h3` is still setting
/// reading copy, and that block keeps the floor.
pub const LEADING_HEADING_TEXT_TAGS: &[&str] = &[
    "a", "span", "b", "i", "em", "strong", "small", "mark", "u", "abbr", "cite", "q", "time",
    "label", "font", "bdi", "bdo", "ins", "del",
];

/// JS: checks.mjs#TEXT_EDGE_TAGS (upper-case tag names, as the JS set).
pub const TEXT_EDGE_TAGS: &[&str] = &[
    "A",
    "BUTTON",
    "CODE",
    "DD",
    "DT",
    "FIGCAPTION",
    "H1",
    "H2",
    "H3",
    "H4",
    "H5",
    "H6",
    "LI",
    "P",
    "PRE",
    "SPAN",
    "TD",
    "TH",
];

/// `all-caps-body`: the length from which an uppercase run reads as a
/// sentence rather than as a label. Shorter runs are buttons, nav items,
/// kickers and eyebrows, where uppercase is a convention and the lost word
/// shapes cost nothing; judged against real sites those labels reach into the
/// seventies. Counted over the element's own text and never its subtree, so a
/// bar, a nav or a form control that holds several short caps labels is not
/// charged for their sum.
pub const ALL_CAPS_LONG_RUN: usize = 80;

/// JS: checks.mjs#SR_ONLY_SELECTOR.
pub const SR_ONLY_SELECTOR: &str = ".sr-only, .visually-hidden, .visuallyhidden, .screen-reader, .screen-reader-only, .screenreader, .a11y-hidden, .hidden-visually, [class*=\"sr-only\" i], [class*=\"visually-hidden\" i], [class*=\"visuallyhidden\" i], [class*=\"screen-reader\" i], [class*=\"screenreader\" i]";

/// JS: checks.mjs#NON_RENDERED_TAGS, less `map`. A `<map>` is an inline box
/// in every UA stylesheet and its flow content renders: the links an image
/// map repeats as text sit in it. Only its `<area>` children paint nothing.
pub const NON_RENDERED_TAGS: &[&str] = &[
    "script", "style", "title", "noscript", "template", "head", "meta", "link", "base", "param",
    "source", "track", "datalist", "col", "colgroup", "area",
];

/// JS: checks.mjs#TEXT_OVERFLOW_SKIP_TAGS.
pub const TEXT_OVERFLOW_SKIP_TAGS: &[&str] = &[
    "pre", "code", "textarea", "svg", "canvas", "select", "option", "marquee",
];

/// JS: checks.mjs#CURSOR_GLYPH_RE (`/^[_|▀-▟■▮❙❚｜]$/`).
pub static CURSOR_GLYPH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[_|▀-▟■▮❙❚｜]$").expect("CURSOR_GLYPH_RE"));

/// JS: checks.mjs#CURSOR_FIRST_VIEWPORT_PX.
pub const CURSOR_FIRST_VIEWPORT_PX: f64 = 1200.0;

/// JS: checks.mjs#HIDDEN_TEXT_EXCLUDE_TAGS.
pub const HIDDEN_TEXT_EXCLUDE_TAGS: &[&str] = &[
    "script", "style", "noscript", "template", "title", "head", "meta", "link", "option",
    "optgroup", "select", "datalist", "dialog",
];

/// JS: checks.mjs#OCCLUSION_TEXT_SKIP_TAGS.
pub const OCCLUSION_TEXT_SKIP_TAGS: &[&str] = &["script", "style", "noscript", "template", "title"];

/// JS: checks.mjs#POSITIONED_CHILD_INTERACTIVE_SELECTOR.
pub const POSITIONED_CHILD_INTERACTIVE_SELECTOR: &str = "a[href],button,input,select,summary,textarea,[tabindex]:not([tabindex=\"-1\"]),[role=\"button\"],[role=\"dialog\"],[role=\"link\"],[role=\"listbox\"],[role=\"menu\"],[role=\"menuitem\"],[role=\"option\"],[role=\"tooltip\"]";

/// Roles and attributes only a layer that has to escape its box carries.
/// They override the mask and scroller exemptions of
/// `clipped-overflow-container`: a menu parked by a transform is a menu.
pub const POPOVER_LAYER_SELECTOR: &str =
    "[popover],[role=\"dialog\"],[role=\"listbox\"],[role=\"menu\"],[role=\"menubar\"],[role=\"tooltip\"]";

// ─── Justified text ─────────────────────────────────────────────────────────

/// Longest measure at which justifying without hyphenation still opens
/// visible rivers. Wider columns have enough words per line to absorb the
/// stretch, which is why print justifies comfortably at a long measure. Read
/// in characters per line, the estimate `line-length` uses.
pub const JUSTIFY_NARROW_CHARS_PER_LINE: f64 = 45.0;

/// How much of an element's own text the script test reads. Long enough to
/// classify a paragraph, short enough that the cost does not grow with the
/// length of the page's longest block.
const JUSTIFY_SCRIPT_SAMPLE_CHARS: usize = 400;

/// Scripts that do not justify by stretching word spaces: CJK and Thai set on
/// a grid of uniform characters with no inter-word gaps to open, and Arabic
/// script justifies by elongating glyphs along the baseline. Rivers of white
/// are a word-space artifact, so they cannot form here.
fn justifies_without_word_spaces(c: char) -> bool {
    matches!(c as u32,
        0x0600..=0x06FF     // Arabic
        | 0x0750..=0x077F   // Arabic Supplement
        | 0x08A0..=0x08FF   // Arabic Extended-A
        | 0x0E00..=0x0E7F   // Thai
        | 0x1100..=0x11FF   // Hangul Jamo
        | 0x2E80..=0x9FFF   // CJK radicals through the main ideographs, kana, Hangul compat jamo
        | 0xA960..=0xA97F   // Hangul Jamo Extended-A
        | 0xAC00..=0xD7FF   // Hangul syllables and Jamo Extended-B
        | 0xF900..=0xFAFF   // CJK compatibility ideographs
        | 0xFB50..=0xFDFF   // Arabic Presentation Forms-A
        | 0xFE70..=0xFEFF   // Arabic Presentation Forms-B
        | 0xFF66..=0xFF9F   // Halfwidth katakana
        | 0x20000..=0x3134F // CJK ideograph extensions B and later
    )
}

/// True when most of the letters in `text` belong to one of those scripts, so
/// the justified-text premise does not reach it. Digits, punctuation and
/// spacing are not letters and do not count either way.
pub fn justifies_without_word_spaces_text(text: &str) -> bool {
    let mut exempt = 0usize;
    let mut other = 0usize;
    for c in text.chars().take(JUSTIFY_SCRIPT_SAMPLE_CHARS) {
        if !c.is_alphabetic() {
            continue;
        }
        if justifies_without_word_spaces(c) {
            exempt += 1;
        } else {
            other += 1;
        }
    }
    exempt > other
}

// ─── Kicker above heading ───────────────────────────────────────────────────

/// Input of `isKickerCandidate`. Numbers are JS numbers: pass NaN where the
/// JS caller would pass `undefined`; strings are `''` where JS would pass
/// `undefined` / `''` (both are falsy there).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KickerCandidateInput<'a> {
    pub heading_level: f64,
    #[serde(borrow)]
    pub heading_text: &'a str,
    pub heading_font_size: f64,
    #[serde(borrow)]
    pub kicker_tag: &'a str,
    #[serde(borrow)]
    pub kicker_text: &'a str,
    #[serde(borrow)]
    pub kicker_text_transform: &'a str,
    #[serde(borrow)]
    pub kicker_font_variant: &'a str,
    pub kicker_font_size: f64,
    pub kicker_letter_spacing: f64,
}

/// JS `text.replace(/^"|"$/g, '')`: one leading and one trailing quote.
pub fn strip_edge_quotes(text: &str) -> String {
    let s = text.strip_prefix('"').unwrap_or(text);
    let s = s.strip_suffix('"').unwrap_or(s);
    s.to_string()
}

// ─── Numbered section labels ────────────────────────────────────────────────

/// `{ index, text }` from `parseNumberedLabelText`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberedLabel {
    pub index: f64,
    pub text: String,
}

/// JS: checks.mjs#parseNumberedLabelText. A zero-padded / two-digit bare
/// index, or a 1-2 digit index, a non-word separator, and a short label.
pub fn parse_numbered_label_text(raw_text: Option<&str>) -> Option<NumberedLabel> {
    re!(WS_RE, format!("{}+", WS));
    re!(TWO_DIGIT_RE, format!(r"^({d}{{2}})$", d = D));
    re!(
        SEP_RE,
        format!(
            r"^({d}{{1,2}}){ws}*[^0-9A-Za-z_{wsc}]{ws}*[^{wsc}]",
            d = D,
            ws = WS,
            wsc = WS_CHARS
        )
    );
    let collapsed = WS_RE.replace_all(raw_text.unwrap_or(""), " ");
    let text = js::trim(&collapsed);
    if text.is_empty() || utf16_len(text) > 40 {
        return None;
    }
    let m = TWO_DIGIT_RE
        .captures(text)
        .or_else(|| SEP_RE.captures(text))?;
    let index = parse_int(&m[1], 10);
    if !index.is_finite() || index > 40.0 {
        return None;
    }
    Some(NumberedLabel {
        index,
        text: text.to_string(),
    })
}

/// Input of `isNumberedSectionLabelCandidate`. Numbers are JS numbers (NaN
/// for `undefined`); `label_index` `None` for JS `null` / `undefined`;
/// `label_font_weight` is the raw style string (`Number(x) || 400`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberedLabelCandidateInput<'a> {
    #[serde(borrow)]
    pub heading_tag: &'a str,
    #[serde(borrow)]
    pub heading_text: &'a str,
    pub heading_font_size: f64,
    #[serde(borrow)]
    pub label_tag: &'a str,
    pub label_index: Option<f64>,
    #[serde(borrow)]
    pub label_text: &'a str,
    pub label_font_size: f64,
    pub label_letter_spacing: f64,
    #[serde(borrow)]
    pub label_font_weight: &'a str,
    #[serde(borrow)]
    pub label_font_family: &'a str,
    #[serde(borrow)]
    pub label_text_transform: &'a str,
    #[serde(borrow)]
    pub label_color: &'a str,
}

/// One `collectNumberedSectionLabelCandidates` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberedLabelCandidate {
    pub index: f64,
    pub label_text: String,
    pub heading_tag: String,
    pub heading_text: String,
}

// ─── Em-dash overuse ────────────────────────────────────────────────────────

/// JS: constants.mjs#EM_DASH_FLOOR.
pub const EM_DASH_FLOOR: usize = 8;

/// JS: constants.mjs#EM_DASH_CHARS_PER_DASH.
pub const EM_DASH_CHARS_PER_DASH: usize = 500;
