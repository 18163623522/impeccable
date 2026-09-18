//! Text with no reading job, for `low-contrast` (taste call r3-04: "Rank
//! those shapes lower: reported as advisory, outside the failure count. The
//! clipto.com mockup labels and the terminal text stay visible.").
//!
//! Four shapes, each read conservatively off the DOM. Where a shape is
//! uncertain the finding keeps its severity, so every test below asks for
//! positive evidence and none of them is a guess about intent:
//!
//! - **Avatar initials**: one letter, or two, alone in a small square or
//!   round box that paints itself, and centred in it. Digits are left out
//!   (a notification count, a page number, a step), and so is a letter that
//!   is the whole label of a control (an A to Z index, a quiz option).
//! - **Build or version stamps**: text made only of a version or build
//!   identifier, optionally with a date, a time or a short hash
//!   (`v2.4.1`, `Build 1289`, `N0.0.1 · 2024-03-01`).
//! - **Signatures**: a short name set in a handwriting face, or marked
//!   `signature` by its class or id, outside headings and controls.
//! - **Text inside illustration mockups**: an ancestor that says it is one,
//!   by a `mockup` / `mock` / `illustration` class or id token, or by
//!   `role="img"`. A mockup built from utility classes alone says nothing,
//!   and its text keeps failing.
//!
//! The adapters gather [`DecorativeTextFacts`] against their own DOM; the
//! decision is made here, once, for both engines.

use once_cell::sync::Lazy;
use regex::Regex;

/// Smallest side, in px, of a box that can hold avatar initials.
pub const AVATAR_MIN_PX: f64 = 12.0;
/// Largest side, in px, of a box that still reads as a small avatar.
pub const AVATAR_MAX_PX: f64 = 72.0;
/// How far from square (width over height) an avatar box may be.
pub const AVATAR_MIN_ASPECT: f64 = 0.8;
pub const AVATAR_MAX_ASPECT: f64 = 1.25;
/// How far the text's centre may sit from the box's, as a share of the box.
pub const AVATAR_CENTRE_TOLERANCE: f64 = 0.15;

/// Whether a box is the size and shape of a small avatar.
pub fn avatar_sized(width: f64, height: f64) -> bool {
    if !(width.is_finite() && height.is_finite()) || height <= 0.0 {
        return false;
    }
    let aspect = width / height;
    (AVATAR_MIN_PX..=AVATAR_MAX_PX).contains(&width)
        && (AVATAR_MIN_PX..=AVATAR_MAX_PX).contains(&height)
        && (AVATAR_MIN_ASPECT..=AVATAR_MAX_ASPECT).contains(&aspect)
}

/// Whether a text box `(left, top, width, height)` is centred in a box.
pub fn centred_in(text: (f64, f64, f64, f64), host: (f64, f64, f64, f64)) -> bool {
    let (tx, ty, tw, th) = text;
    let (bx, by, bw, bh) = host;
    let dx = ((tx + tw / 2.0) - (bx + bw / 2.0)).abs();
    let dy = ((ty + th / 2.0) - (by + bh / 2.0)).abs();
    dx <= (bw * AVATAR_CENTRE_TOLERANCE).max(2.0) && dy <= (bh * AVATAR_CENTRE_TOLERANCE).max(2.0)
}

/// What the adapters read off one element.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecorativeTextFacts {
    /// The element's whole text, whitespace collapsed and trimmed.
    pub text: String,
    /// The element's computed `font-family`.
    pub font_family: String,
    /// The element is, or sits in, a heading.
    pub in_heading: bool,
    /// The element is, or sits in, an interactive control whose whole text
    /// is this element's text: the element is the control's label.
    pub control_label: bool,
    /// A small square or round box that paints itself holds this text and
    /// nothing else, centred in it ([`avatar_sized`], [`centred_in`]).
    pub avatar_box: bool,
    /// The element or its parent is marked a signature by class or id
    /// ([`is_signature_marker`]).
    pub signature_marked: bool,
    /// An ancestor says it is an illustration mockup ([`is_mockup_marker`],
    /// or `role="img"`).
    pub mockup_ancestor: bool,
}

