//! `line-length` counts the characters that reached the rendered lines, on
//! `line-length-text-count.html`, against an installed browser. Skips cleanly
//! when there is none.
//!
//! The rule divides an element's characters among its line rects. Counted
//! from `textContent`, a paragraph with an inline `<style>` child, deep
//! source indentation or a script written with combining marks was charged
//! with characters that are on no line, and a comfortable measure read as a
//! long column.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use impeccable_browser::BrowserEngine;
use impeccable_detect::engines::{ScanOptions, UrlEngine};

const FIXTURE: &str = "line-length-text-count.html";

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/antipatterns").join(FIXTURE)
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

/// Serves the one fixture this test scans at `/<FIXTURE>` and nothing else,
/// so no request path reaches the rest of the disk.
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
    let fixture = (path.strip_prefix('/') == Some(FIXTURE))
        .then(|| std::fs::read(fixture_path()).ok())
        .flatten();
    let (status, body) = match fixture {
        Some(body) => ("200 OK", body),
        None => ("404 Not Found", b"missing".to_vec()),
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

/// `(snippet, selector)` for each finding of `rule` on the fixture.
fn findings(engine: &BrowserEngine, port: u16, rule: &str) -> Vec<(String, String)> {
    let url = format!("http://127.0.0.1:{port}/{FIXTURE}");
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

/// The `class` attribute of every `<p>` on the fixture page, in order. The
/// paragraphs are all children of `<body>`, so the n-th is
/// `p:nth-of-type(n)`.
fn paragraph_classes() -> Vec<Vec<String>> {
    let html = std::fs::read_to_string(fixture_path()).expect("fixture");
    html.split("<p class=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or("").split_whitespace().map(str::to_string).collect())
        .collect()
}

/// The classes of the paragraph a finding's selector names. The selector's
/// last segment carries the paragraph's first two classes, and an
/// `:nth-of-type` index when those two do not pick it out alone.
fn classes_of(selector: &str, paragraphs: &[Vec<String>]) -> Vec<String> {
    let segment = selector.rsplit('>').next().unwrap_or("").trim();
    let (head, nth) = match segment.split_once(":nth-of-type(") {
        Some((head, rest)) => (head, rest.trim_end_matches(')').parse::<usize>().ok()),
        None => (segment, None),
    };
    if let Some(n) = nth {
        return paragraphs.get(n.wrapping_sub(1)).cloned().unwrap_or_default();
    }
    let named: Vec<&str> = head.split('.').skip(1).collect();
    let mut matching = paragraphs.iter().filter(|p| named.iter().all(|c| p.iter().any(|pc| pc == c)));
    let first = matching.next().cloned().unwrap_or_default();
    assert!(matching.next().is_none(), "{selector} names more than one paragraph");
    first
}

/// The snippet of the finding on the paragraph with class `class`, if any.
fn snippet_for(found: &[(String, String)], class: &str) -> Option<String> {
    let paragraphs = paragraph_classes();
    let mut hits = found
        .iter()
        .filter(|(_, selector)| classes_of(selector, &paragraphs).iter().any(|c| c == class));
    let hit = hits.next().map(|(snippet, _)| snippet.clone());
    assert!(hits.next().is_none(), "more than one finding names .{class}: {found:?}");
    hit
}

/// Every case is compared with a plain twin on the same page rather than with
/// a fixed count: the fixture sets `system-ui`, so where the lines wrap, and
/// with it the count per line, is the host's font's business. What the rule
/// owes is that a case reports exactly what its twin reports. Counted from
/// `textContent`, no case read what its twin reads: the style child's CSS,
/// the hidden child and the script, and the indentation were charged to the
/// lines, and the nested `pre-wrap` span's runs of spaces were folded away.
/// The Devanagari paragraph has no twin and whether it flags is up to the
/// font, so it is not asserted here; the unit tests pin the marks.
#[test]
fn line_length_counts_the_characters_on_the_lines() {
    let Some(engine) = engine() else { return };
    let port = serve();
    let found = findings(&engine, port, "line-length");

    // A 1000px column at 16px is a long column in any font.
    let wide = snippet_for(&found, "plain-wide");
    assert!(wide.is_some(), "the plain wide column flags: {found:?}");
    for case in ["flag-style-child", "flag-indented"] {
        assert_eq!(snippet_for(&found, case), wide, ".{case} reads as its plain twin: {found:?}");
    }

    // Whether a 560px measure flags depends on the font; that each case
    // reads the same as the plain one does not.
    let measure = snippet_for(&found, "plain-measure");
    for case in ["pass-style-child", "pass-hidden-child", "pass-indented"] {
        assert_eq!(snippet_for(&found, case), measure, ".{case} reads as its plain twin: {found:?}");
    }

    // The preserved runs of spaces are on the line, whether the paragraph
    // preserves them or a span inside a normal paragraph does.
    let preserved = snippet_for(&found, "flag-preserved");
    assert!(preserved.is_some(), "the pre-wrap column flags: {found:?}");
    assert_eq!(snippet_for(&found, "flag-preserved-nested"), preserved, "{found:?}");
}
