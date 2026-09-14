//! The text geometry rules against an installed browser, on the fixtures'
//! should-flag and should-pass cases. Skips cleanly when there is none.
//!
//! `line-length`, `body-text-viewport-edge`, `tight-leading` and
//! `cramped-padding` measure where the text is (its Range client rects and
//! the line boxes it sits on) rather than the box around it; `text-overflow`
//! confirms a spill from what paints past the box and reads a wrapper's
//! `overflow-x`; `edge-flush-cards` takes only x scrollers that hold a row of
//! cards. None of that is visible to the static engine, which measures no
//! boxes.

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
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();
    let (status, body) = match std::fs::read(fixtures_dir().join(path.trim_start_matches('/'))) {
        Ok(body) => ("200 OK", body),
        Err(_) => ("404 Not Found", b"missing".to_vec()),
    };
    let head = format!(
        "HTTP/1.0 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

fn engine() -> Option<BrowserEngine> {
    let env: HashMap<String, String> = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: no installed browser found");
        return None;
    }
    Some(BrowserEngine::new(env))
}

/// `(snippet, selector)` for each finding of `rule` on one fixture.
fn findings(engine: &BrowserEngine, port: u16, fixture: &str, rule: &str) -> Vec<(String, String)> {
    let url = format!("http://127.0.0.1:{port}/{fixture}");
    engine
        .detect_url(&url, &ScanOptions::default())
        .expect("scan")
        .into_iter()
        .filter(|f| f.antipattern == rule)
        .map(|f| {
            let selector = f.extras.get("selector").and_then(|s| s.as_str()).unwrap_or("").to_string();
            (f.snippet, selector)
        })
        .collect()
}

fn assert_cases(found: &[(String, String)], flag: &[&str], pass: &[&str], rule: &str) {
    for case in flag {
        assert!(
            found.iter().any(|(_, sel)| sel.contains(case)),
            "{rule}: `{case}` should flag, got {found:?}"
        );
    }
    for case in pass {
        assert!(
            !found.iter().any(|(_, sel)| sel.contains(case)),
            "{rule}: `{case}` should pass, got {found:?}"
        );
    }
}

#[test]
fn the_text_geometry_rules_measure_the_text() {
    let Some(engine) = engine() else { return };
    let port = serve();

    // The generated selectors name the wide paragraphs by position; the
    // one-line note, the centred lines, the 16px CJK column and the narrow
    // measure are absent.
    let mut lines = findings(&engine, port, "line-length.html", "line-length");
    lines.sort();
    assert_eq!(
        lines,
        vec![
            ("~100 chars/line (aim for <80)".to_string(), "p.copy.flag-large-span".to_string()),
            ("~101 chars/line (aim for <80)".to_string(), "p.copy.cjk-wide".to_string()),
            ("~119 chars/line (aim for <80)".to_string(), "p.copy.flag-mono".to_string()),
            ("~125 chars/line (aim for <80)".to_string(), "p.copy.wide:nth-of-type(1)".to_string()),
            ("~125 chars/line (aim for <80)".to_string(), "p.copy.wide:nth-of-type(2)".to_string()),
            ("~125 chars/line (aim for <80)".to_string(), "p.copy.wide:nth-of-type(3)".to_string()),
        ],
        "the wide column, its inline-prose twin, its 2.4 line-height twin, the 12px CJK column, the \
         24px span across 1,200px and the monospace column; the 16px paragraph set in a 24px span \
         and the 680px monospace column hold under 85 characters a line"
    );

    let edges = findings(&engine, port, "body-text-viewport-edge.html", "body-text-viewport-edge");
    assert_cases(
        &edges,
        &[
            "div.escape:nth-of-type(1) > p",
            "div.escape:nth-of-type(2) > p",
            "li",
            "flag-inline-prose",
            "flag-cut-by-wrapper",
            "flag-runs-past",
            "flag-transformed-wrapper",
            "flag-shell-sliver",
        ],
        &[
            "pass-centred-text",
            "pass-padded-text",
            "pass-clipped-slide",
            "pass-past-viewport",
            "pass-ticker-first",
            "pass-ticker-second",
        ],
        "body-text-viewport-edge",
    );
    assert_eq!(
        edges.len(),
        8,
        "two paragraphs, the list item, the inline prose, the paragraph an overflow-x-hidden wrapper cuts, \
         the column running past the window, the paragraph in a transformed wrapper that holds no row and \
         the non-wrapping row's second column the page wrapper cuts under a quarter in view: {edges:?}"
    );

    let leading = findings(&engine, port, "tight-leading.html", "tight-leading");
    assert_cases(&leading, &["card-blurb", "nested-desc"], &["teaser-copy", "link-run"], "tight-leading");
    assert_eq!(leading.len(), 5, "the five flag cases: {leading:?}");

    let cramped = findings(&engine, port, "cramped-padding.html", "cramped-padding");
    assert_cases(&cramped, &["flag-card-4"], &["pass-highlight"], "cramped-padding");
}

#[test]
fn text_overflow_and_edge_flush_cards_read_the_x_axis() {
    let Some(engine) = engine() else { return };
    let port = serve();

    let overflow = findings(&engine, port, "text-overflow.html", "text-overflow");
    assert_cases(
        &overflow,
        &["flag-nowrap", "flag-in-x-hidden-wrapper", "flag-pseudo-suffix"],
        &["pass-ripple", "pass-image-replacement", "pass-reserve"],
        "text-overflow",
    );
    assert!(
        overflow.iter().all(|(snippet, _)| snippet.contains("flag-")),
        "only flag cases report: {overflow:?}"
    );

    let flush = findings(&engine, port, "edge-flush-cards.html", "edge-flush-cards");
    assert_eq!(flush.len(), 1, "only the pager: {flush:?}");
    assert!(flush[0].0.contains("flag-pager"), "{flush:?}");
}