/// The shape a text with no reading job was recognised by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorativeShape {
    AvatarInitials,
    VersionStamp,
    Signature,
    Mockup,
}

/// The decorative shape these facts show, if any.
pub fn classify_decorative_text(f: &DecorativeTextFacts) -> Option<DecorativeShape> {
    if f.mockup_ancestor {
        return Some(DecorativeShape::Mockup);
    }
    if f.text.is_empty() {
        return None;
    }
    if f.avatar_box && !f.control_label && is_initials(&f.text) {
        return Some(DecorativeShape::AvatarInitials);
    }
    if is_version_stamp(&f.text) {
        return Some(DecorativeShape::VersionStamp);
    }
    if !f.in_heading && !f.control_label {
        let by_marker = f.signature_marked && is_name_like(&f.text, false);
        let by_face = is_handwriting_face(&f.font_family) && is_name_like(&f.text, true);
        if by_marker || by_face {
            return Some(DecorativeShape::Signature);
        }
    }
    None
}

/// One letter or two, in any script, and nothing else.
pub fn is_initials(text: &str) -> bool {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    (1..=2).contains(&chars.len())
        && chars.iter().all(|c| c.is_alphabetic())
        && !text.trim().contains(char::is_whitespace)
}

static STAMP_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \b(?:version|ver\.?|build|release|rev(?:ision)?|commit)\s*[:\#]?\s*[0-9a-z][0-9a-z.+_-]*\d[0-9a-z.+_-]*
        | (?:^|[\s(\[])v\d+(?:\.\d+)+(?:[-+][0-9a-z.]+)?
        | (?:^|[\s(\[])[a-z]{1,3}\d+\.\d+\.\d+(?:\.\d+)?(?:[-+][0-9a-z.]+)?
        ",
    )
    .expect("STAMP_TOKEN_RE")
});
/// A bare three-part version (`1.10.0`). A dotted date (`2026.08.13`,
/// `13.08.26`) has a four-digit part or a zero-padded one, which a version
/// never has, and is left to [`STAMP_FILLER_RE`] as a date.
static BARE_VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:^|[\s(\[])(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?:[-+][0-9a-z.]+)?\b")
        .expect("BARE_VERSION_RE")
});

fn bare_version_spans(text: &str) -> Vec<(usize, usize)> {
    let padded = |p: &str| p.len() > 1 && p.starts_with('0');
    BARE_VERSION_RE
        .captures_iter(text)
        .filter(|c| !(padded(&c[2]) || padded(&c[3])))
        .map(|c| {
            let m = c.get(0).unwrap();
            (m.start(), m.end())
        })
        .collect()
}
static STAMP_FILLER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \b\d{4}[-./]\d{1,2}[-./]\d{1,2}\b
        | \b\d{1,2}[-./]\d{1,2}[-./]\d{2,4}\b
        | \b\d{1,2}:\d{2}(?::\d{2})?\b
        | \b[0-9a-f]{7,12}\b
        | [\s·•|/,()\[\]–—-]+
        ",
    )
    .expect("STAMP_FILLER_RE")
});

/// Text made only of a version or build identifier, with at most a date, a
/// time, a short hash and separators beside it. A version inside a sentence
/// ("Version 2 adds dark mode") is copy, not a stamp.
pub fn is_version_stamp(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 64 {
        return false;
    }
    let tokens = STAMP_TOKEN_RE.is_match(text);
    let mut rest = STAMP_TOKEN_RE.replace_all(text, " ").into_owned();
    let bare = bare_version_spans(&rest);
    if !tokens && bare.is_empty() {
        return false;
    }
    for (start, end) in bare.into_iter().rev() {
        rest.replace_range(start..end, " ");
    }
    STAMP_FILLER_RE.replace_all(&rest, "").is_empty()
}

