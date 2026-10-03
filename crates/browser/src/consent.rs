//! Known consent managers, and hiding their banners before the rule pass.
//!
//! A consent banner is not the site. At capture it covers copy a visitor
//! reads once they dismiss it, so text under it is either skipped as covered
//! or scored against the banner, and the crops show the banner instead of the
//! page. Before the reveal sweep the URL engine hides the banners and
//! backdrops of the consent managers listed here with an injected style
//! (`display: none`), and undoes the scroll lock the manager applied. It never
//! clicks, sets a cookie, or records a consent choice: the page's own state is
//! unchanged, only painted without the manager's layer.
//!
//! The list holds selectors specific to one vendor (its ids, or class names
//! carrying its name), never a generic word such as "cookie" or "banner", so
//! a notice a site wrote for itself stays on the page and is scanned like the
//! rest of it. The validity gate ([`crate::validity`]) reads the same list to
//! recognize a page that is nothing but a consent wall.
//!
//! Corpus evidence (runs 20 and 25, 848 captures of 126 sites, desktop and
//! 390px): OneTrust on 7 sites (adm.com, cisco.com, cvs.com, exxonmobil.com,
//! mckesson.com, otto.de, thecignagroup.com; `#onetrust-consent-sdk`,
//! `#onetrust-banner-sdk` and `.onetrust-pc-dark-filter` on every one),
//! Usercentrics on 2 (fedex.com, tchibo.de; `#usercentrics-cmp-ui`), and one
//! site each for Cookiebot (theagenticdatacompany.com,
//! `#CybotCookiebotDialog`), TrustArc (samsung.com, `#consent_blackbar` and
//! `#truste-consent-track`), Didomi (ladepeche.fr, `#didomi-host`), the
//! open-source Cookie Consent library (centene.com,
//! `.cc-window[aria-label="cookieconsent"]`) and Google's consent messages
//! (ynet.co.il, `.fc-ccpa-root`). The other vendors are listed from their
//! published markup and have no corpus capture yet.
//!
//! Run 35 added two. consentmanager.net on letour.fr: `div#cmpbox` with
//! `role=dialog`, rendered in the open shadow root of `div#cmpwrapper`, a
//! fixed bar over the lower 28 to 36% of the first screen. And Borlabs Cookie
//! 3 on fischundfang.de, already listed by `#BorlabsCookieBox`, left three
//! things behind once the box was hidden: its floating reopen button
//! (`#BorlabsCookieWidget`, a fixed 48px shield at the left edge), the inline
//! `overflow: hidden` on `<body>` its modal sets behind
//! `#BorlabsDialogBackdrop` (it stopped the reveal sweep from scrolling), and
//! `aria-hidden="true"` on the page wrapper (`#td-outer-wrap`) and its
//! siblings, each marked `data-borlabs-cookie-aria-hidden`.

use serde_json::{json, Value};

/// One consent manager: the elements to hide and the scroll lock it applies.
#[derive(Debug, Clone, Copy)]
pub struct ConsentManager {
    pub name: &'static str,
    /// The banner, dialog, or preference-center roots.
    pub roots: &'static [&'static str],
    /// Overlays the manager lays over the page behind its dialog.
    pub backdrops: &'static [&'static str],
    /// Classes the manager puts on `<html>` to stop the page scrolling.
    pub html_lock_classes: &'static [&'static str],
    /// Classes the manager puts on `<body>` to stop the page scrolling.
    pub body_lock_classes: &'static [&'static str],
    /// Attributes the manager sets beside the `aria-hidden="true"` it puts on
    /// the page while its dialog is open, as its own record of which elements
    /// it hid. The hide step removes `aria-hidden` from exactly those
    /// elements, since the rules treat aria-hidden text as not the page's.
    pub aria_hidden_marks: &'static [&'static str],
}

