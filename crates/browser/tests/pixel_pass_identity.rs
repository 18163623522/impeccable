//! The pixel pass measures the element its candidate came from when the page
//! repeats the id the candidate's selector is anchored on. Rendered by an
//! installed browser; skips cleanly when there is none.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use impeccable_browser::BrowserEngine;
use impeccable_detect::engines::{ScanOptions, UrlEngine};

fn serve(body: String) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let body = body.clone();
            std::thread::spawn(move || handle(stream, &body));
        }
    });
    port
}

fn handle(mut stream: TcpStream, body: &str) {
    let mut buf = [0u8; 8192];
    let _ = stream.read(&mut buf);
    let head = format!(
        "HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

#[test]
fn a_repeated_id_does_not_send_the_pixel_pass_to_a_collapsed_copy() {
    let env: HashMap<String, String> = std::env::vars().collect();
    if impeccable_browser::discovery::find_browser(&env).is_err() {
        eprintln!("skip: no installed browser found");
        return;
    }
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/antipatterns/fullpage-screenshot-scroller.html");
    let html = std::fs::read_to_string(fixture).expect("fixture");
    // A collapsed copy of the vector-copy paragraph, first in the document:
    // `querySelector` answers with it, and it sits in the first viewport, so
    // scrolling it into view moves nothing.
    let html = html.replacen(
        "<body>",
        "<body>\n<div style=\"height:0;overflow:hidden\"><p id=\"flag-scroller-vector-copy\">Collapsed copy.</p></div>",
        1,
    );
    assert!(html.contains("Collapsed copy."));
    let port = serve(html);
    let engine = BrowserEngine::new(env);
    let url = format!("http://127.0.0.1:{port}/");
    let findings = engine.detect_url(&url, &ScanOptions::default()).expect("scan");
    let pixel: Vec<&str> = findings
        .iter()
        .filter(|f| {
            f.antipattern == "low-contrast"
                && f.extras.get("selector").and_then(|v| v.as_str()) == Some("#flag-scroller-vector-copy")
        })
        .map(|f| f.snippet.as_str())
        .collect();
    assert!(
        pixel.iter().any(|s| s.starts_with("pixel contrast ")),
        "the vector copy past the body scroller's fold was not measured: {pixel:?}"
    );
}