static SIGNATURE_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^(?:[a-z0-9]+(?:-{1,2}|_{1,2}))?(?:signature|autograph|sign-?off)(?:(?:-{1,2}|_{1,2})(?:name|text|line|block|wrap|wrapper|img|image))?$",
    )
    .expect("SIGNATURE_TOKEN_RE")
});

/// Whether a class list or id names a signature: `signature`,
/// `author-signature`, `signature__name`. A class that only starts with the
/// word (`signature-dish`) names something else.
pub fn is_signature_marker(class_or_id: &str) -> bool {
    class_or_id
        .split_whitespace()
        .any(|token| SIGNATURE_TOKEN_RE.is_match(token))
}

/// Whether a class list or id names an illustration mockup: a token part
/// (split on `-`, `_`, `:` and `/`) that is `mock`, `mockup` or `mockups`,
/// or a token that contains `mockup` or `illustration`.
pub fn is_mockup_marker(class_or_id: &str) -> bool {
    class_or_id.split_whitespace().any(|token| {
        let lower = token.to_ascii_lowercase();
        lower.contains("mockup")
            || lower.contains("illustration")
            || lower
                .split(|c: char| matches!(c, '-' | '_' | ':' | '/'))
                .any(|part| part == "mock")
    })
}

/// Tags whose class or id says nothing about the text inside them being an
/// illustration: a `<section class="mockups">` holds the copy about them.
pub const MOCKUP_MARKER_SKIP_TAGS: &[&str] = &[
    "html", "body", "main", "section", "article", "header", "footer", "nav", "aside",
    "figcaption",
];

/// Handwriting faces a signature is set in, beyond the generic `cursive`
/// and any family with `script` or `handwriting` in its name.
const HANDWRITING_FACES: &[&str] = &[
    "allura",
    "alex brush",
    "arizonia",
    "birthstone",
    "caveat",
    "cedarville cursive",
    "cookie",
    "dawning of a new day",
    "great vibes",
    "herr von muellerhoff",
    "homemade apple",
    "italianno",
    "la belle aurore",
    "meddon",
    "monsieur la doulaise",
    "mr dafoe",
    "mrs saint delafield",
    "ms madi",
    "nothing you could do",
    "pacifico",
    "parisienne",
    "qwigley",
    "reenie beanie",
    "sacramento",
    "satisfy",
    "tangerine",
    "whisper",
    "yellowtail",
    "zeyada",
];

/// Whether the first family of a `font-family` list is a handwriting face.
pub fn is_handwriting_face(font_family: &str) -> bool {
    let first = font_family
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_ascii_lowercase();
    if first.is_empty() {
        return false;
    }
    first == "cursive"
        || first.contains("script")
        || first.contains("handwriting")
        || HANDWRITING_FACES.contains(&first.as_str())
}

