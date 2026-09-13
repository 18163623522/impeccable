//! `heading-rhythm` over `tests/fixtures/antipatterns/heading-rhythm.html`,
//! rendered by an installed browser. Skips cleanly when there is none.
//!
//! The should-pass column holds the shapes a live recapture of real sites
//! misread once the rule pass ran after the reveal sweep: an eyebrow wrapped
//! in its own box, a block that ends in a rule, an accordion trigger that ends
//! its row, a caption under its photo, a title band that draws its own rules,
//! and a standfirst behind a `display: contents` wrapper.

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

#[test]
fn fixture_flags_crowded_headings_and_passes_the_misread_shapes() {
    let env: HashMap<String, String> = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: no installed browser found");
        return;
    }
    let engine = BrowserEngine::new(env);
    let port = serve();
    let url = format!("http://127.0.0.1:{port}/heading-rhythm.html");
    let findings = engine.detect_url(&url, &ScanOptions::default()).expect("scan");
    let snippets: Vec<&str> = findings
        .iter()
        .filter(|f| f.antipattern == "heading-rhythm")
        .map(|f| f.snippet.as_str())
        .collect();

    for heading in [
        "\"Flag Crowded One\"",
        "\"Flag Crowded Two\"",
        "\"Flag Crowded Three\"",
        "\"Flag Wrapped Section Title\"",
        // The wrapper spacing column: the title's wrapper, a stretched row, a
        // grid column or a spacer holds the gap, and it is still measured.
        "\"Flag Header Padding\" has 16px above vs 48px below",
        "\"Flag Block Padding\" has 16px above vs 48px below",
        "\"Flag Small Padding\" has 16px above vs 48px below",
        "\"Flag Icon Beside Heading\" has 22px above vs 54px below",
        "\"Flag Tall Button Row\" has 26px above vs 58px below",
        "\"Flag Column Heading\" has 12px above vs 48px below",
        "\"Flag Spacer Below\" has 10px above vs 48px below",
        "\"Flag Stack Heading\" has 16px above vs 48px below",
        // A date line set like body copy is not folded in as a label.
        "\"Flag Plain Line Above\" has 10px above vs 48px below",
        // A code block framed on every side is content, not a rule, and a
        // layout row that shares a class with its neighbour repeats nothing.
        "\"Flag Code Block Above\" has 12px above vs 36px below",
        "\"Flag Title Alone In A Row\" has 16px above vs 48px below",
    ] {
        assert!(
            snippets.iter().any(|s| s.contains(heading)),
            "expected {heading:?} to flag, got {snippets:?}"
        );
    }
    let stray: Vec<&&str> = snippets.iter().filter(|s| s.contains("\"Pass ")).collect();
    assert!(stray.is_empty(), "should-pass headings flagged: {stray:?}");
    assert_eq!(snippets.len(), 15, "{snippets:?}");
}
