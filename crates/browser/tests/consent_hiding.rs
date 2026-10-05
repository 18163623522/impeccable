//! Known consent managers' banners are hidden before the rule pass, against
//! an installed browser. Skips cleanly when there is none.
//!
//! - A OneTrust-shaped banner over copy and a CTA with real contrast
//!   failures: with hiding (the default) the page copy and the CTA report and
//!   the banner's own text does not; with `keep_consent_banners` (the CLI's
//!   `--no-consent-hiding`, the engine's old behavior) the covered copy and
//!   CTA go unscored and the banner text reports.
//! - A Cookiebot-shaped modal injected late, on the first scroll, with an
//!   underlay and an inline scroll lock on `<body>`: caught by the hide pass
//!   after the reveal sweep, recorded in the evidence, and the lock undone.
//! - A notice the site wrote itself (generic names, no vendor) stays.
//! - A page that is only a consent wall is refused as one; with the banners
//!   kept it is scanned, since the banner is what that scan asked for.
//! - A short page under an ordinary banner (a sign-in form) is not a wall.
//! - An app shell's own body lock stays when a manager's backdrop shows.
//! - A wall that arrives after the load-time check, or renders its text in a
//!   shadow root, is still refused as one.
//! - A lock the site's own open modal holds stays.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use impeccable_browser::{detect_url_evidence, BrowserEngine, EvidenceRequest};
use impeccable_core::findings::Finding;
use impeccable_detect::engines::{ScanOptions, UrlEngine};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/consent")
}

fn serve() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || handle(stream));
        }
    });
    port
}

fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    // Only files in the fixture directory: no `..` segment, no absolute path.
    let rel = path.trim_start_matches('/');
    let body = if rel.split('/').any(|seg| seg == ".." || seg.contains('\\')) {
        Vec::new()
    } else {
        std::fs::read(fixtures_dir().join(rel)).unwrap_or_default()
    };
    let head = format!(
        "HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
}

fn engine() -> Option<BrowserEngine> {
    let env: HashMap<String, String> = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: no installed browser found");
        return None;
    }
    Some(BrowserEngine::new(env))
}

fn flagged(findings: &[Finding]) -> Vec<(String, String)> {
    findings
        .iter()
        .map(|f| {
            (
                f.antipattern.clone(),
                f.extras.get("selector").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            )
        })
        .collect()
}

fn has(flagged: &[(String, String)], rule: &str, selector: &str) -> bool {
    flagged.iter().any(|(r, s)| r == rule && s == selector)
}

const KEEP: ScanOptions = ScanOptions {
    inline_ignores: false,
    design_system: None,
    viewport: None,
    profile: None,
    rule_pack: None,
    keep_consent_banners: true,
};

#[test]
fn a_onetrust_banner_is_hidden_and_the_page_under_it_scored() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/onetrust.html");

    let scan = engine.detect_url_scan(&url, &ScanOptions::default()).expect("scan");
    let hidden = flagged(&scan.findings);
    assert!(has(&hidden, "low-contrast", "#covered-copy"), "{hidden:#?}");
    assert!(has(&hidden, "low-contrast", "#covered-cta"), "{hidden:#?}");
    assert!(!has(&hidden, "low-contrast", "#onetrust-policy-text"), "{hidden:#?}");
    // Every finding names what was hidden, and the text output says so.
    for f in &scan.findings {
        assert_eq!(f.extras.get("consentHidden"), Some(&serde_json::json!(["OneTrust"])), "{f:?}");
    }
    assert_eq!(
        scan.notes,
        vec![format!(
            "Hid the OneTrust consent banner on {url} before scanning. Pass --no-consent-hiding to scan it."
        )]
    );

    // Opted out, the banner stays: what it covers is not scored, and its own
    // text is, the way the engine scanned before hiding existed.
    let kept = engine.detect_url_scan(&url, &KEEP).expect("scan");
    let shown = flagged(&kept.findings);
    assert!(!has(&shown, "low-contrast", "#covered-copy"), "{shown:#?}");
    assert!(!has(&shown, "low-contrast", "#covered-cta"), "{shown:#?}");
    assert!(has(&shown, "low-contrast", "#onetrust-policy-text"), "{shown:#?}");
    assert!(kept.findings.iter().all(|f| f.extras.get("consentHidden").is_none()));
    assert!(kept.notes.is_empty());
}

#[test]
fn a_late_cookiebot_modal_is_caught_and_its_scroll_lock_undone() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/cookiebot.html");
    let mut browser = engine.launch().expect("launch");
    let (findings, evidence) =
        detect_url_evidence(&mut browser, &url, &ScanOptions::default(), "load", 100, &EvidenceRequest::default())
            .expect("scan");
    let (kept, kept_evidence) =
        detect_url_evidence(&mut browser, &url, &KEEP, "load", 100, &EvidenceRequest::default()).expect("scan");
    browser.close();

    let hidden = flagged(&findings);
    assert!(has(&hidden, "low-contrast", "#covered-copy"), "{hidden:#?}");
    assert!(has(&hidden, "low-contrast", "#covered-cta"), "{hidden:#?}");
    assert!(!has(&hidden, "low-contrast", "#CybotCookiebotDialogBodyContentText"), "{hidden:#?}");
    let consent = evidence.consent.as_ref().expect("consent report");
    assert_eq!(consent.hidden, vec!["Cookiebot"]);
    assert_eq!(
        consent.matched,
        vec![(
            "Cookiebot".to_string(),
            vec!["#CybotCookiebotDialog".to_string(), "#CybotCookiebotDialogBodyUnderlay".to_string()]
        )]
    );
    assert_eq!(consent.unlocked, vec!["body style overflow"]);
    // The validity probe ran before the dialog arrived.
    assert!(evidence.probe.as_ref().unwrap().consent.is_empty());
    // The capture every pass read has the dialog hidden, and the evidence
    // path leaves the findings untagged so they replay equal.
    assert!(evidence.scan_snapshot.as_deref().unwrap().contains("impeccable-consent-hide"));
    assert!(findings.iter().all(|f| f.extras.get("consentHidden").is_none()));

    let shown = flagged(&kept);
    assert!(!has(&shown, "low-contrast", "#covered-copy"), "{shown:#?}");
    assert!(has(&shown, "low-contrast", "#CybotCookiebotDialogBodyContentText"), "{shown:#?}");
    assert!(kept_evidence.consent.is_none());
}