/// A short name: one to four words (three when `capitalised`) of letters,
/// with `.`, `'` and `-` inside them, at most 40 characters, after an
/// optional sign-off dash or tilde. With `capitalised` every word starts in
/// upper case, which is what separates a name set in a script face from a
/// tagline set in one.
pub fn is_name_like(text: &str, capitalised: bool) -> bool {
    let text = text
        .trim()
        .trim_start_matches(|c: char| matches!(c, '-' | '–' | '—' | '~') || c.is_whitespace());
    if text.is_empty() || text.chars().count() > 40 {
        return false;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let max = if capitalised { 3 } else { 4 };
    if words.is_empty() || words.len() > max {
        return false;
    }
    words.iter().all(|w| {
        let mut chars = w.chars();
        let Some(first) = chars.next() else { return false };
        first.is_alphabetic()
            && (!capitalised || first.is_uppercase() || !first.is_lowercase())
            && w.chars().all(|c| c.is_alphabetic() || matches!(c, '.' | '\'' | '’' | '-'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(text: &str) -> DecorativeTextFacts {
        DecorativeTextFacts { text: text.to_string(), ..Default::default() }
    }

    #[test]
    fn initials_need_an_avatar_box_and_letters() {
        let boxed = |t: &str| DecorativeTextFacts { avatar_box: true, ..facts(t) };
        for t in ["J", "AB", "r", "李"] {
            assert_eq!(classify_decorative_text(&boxed(t)), Some(DecorativeShape::AvatarInitials), "{t}");
        }
        for t in ["3", "12", "A1", "ABC", "A B", "×"] {
            assert_eq!(classify_decorative_text(&boxed(t)), None, "{t}");
        }
        assert_eq!(classify_decorative_text(&facts("JD")), None);
        let label = DecorativeTextFacts { control_label: true, ..boxed("A") };
        assert_eq!(classify_decorative_text(&label), None);
    }

    #[test]
    fn version_stamps_stand_alone() {
        for t in [
            "v2.4.1",
            "V1.2",
            "Version 3.1.0",
            "Build 1289",
            "build: 2024.03.1",
            "N0.0.1 · 2024-03-01",
            "v1.8.0 (5f3a2c1)",
            "Release 12.4 — 03/01/2024",
            "commit 5f3a2c1d",
            "1.10.0",
            "0.0.1 (2024-03-01)",
        ] {
            assert!(is_version_stamp(t), "{t}");
        }
        for t in [
            "4.99",
            "1.2",
            "Version 2 adds dark mode",
            "© 2024 Acme Inc. v1.2",
            "Rebuild your workflow",
            "Build faster with templates",
            "10.0.0.1 is the router",
            "Revenue",
            "2026.08.13",
            "13.08.26",
            "13.08.2026",
            "12.03.24 · 09:30",
        ] {
            assert!(!is_version_stamp(t), "{t}");
        }
    }

    #[test]
    fn signatures_are_short_names_in_a_script_face_or_marked() {
        let face = |t: &str, fam: &str| DecorativeTextFacts { font_family: fam.to_string(), ..facts(t) };
        assert_eq!(
            classify_decorative_text(&face("Pedro", "\"Great Vibes\", cursive")),
            Some(DecorativeShape::Signature)
        );
        assert_eq!(
            classify_decorative_text(&face("— Sarah J.", "Dancing Script")),
            Some(DecorativeShape::Signature)
        );
        assert_eq!(classify_decorative_text(&face("Made with love", "cursive")), None);
        assert_eq!(classify_decorative_text(&face("Pedro", "Georgia, serif")), None);
        let heading = DecorativeTextFacts { in_heading: true, ..face("Pedro", "cursive") };
        assert_eq!(classify_decorative_text(&heading), None);
        let marked = DecorativeTextFacts { signature_marked: true, ..facts("pedro") };
        assert_eq!(classify_decorative_text(&marked), Some(DecorativeShape::Signature));
        assert!(is_signature_marker("text-sm author-signature"));
        assert!(is_signature_marker("signature__name"));
        assert!(!is_signature_marker("signature-dish"));
        assert!(!is_signature_marker("signatures-list item"));
    }

    #[test]
    fn mockups_say_so() {
        assert!(is_mockup_marker("hero-mockup rounded"));
        assert!(is_mockup_marker("mock-window"));
        assert!(is_mockup_marker("heroIllustration"));
        assert!(!is_mockup_marker("hammock"));
        assert!(!is_mockup_marker("mocha card"));
        let m = DecorativeTextFacts { mockup_ancestor: true, ..facts("Ready") };
        assert_eq!(classify_decorative_text(&m), Some(DecorativeShape::Mockup));
    }

    #[test]
    fn avatar_geometry() {
        assert!(avatar_sized(40.0, 40.0));
        assert!(avatar_sized(26.0, 24.0));
        assert!(!avatar_sized(80.0, 80.0));
        assert!(!avatar_sized(40.0, 20.0));
        assert!(!avatar_sized(10.0, 10.0));
        assert!(centred_in((14.0, 10.0, 12.0, 20.0), (0.0, 0.0, 40.0, 40.0)));
        assert!(!centred_in((2.0, 10.0, 12.0, 20.0), (0.0, 0.0, 40.0, 40.0)));
    }
}
