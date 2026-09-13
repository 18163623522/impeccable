//! Findings whose evidence a corpus judge could not trust, over
//! `tests/fixtures/antipatterns/`, rendered by an installed browser. Skips
//! cleanly when there is none.
//!
//! - `visual-contrast.html`: the pixel pass reads glyph cores, so a readable
//!   date through an opacity stack passes, a faded one still fails, and no
//!   snippet prints a verdict above its own median.
//! - `script-error.html`: a thrown object names its properties and every
//!   script error names where it was thrown; a caught error reports nothing.
//! - `text-overflow.html`: an ellipsis, a line clamp and an inline run inside
//!   an ellipsizing row are truncations, not spills; a clipped line with no
//!   marker still reports.

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
    let rel = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .trim_start_matches('/')
        .to_string();
    let (status, body) = match std::fs::read(fixtures_dir().join(&rel)) {
        Ok(body) if !rel.contains("..") => ("200 OK", body),
        _ => ("404 Not Found", b"missing".to_vec()),
    };
    let head = format!(
        "HTTP/1.0 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

/// `(antipattern, snippet, selector)` for every finding on one fixture, or
/// `None` when no browser is installed.
fn scan(fixture: &str) -> Option<Vec<(String, String, String)>> {
    let env: HashMap<String, String> = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: no installed browser found");
        return None;
    }
    let engine = BrowserEngine::new(env);
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/{fixture}");
    let findings = engine.detect_url(&url, &ScanOptions::default()).expect("scan");
    Some(
        findings
            .iter()
            .map(|f| {
                let selector = f
                    .extras
                    .get("selector")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                (f.antipattern.clone(), f.snippet.clone(), selector)
            })
            .collect(),
    )
}

/// `pixel contrast X:1 median Y:1 ...` into `(X, Y)`.
fn pixel_pair(snippet: &str) -> Option<(f64, f64)> {
    let rest = snippet.strip_prefix("pixel contrast ")?;
    let (verdict, rest) = rest.split_once(":1 median ")?;
    let (median, _) = rest.split_once(":1")?;
    Some((verdict.parse().ok()?, median.parse().ok()?))
}

#[test]
fn pixel_contrast_reads_glyph_cores() {
    let Some(findings) = scan("visual-contrast.html") else {
        return;
    };
    let low: Vec<&str> = findings
        .iter()
        .filter(|(id, _, _)| id == "low-contrast")
        .map(|(_, s, _)| s.as_str())
        .collect();
    for snippet in &low {
        if let Some((verdict, median)) = pixel_pair(snippet) {
            assert!(verdict <= median, "verdict above its median: {snippet}");
        }
    }
    assert!(
        low.iter().any(|s| s.contains("\"Faded date through a heavy opacity stack\"")),
        "expected the faded date to flag, got {low:?}"
    );
    assert!(
        !low.iter().any(|s| s.contains("Readable date through a light opacity stack")),
        "the readable date flagged: {low:?}"
    );
}

#[test]
fn script_errors_name_the_thrown_value_and_where_it_was_thrown() {
    let Some(findings) = scan("script-error.html") else {
        return;
    };
    let errors: Vec<&str> = findings
        .iter()
        .filter(|(id, _, _)| id == "script-error")
        .map(|(_, s, _)| s.as_str())
        .collect();
    assert!(
        errors.iter().any(|s| s.starts_with(
            "Object {code: \"E_CONSENT\", message: \"consent config missing\"} (at loadConsentConfig, http://127.0.0.1:"
        ) && s.contains("/script-error.html:")),
        "expected the thrown object with its source, got {errors:?}"
    );
    assert!(errors.len() >= 2, "expected the syntax error too, got {errors:?}");
    for snippet in &errors {
        assert!(
            snippet.contains("/script-error.html:"),
            "script error without a source: {snippet}"
        );
    }
    assert!(
        !errors.iter().any(|s| s.contains("handled quietly")),
        "a caught error reported: {errors:?}"
    );
}

#[test]
fn text_overflow_skips_marked_truncation_and_keeps_real_spills() {
    let Some(findings) = scan("text-overflow.html") else {
        return;
    };
    let overflow: Vec<(&str, &str)> = findings
        .iter()
        .filter(|(id, _, _)| id == "text-overflow")
        .map(|(_, s, sel)| (s.as_str(), sel.as_str()))
        .collect();
    for flag in ["flag-nowrap", "flag-longword", "flag-inline-spill", "flag-hidden-no-marker"] {
        assert!(
            overflow.iter().any(|(s, sel)| s.contains(flag) || sel.contains(flag)),
            "expected {flag} to flag, got {overflow:?}"
        );
    }
    let stray: Vec<&(&str, &str)> = overflow
        .iter()
        .filter(|(s, sel)| s.contains("pass-") || sel.contains("pass-") || sel.contains("ellipsis"))
        .collect();
    assert!(stray.is_empty(), "should-pass boxes flagged: {stray:?}");
    assert_eq!(overflow.len(), 4, "{overflow:?}");
}
