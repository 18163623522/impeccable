//! Surfaces off the ancestor chain, against an installed browser. Skips
//! cleanly when there is none.
//!
//! - Paint laid beside the content (an `absolute inset-0` gradient, a grain
//!   tile) decides a contrast verdict where its colours are known: dim copy
//!   on a dark gradient fails and is printed against it, emerald copy on it
//!   passes, and a pale link over a faint grain fails.
//! - Paint whose colours are not in the capture is read by the pixel pass:
//!   cream copy on a cream photo under a `pointer-events: none` layer, and a
//!   faint link over a photo below the fold, are flagged from pixels; a white
//!   caption on a scrim over a dark photo and outlined sticker type pass.
//! - A card photo faded to 0.16 over its white card is read faded.
//! - A glow over a light panel laid on a dark section is not on a dark
//!   background; one under a translucent vignette still is.
//! - A page laid out wider than the phone scores its right column.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use impeccable_browser::BrowserEngine;
use impeccable_detect::engines::{ScanOptions, UrlEngine};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/antipatterns")
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
    let body = std::fs::read(fixtures_dir().join(path.trim_start_matches('/'))).unwrap_or_default();
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

/// `(rule, selector, snippet)` for every finding of a scan.
fn scan(engine: &BrowserEngine, fixture: &str, viewport: Option<(u32, u32)>) -> Vec<(String, String, String)> {
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/{fixture}");
    let options = ScanOptions {
        viewport,
        ..ScanOptions::default()
    };
    engine
        .detect_url(&url, &options)
        .expect("scan")
        .iter()
        .map(|f| {
            (
                f.antipattern.clone(),
                f.extras
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                f.snippet.clone(),
            )
        })
        .collect()
}

fn snippet<'a>(flagged: &'a [(String, String, String)], rule: &str, selector: &str) -> Option<&'a str> {
    flagged
        .iter()
        .find(|(r, s, _)| r == rule && s == selector)
        .map(|(_, _, snippet)| snippet.as_str())
}

#[test]
fn surfaces_off_the_ancestor_chain_are_read_where_they_paint() {
    let Some(engine) = engine() else { return };
    let flagged = scan(&engine, "unread-surface-contrast.html", None);
    for selector in [
        "#flag-dim-on-dark-panel",
        "#flag-link-over-grain",
        "#flag-cream-copy-on-photo",
        "#flag-faint-link-below-fold",
    ] {
        assert!(
            snippet(&flagged, "low-contrast", selector).is_some(),
            "missing low-contrast on {selector} in {flagged:#?}"
        );
    }
    for selector in [
        "#pass-emerald-on-dark-panel",
        "#pass-caption-on-scrim",
        "#pass-svg-initial",
        "#pass-outline-sticker",
        "#pass-card-over-faded-photo",
    ] {
        assert!(
            snippet(&flagged, "low-contrast", selector).is_none(),
            "low-contrast on {selector} was flagged in {flagged:#?}"
        );
    }
    // Dim copy is printed against the gradient it sits on.
    let dim = snippet(&flagged, "low-contrast", "#flag-dim-on-dark-panel").unwrap_or_default();
    assert!(dim.ends_with("(layer on div.layer)") && !dim.contains("on #ffffff"), "{dim}");
    // What the walk never saw is read from pixels.
    for selector in ["#flag-cream-copy-on-photo", "#flag-faint-link-below-fold"] {
        let s = snippet(&flagged, "low-contrast", selector).unwrap_or_default();
        assert!(s.starts_with("pixel contrast "), "{selector}: {s}");
    }
    assert!(snippet(&flagged, "dark-glow", "#flag-glow-under-vignette").is_some(), "{flagged:#?}");
    assert!(snippet(&flagged, "dark-glow", "#pass-glow-on-light-panel").is_none(), "{flagged:#?}");
}

#[test]
fn a_page_wider_than_the_phone_scores_its_right_column() {
    let Some(engine) = engine() else { return };
    let flagged = scan(&engine, "wide-page-contrast.html", Some((390, 844)));
    assert!(
        snippet(&flagged, "low-contrast", "#flag-right-column-label").is_some(),
        "{flagged:#?}"
    );
    assert!(
        snippet(&flagged, "low-contrast", "#pass-parked-drawer-label").is_none(),
        "{flagged:#?}"
    );
}