#[test]
fn a_notice_the_site_wrote_itself_stays() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/site-notice.html");
    let scan = engine.detect_url_scan(&url, &ScanOptions::default()).expect("scan");
    let shown = flagged(&scan.findings);
    assert!(!has(&shown, "low-contrast", "#covered-copy"), "{shown:#?}");
    assert!(!has(&shown, "low-contrast", "#covered-cta"), "{shown:#?}");
    assert!(has(&shown, "low-contrast", "#notice-text"), "{shown:#?}");
    assert!(scan.findings.iter().all(|f| f.extras.get("consentHidden").is_none()));
    assert!(scan.notes.is_empty());
}

#[test]
fn a_page_that_is_only_a_consent_wall_is_refused() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/wall.html");
    let err = engine.detect_url(&url, &ScanOptions::default()).expect_err("a consent wall must not scan");
    assert!(err.message.starts_with("the page is a consent wall, not the site (consent manager OneTrust, "), "{}", err.message);
    // With the banners kept, the gate is off and the dialog is what is read.
    engine.detect_url(&url, &KEEP).expect("a kept consent wall scans");
    let mut browser = engine.launch().expect("launch");
    let (findings, evidence) =
        detect_url_evidence(&mut browser, &url, &ScanOptions::default(), "load", 100, &EvidenceRequest::default())
            .expect("evidence scan of a consent wall is Ok");
    browser.close();
    assert!(findings.is_empty());
    assert_eq!(evidence.validity.as_ref().unwrap().to_value()["status"], "consent-wall");
    assert_eq!(evidence.probe.as_ref().unwrap().consent, vec!["OneTrust"]);
    assert!(evidence.consent.is_none());
}

#[test]
fn a_short_page_under_a_banner_is_scanned_not_refused() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/short-page.html");
    let scan = engine.detect_url_scan(&url, &ScanOptions::default()).expect("a sign-in page is a page");
    let found = flagged(&scan.findings);
    assert!(has(&found, "low-contrast", "#hint"), "{found:#?}");
    assert!(!has(&found, "low-contrast", "#onetrust-policy-text"), "{found:#?}");
    let kept = engine.detect_url_scan(&url, &KEEP).expect("kept, it scans too");
    let found = flagged(&kept.findings);
    assert!(has(&found, "low-contrast", "#onetrust-policy-text"), "{found:#?}");
}

#[test]
fn an_app_shells_own_body_lock_stays() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/app-shell.html");
    let mut browser = engine.launch().expect("launch");
    let (_, evidence) =
        detect_url_evidence(&mut browser, &url, &ScanOptions::default(), "load", 100, &EvidenceRequest::default())
            .expect("scan");
    browser.close();
    let consent = evidence.consent.as_ref().expect("consent report");
    assert_eq!(consent.hidden, vec!["OneTrust"]);
    // The backdrop was showing, so only the app-shell check kept the lock.
    assert!(consent.matched[0].1.iter().any(|s| s == ".onetrust-pc-dark-filter"), "{:?}", consent.matched);
    assert!(consent.unlocked.is_empty(), "{:?}", consent.unlocked);
    let snapshot = evidence.scan_snapshot.as_deref().unwrap();
    assert!(snapshot.contains("overflow: hidden"), "the body keeps its own lock");
}

#[test]
fn a_wall_that_arrives_after_load_is_refused() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/late-wall.html");
    let err = engine.detect_url(&url, &ScanOptions::default()).expect_err("a late consent wall must not scan");
    assert!(err.message.starts_with("the page is a consent wall, not the site (consent manager OneTrust, "), "{}", err.message);
    let mut browser = engine.launch().expect("launch");
    let (findings, evidence) =
        detect_url_evidence(&mut browser, &url, &ScanOptions::default(), "load", 100, &EvidenceRequest::default())
            .expect("evidence scan of a consent wall is Ok");
    browser.close();
    assert!(findings.is_empty());
    assert_eq!(evidence.validity.as_ref().unwrap().to_value()["status"], "consent-wall");
}

#[test]
fn a_wall_rendered_in_a_shadow_root_is_refused() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/shadow-wall.html");
    let err = engine.detect_url(&url, &ScanOptions::default()).expect_err("a shadow-root consent wall must not scan");
    assert!(err.message.starts_with("the page is a consent wall, not the site (consent manager Usercentrics, "), "{}", err.message);
}

#[test]
fn a_lock_the_sites_own_modal_holds_stays() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/site-modal.html");
    let mut browser = engine.launch().expect("launch");
    let (_, evidence) =
        detect_url_evidence(&mut browser, &url, &ScanOptions::default(), "load", 100, &EvidenceRequest::default())
            .expect("scan");
    browser.close();
    let consent = evidence.consent.as_ref().expect("consent report");
    assert_eq!(consent.hidden, vec!["Cookiebot"]);
    assert!(consent.unlocked.is_empty(), "{:?}", consent.unlocked);
}