/// Every known consent manager, in the order a report names them.
pub const CONSENT_MANAGERS: &[ConsentManager] = &[
    ConsentManager {
        name: "OneTrust",
        roots: &["#onetrust-consent-sdk", "#onetrust-banner-sdk", "#onetrust-pc-sdk"],
        backdrops: &[".onetrust-pc-dark-filter"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Cookiebot",
        roots: &["#CybotCookiebotDialog"],
        backdrops: &["#CybotCookiebotDialogBodyUnderlay"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Usercentrics",
        roots: &["#usercentrics-root", "#usercentrics-cmp-ui"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "TrustArc",
        roots: &["#consent_blackbar", "#truste-consent-track", ".truste_box_overlay"],
        backdrops: &[".truste_overlay"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Didomi",
        roots: &["#didomi-host"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &["didomi-popup-open"],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Quantcast Choice",
        roots: &["#qc-cmp2-container", ".qc-cmp2-container"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Sourcepoint",
        roots: &["[id^='sp_message_container_']"],
        backdrops: &[],
        html_lock_classes: &["sp-message-open"],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Osano",
        roots: &[".osano-cm-dialog", ".osano-cm-info-dialog"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Cookie Consent",
        roots: &[".cc-window[aria-label='cookieconsent']", ".cc-window.cc-banner", ".cc-grower"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "CookieYes",
        roots: &[".cky-consent-container", ".cky-modal", "#cookie-law-info-bar", "#cliSettingsPopup"],
        backdrops: &[".cky-overlay", ".cli-modal-backdrop"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Complianz",
        roots: &["#cmplz-cookiebanner-container", ".cmplz-cookiebanner"],
        backdrops: &[".cmplz-soft-cookiewall"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "iubenda",
        roots: &["#iubenda-cs-banner"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Termly",
        roots: &["#termly-code-snippet-support"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Axeptio",
        roots: &[".axeptio_mount"],
        backdrops: &["#axeptio_overlay"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Borlabs Cookie",
        roots: &["#BorlabsCookieBox", "#BorlabsCookieBoxWrap", "#BorlabsCookieWidget"],
        backdrops: &["#BorlabsDialogBackdrop"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &["data-borlabs-cookie-aria-hidden"],
    },
    ConsentManager {
        name: "Klaro",
        roots: &["#klaro .cookie-notice", "#klaro .cookie-modal"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Cookie Notice",
        roots: &["#cookie-notice:has(#cn-notice-text)"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Shopify",
        roots: &["#shopify-pc__banner", "#shopify-pc__prefs"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "consentmanager",
        // Current builds render the box inside an open shadow root on
        // `#cmpwrapper`, out of reach of a document selector, so the host is
        // what is hidden; `#cmpbox` covers builds that render it in the page.
        roots: &["#cmpwrapper", "#cmpbox"],
        backdrops: &[],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
    ConsentManager {
        name: "Google Funding Choices",
        roots: &[".fc-consent-root", ".fc-ccpa-root"],
        backdrops: &[".fc-dialog-overlay"],
        html_lock_classes: &[],
        body_lock_classes: &[],
        aria_hidden_marks: &[],
    },
];

/// The id of the style element the hide step injects.
pub const HIDE_STYLE_ID: &str = "impeccable-consent-hide";

fn managers_json() -> Value {
    Value::Array(
        CONSENT_MANAGERS
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "roots": m.roots,
                    "backdrops": m.backdrops,
                    "htmlLock": m.html_lock_classes,
                    "bodyLock": m.body_lock_classes,
                    "ariaMarks": m.aria_hidden_marks,
                })
            })
            .collect(),
    )
}

/// Page JS shared by the probe and the hide step. `showing(el)` is whether an
/// element (or, for a zero-size wrapper such as `#onetrust-consent-sdk`, one
/// of its first descendants) paints a visible box.
const SHOWING_JS: &str = r#"const q = s => { try { return Array.from(document.querySelectorAll(s)); } catch (e) { return []; } };
  const boxShows = el => {
    try {
      if (el.checkVisibility && !el.checkVisibility({ opacityProperty: true, visibilityProperty: true })) return false;
      const r = el.getBoundingClientRect();
      return r.width >= 1 && r.height >= 1;
    } catch (e) { return false; }
  };
  const showing = el => {
    if (!el || !el.isConnected) return false;
    try {
      if (el.checkVisibility && !el.checkVisibility({ opacityProperty: true, visibilityProperty: true })) return false;
    } catch (e) {}
    if (boxShows(el)) return true;
    for (const scope of [el, el.shadowRoot]) {
      if (!scope) continue;
      const kids = scope.querySelectorAll('*');
      for (let i = 0; i < kids.length && i < 400; i++) if (boxShows(kids[i])) return true;
    }
    return false;
  };"#;

/// A probe fragment. Defines `consent` (the names of the managers whose roots
/// are showing), `consentChars` (the visible text inside those roots) and
/// `consentOutside` (visible form controls and sizable media outside them,
/// counted up to 20).
pub fn probe_fragment() -> String {
    format!(
        r#"const consentManagers = {managers};
  {showing}
  const consent = [];
  const consentRoots = [];
  for (const m of consentManagers) {{
    let on = false;
    for (const s of m.roots) for (const el of q(s)) if (showing(el)) {{ on = true; consentRoots.push(el); }}
    if (on) consent.push(m.name);
  }}
  let consentChars = 0;
  for (const el of consentRoots) {{
    if (consentRoots.some(o => o !== el && o.contains(el))) continue;
    consentChars += (el.innerText || '').replace(/\s+/g, ' ').trim().length;
  }}
  // Content outside the managers that is not text: a form control of any
  // size, or an image, video, canvas, SVG or frame of at least
  // {media_px} square pixels. A sign-in form or an image-first page has
  // little text of its own and is still a page.
  let consentOutside = 0;
  if (consentRoots.length) {{
    const inside = el => consentRoots.some(r => r.contains(el));
    for (const el of q({controls})) {{
      if (consentOutside >= 20) break;
      if (!inside(el) && boxShows(el)) consentOutside++;
    }}
    for (const el of q({media})) {{
      if (consentOutside >= 20) break;
      if (inside(el) || !boxShows(el)) continue;
      const r = el.getBoundingClientRect();
      if (r.width * r.height >= {media_px}) consentOutside++;
    }}
  }}"#,
        managers = managers_json(),
        showing = SHOWING_JS,
        controls = json!(OUTSIDE_CONTROLS),
        media = json!(OUTSIDE_MEDIA),
        media_px = OUTSIDE_MEDIA_MIN_AREA,
    )
}

/// Form controls that make a page more than its consent wall, wherever they
/// sit outside a manager's roots.
pub const OUTSIDE_CONTROLS: &str = "input:not([type='hidden']), select, textarea, button";
/// Media that make a page more than its consent wall, when at least
/// [`OUTSIDE_MEDIA_MIN_AREA`] square pixels (a logo is smaller).
pub const OUTSIDE_MEDIA: &str = "img, svg, video, canvas, iframe, object, embed";
pub const OUTSIDE_MEDIA_MIN_AREA: u32 = 10_000;

/// The hide step. Idempotent: run it once before the reveal sweep and again
/// right before the capture, to catch a manager that injects its banner late.
/// Returns `{ hidden: [name], matched: { name: [selector] }, unlocked: [what],
/// changed }` for the managers it found showing this time; `changed` is
/// whether this run altered the page (a new rule, an inline hide, an unlock).
pub fn hide_js() -> String {
    format!(
        r#"(() => {{
  const managers = {managers};
  const STYLE_ID = {style_id};
  {showing}
  const out = {{ hidden: [], matched: {{}}, unlocked: [], changed: false }};
  const present = [];
  for (const m of managers) {{
    const sels = m.roots.concat(m.backdrops).filter(s => q(s).length > 0);
    if (sels.length) present.push({{ m, sels }});
  }}
  if (!present.length) return out;
  let style = document.getElementById(STYLE_ID);
  // Measure with the hide rule off, so a banner the rule already hides (one
  // injected after the first pass) is still reported. Nothing paints between
  // disabling and re-enabling it: this runs in one task.
  if (style) style.disabled = true;
  const showingBackdrop = [];
  for (const {{ m, sels }} of present) {{
    const on = sels.filter(s => q(s).some(showing));
    if (on.length) {{
      out.hidden.push(m.name);
      out.matched[m.name] = on;
      if (on.some(s => m.backdrops.includes(s))) showingBackdrop.push(m.name);
    }}
  }}
  if (!style) {{
    style = document.createElement('style');
    style.id = STYLE_ID;
    (document.head || document.documentElement).appendChild(style);
  }}
  style.disabled = false;
  const rules = [];
  for (const {{ sels }} of present) for (const s of sels) rules.push(s + ' {{ display: none !important; }}');
  const text = rules.join('\n');
  if (style.textContent !== text) {{ style.textContent = text; out.changed = true; }}
  // A manager that sets `display` inline with !important outranks the rule.
  for (const {{ sels }} of present) for (const s of sels) for (const el of q(s)) {{
    try {{
      if (getComputedStyle(el).display !== 'none') {{ el.style.setProperty('display', 'none', 'important'); out.changed = true; }}
    }} catch (e) {{}}
  }}
  // Undo the scroll lock the manager applied, and only that: its own classes
  // on <html> and <body>, and an inline overflow: hidden on either while its
  // backdrop was up (a modal consent layer locks the page that way) and the
  // page is not an app shell that scrolls inside itself. A site's own
  // overflow is left alone.
  const html = document.documentElement;
  const body = document.body;
  for (const {{ m }} of present) {{
    if (!out.hidden.includes(m.name)) continue;
    for (const c of m.htmlLock) if (html.classList.contains(c)) {{ html.classList.remove(c); out.unlocked.push('html.' + c); }}
    if (body) for (const c of m.bodyLock) if (body.classList.contains(c)) {{ body.classList.remove(c); out.unlocked.push('body.' + c); }}
    // The aria-hidden the manager put on the page behind its dialog, read off
    // the manager's own mark, so an aria-hidden the site set stays.
    for (const mark of m.ariaMarks) {{
      let n = 0;
      for (const el of q('[' + mark + ']')) {{
        if (el.getAttribute('aria-hidden') === 'true') {{ el.removeAttribute('aria-hidden'); n++; }}
      }}
      if (n && !out.unlocked.includes('aria-hidden [' + mark + ']')) out.unlocked.push('aria-hidden [' + mark + ']');
    }}
  }}
  // An app shell locks <body> itself and scrolls an inner container that
  // fills the viewport. That lock is the site's, whatever the manager did,
  // so it stays.
  const appShell = () => {{
    const all = body ? body.querySelectorAll('*') : [];
    const vh = window.innerHeight || 0;
    for (let i = 0; i < all.length && i < 5000; i++) {{
      const el = all[i];
      if (el.clientHeight < vh * 0.5 || el.scrollHeight <= el.clientHeight + 1) continue;
      if (present.some(({{ sels }}) => sels.some(s => {{ try {{ return !!el.closest(s); }} catch (e) {{ return false; }} }}))) continue;
      const oy = getComputedStyle(el).overflowY;
      if (oy === 'auto' || oy === 'scroll') return true;
    }}
    return false;
  }};
  if (showingBackdrop.length && !appShell()) {{
    for (const [el, tag] of [[html, 'html'], [body, 'body']]) {{
      if (!el) continue;
      for (const p of ['overflow', 'overflow-y']) {{
        if (el.style.getPropertyValue(p) === 'hidden') {{
          el.style.removeProperty(p);
          out.unlocked.push(tag + ' style ' + p);
        }}
      }}
    }}
  }}
  if (out.unlocked.length) out.changed = true;
  return out;
}})()"#,
        managers = managers_json(),
        style_id = json!(HIDE_STYLE_ID),
        showing = SHOWING_JS,
    )
}

/// What the hide step did over a scan: the managers it hid, the selectors
/// that matched a showing element, and the scroll locks it undid.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConsentReport {
    /// Managers whose banner or backdrop was showing and was hidden, in
    /// [`CONSENT_MANAGERS`] order.
    pub hidden: Vec<String>,
    /// Per hidden manager, the selectors that matched a showing element.
    pub matched: Vec<(String, Vec<String>)>,
    /// The scroll locks undone (`body.didomi-popup-open`, `body style overflow`).
    pub unlocked: Vec<String>,
}

impl ConsentReport {
    /// Fold one hide-step result into the report.
    pub fn merge(&mut self, v: &Value) {
        let strs = |v: Option<&Value>| -> Vec<String> {
            v.and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default()
        };
        for name in strs(v.get("hidden")) {
            let sels = strs(v.get("matched").and_then(|m| m.get(&name)));
            match self.matched.iter_mut().find(|(n, _)| *n == name) {
                Some((_, have)) => {
                    for s in sels {
                        if !have.contains(&s) {
                            have.push(s);
                        }
                    }
                }
                None => self.matched.push((name.clone(), sels)),
            }
            if !self.hidden.contains(&name) {
                self.hidden.push(name);
            }
        }
        for u in strs(v.get("unlocked")) {
            if !self.unlocked.contains(&u) {
                self.unlocked.push(u);
            }
        }
        let order = |n: &String| CONSENT_MANAGERS.iter().position(|m| m.name == n).unwrap_or(usize::MAX);
        self.hidden.sort_by_key(order);
        self.matched.sort_by_key(|(n, _)| order(n));
    }

    pub fn to_value(&self) -> Value {
        let matched: serde_json::Map<String, Value> = self
            .matched
            .iter()
            .map(|(n, s)| (n.clone(), json!(s)))
            .collect();
        json!({ "hidden": self.hidden, "matched": matched, "unlocked": self.unlocked })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No selector is a bare generic word: each names its vendor, or is an id
    /// or class the vendor's own markup carries.
    #[test]
    fn selectors_are_vendor_specific() {
        let generic = ["cookie", "cookies", "consent", "banner", "gdpr", "privacy", "cookie-banner", "cookie-consent", "consent-banner", "modal", "overlay"];
        for m in CONSENT_MANAGERS {
            for s in m.roots.iter().chain(m.backdrops) {
                let bare = s.trim_start_matches(['#', '.']).to_ascii_lowercase();
                assert!(!generic.contains(&bare.as_str()), "{}: {s} is generic", m.name);
            }
        }
    }

    #[test]
    fn every_requested_vendor_is_listed() {
        for name in [
            "OneTrust", "Cookiebot", "Usercentrics", "TrustArc", "Didomi", "Quantcast Choice", "Sourcepoint",
            "Osano", "CookieYes", "Complianz", "iubenda", "Termly", "Axeptio", "Borlabs Cookie", "Klaro",
            "Cookie Notice", "Shopify", "consentmanager",
        ] {
            assert!(CONSENT_MANAGERS.iter().any(|m| m.name == name), "{name}");
        }
    }

    #[test]
    fn report_merges_passes_in_list_order() {
        let mut r = ConsentReport::default();
        r.merge(&json!({ "hidden": ["Cookiebot"], "matched": { "Cookiebot": ["#CybotCookiebotDialog"] }, "unlocked": [] }));
        r.merge(&json!({
            "hidden": ["OneTrust", "Cookiebot"],
            "matched": { "OneTrust": ["#onetrust-consent-sdk"], "Cookiebot": ["#CybotCookiebotDialog", "#CybotCookiebotDialogBodyUnderlay"] },
            "unlocked": ["body style overflow"],
        }));
        assert_eq!(r.hidden, vec!["OneTrust", "Cookiebot"]);
        assert_eq!(
            r.to_value(),
            json!({
                "hidden": ["OneTrust", "Cookiebot"],
                "matched": {
                    "OneTrust": ["#onetrust-consent-sdk"],
                    "Cookiebot": ["#CybotCookiebotDialog", "#CybotCookiebotDialogBodyUnderlay"],
                },
                "unlocked": ["body style overflow"],
            })
        );
    }

    #[test]
    fn scripts_embed_every_selector() {
        let (probe, hide) = (probe_fragment(), hide_js());
        for m in CONSENT_MANAGERS {
            for s in m.roots.iter().chain(m.backdrops) {
                let quoted = serde_json::to_string(s).unwrap();
                assert!(hide.contains(&quoted), "{s}");
                assert!(probe.contains(&quoted), "{s}");
            }
        }
    }
}
