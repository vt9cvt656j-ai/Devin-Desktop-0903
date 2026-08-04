//! Desktop → browser session handoff.
//!
//! When the web sign-in page loads, it asks the running desktop app whether anyone is
//! signed in. If so, "Continue with Mr.day One" adopts that session instead of making
//! the user retype a password they already typed here.
//!
//! Everything below is shaped by one fact: ANY page the user visits can send requests
//! to 127.0.0.1. The session token leaves this process only for an origin on the
//! allowlist, and three independent things enforce that:
//!
//!   * the socket binds to loopback, so nothing off this machine can reach it at all;
//!   * every route requires `X-MrDay-Handoff`, a non-safelisted header, which forces
//!     browsers to preflight — a disallowed origin is refused at OPTIONS and the real
//!     request is never sent;
//!   * `Origin` is checked here in Rust on every request, not merely reflected back in
//!     a CORS header, so a hand-rolled client gains nothing by skipping the preflight.
//!
//! A local process running as the user could read the token off disk anyway, so the
//! threat this defends against is the realistic one: a web page the user happens to
//! have open trying to lift the session.

use serde_json::json;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;
use tauri::command;

/// Probed in order by this listener and by the web page. Deliberately below the
/// ephemeral range (49152+ on macOS) so an outbound socket can never squat on them.
/// Kept short on purpose: the page probes these one at a time, and every port that
/// answers nothing is a red line in the visitor's console. Three covers a dev build
/// and a release build running side by side, which is the only realistic collision.
const PORTS: [u16; 3] = [47821, 47822, 47823];

const ALLOWED_ORIGINS: &[&str] = &["https://code.mrday.one"];

/// The marketing site's dev server, so this flow can be worked on locally. Debug
/// builds only — a shipped app must not vouch for whatever is on a local port.
#[cfg(debug_assertions)]
const DEV_ORIGINS: &[&str] = &["http://localhost:5273", "http://127.0.0.1:5273"];
#[cfg(not(debug_assertions))]
const DEV_ORIGINS: &[&str] = &[];

/// Requests are tiny and fixed-shape; anything larger is not one of ours.
const MAX_REQUEST_BYTES: usize = 8192;

struct Session {
    token: String,
    email: String,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Mirrors the frontend's account state into this process. Passing `None` clears it,
/// so signing out of the app immediately stops it vouching for anyone.
#[command]
pub fn handoff_set_session(token: Option<String>, email: Option<String>) {
    let next = match (token, email) {
        (Some(t), Some(e)) if !t.is_empty() && !e.is_empty() => Some(Session { token: t, email: e }),
        _ => None,
    };
    if let Ok(mut guard) = SESSION.lock() {
        *guard = next;
    }
}

/// Binds the first free candidate port. A failure here disables handoff and nothing
/// else — the web page simply falls back to asking for a password.
pub fn start() {
    let bound = PORTS
        .iter()
        .find_map(|p| TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, *p)).ok());

    let Some(listener) = bound else {
        tracing::warn!("Handoff: all candidate ports busy; browser sign-in handoff is off");
        return;
    };

    match listener.local_addr() {
        Ok(addr) => tracing::info!("Handoff listening on {addr}"),
        Err(_) => tracing::info!("Handoff listening on loopback"),
    }

    serve(listener);
}

/// Split out from `start` so tests can drive the real accept loop on an ephemeral
/// port instead of fighting over the fixed ones.
fn serve(listener: TcpListener) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            // One thread per connection: a client that opens a socket and then stalls
            // must not hold the accept loop hostage until its timeout expires.
            std::thread::spawn(move || handle(stream));
        }
    });
}

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

fn handle(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let Some(req) = read_request(&mut stream) else { return };
    let origin = header(&req.headers, "origin").unwrap_or_default();

    if !ALLOWED_ORIGINS.iter().chain(DEV_ORIGINS).any(|o| *o == origin) {
        // No CORS headers on this path: an unrecognised caller learns nothing, not
        // even whether anyone is signed in.
        respond(&mut stream, "403 Forbidden", None, r#"{"error":"origin not allowed"}"#);
        return;
    }

    if req.method == "OPTIONS" {
        respond(&mut stream, "200 OK", Some(&origin), "{}");
        return;
    }

    // Present on every real request. Its absence means the caller skipped the
    // preflight the browser would have forced, so it is not the sign-in page.
    if header(&req.headers, "x-mrday-handoff").is_none() {
        respond(&mut stream, "400 Bad Request", Some(&origin), r#"{"error":"missing handoff header"}"#);
        return;
    }

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/session") => {
            let guard = SESSION.lock().ok();
            let session = guard.as_ref().and_then(|g| g.as_ref());
            let body = json!({
                "app": "mrday-one",
                "version": env!("CARGO_PKG_VERSION"),
                "signedIn": session.is_some(),
                "email": session.map(|s| s.email.clone()),
            });
            respond(&mut stream, "200 OK", Some(&origin), &body.to_string());
        }
        ("POST", "/handoff") => {
            let token = SESSION
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|s| s.token.clone()));
            match token {
                Some(token) => {
                    let body = json!({ "token": token, "app": "mrday-one" });
                    respond(&mut stream, "200 OK", Some(&origin), &body.to_string());
                }
                None => respond(
                    &mut stream,
                    "409 Conflict",
                    Some(&origin),
                    r#"{"error":"no desktop session"}"#,
                ),
            }
        }
        _ => respond(&mut stream, "404 Not Found", Some(&origin), r#"{"error":"no such route"}"#),
    }
}

/// Reads until the end of the header block. The body is never read — no route takes
/// one, and the response closes the connection regardless.
fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 512];
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_REQUEST_BYTES {
            return None;
        }
    }

    let text = String::from_utf8_lossy(&buf).into_owned();
    let mut lines = text.split("\r\n");
    let mut start = lines.next()?.split_whitespace();
    let method = start.next()?.to_string();
    // Ignore any query string: routes here take no parameters.
    let path = start.next()?.split('?').next().unwrap_or_default().to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }

    Some(Request { method, path, headers })
}

fn header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

fn respond(stream: &mut TcpStream, status: &str, allow_origin: Option<&str>, body: &str) {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    head.push_str("Connection: close\r\n");
    head.push_str("Cache-Control: no-store\r\n");
    // The allowlist makes the response origin-dependent, so it must never be shared.
    head.push_str("Vary: Origin\r\n");
    if let Some(origin) = allow_origin {
        head.push_str(&format!("Access-Control-Allow-Origin: {origin}\r\n"));
        head.push_str("Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n");
        head.push_str("Access-Control-Allow-Headers: content-type, x-mrday-handoff\r\n");
        // Chrome's Private Network Access check: a public HTTPS page reaching loopback
        // is refused at preflight without this, which is the failure mode that makes
        // integrations like this one silently stop working.
        head.push_str("Access-Control-Allow-Private-Network: true\r\n");
        head.push_str("Access-Control-Max-Age: 600\r\n");
    }
    head.push_str("Content-Type: application/json\r\n");
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    const GOOD: &str = "https://code.mrday.one";

    /// Drives the real accept loop over a real socket and returns the raw response,
    /// so what is asserted below is the bytes a browser would actually receive.
    fn request(port: u16, raw: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(raw.as_bytes()).unwrap();
        s.flush().unwrap();
        let mut out = String::new();
        // The handler always closes, so read-to-end terminates.
        let _ = BufReader::new(s).read_to_string(&mut out);
        out
    }

    fn get_session(port: u16, origin: &str) -> String {
        request(
            port,
            &format!("GET /session HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {origin}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
        )
    }

    /// One test, not several: every case shares the process-wide SESSION, so running
    /// them as separate #[test] fns would let cargo's thread pool race them.
    #[test]
    fn the_listener_only_ever_gives_the_token_to_the_sign_in_page() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let port = listener.local_addr().unwrap().port();
        serve(listener);

        // --- signed out -----------------------------------------------------
        handoff_set_session(None, None);
        let res = get_session(port, GOOD);
        assert!(res.starts_with("HTTP/1.1 200"), "{res}");
        assert!(res.contains(r#""signedIn":false"#), "{res}");
        assert!(res.contains("Access-Control-Allow-Origin: https://code.mrday.one"), "{res}");

        let res = request(
            port,
            &format!("POST /handoff HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
        );
        assert!(res.starts_with("HTTP/1.1 409"), "signed out means no token to hand over: {res}");
        assert!(!res.contains("secret-token"), "{res}");

        // --- half a session is not a session --------------------------------
        handoff_set_session(Some("secret-token".into()), None);
        assert!(get_session(port, GOOD).contains(r#""signedIn":false"#));
        handoff_set_session(None, Some("dev@example.com".into()));
        assert!(get_session(port, GOOD).contains(r#""signedIn":false"#));
        handoff_set_session(Some(String::new()), Some("dev@example.com".into()));
        assert!(get_session(port, GOOD).contains(r#""signedIn":false"#), "an empty token is not a session");

        // --- signed in ------------------------------------------------------
        handoff_set_session(Some("secret-token".into()), Some("dev@example.com".into()));
        let res = get_session(port, GOOD);
        assert!(res.contains(r#""signedIn":true"#), "{res}");
        assert!(res.contains("dev@example.com"), "{res}");
        assert!(!res.contains("secret-token"), "/session reports who, never the token: {res}");

        let res = request(
            port,
            &format!("POST /handoff HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
        );
        assert!(res.starts_with("HTTP/1.1 200"), "{res}");
        assert!(res.contains("secret-token"), "{res}");

        // --- a page from anywhere else gets nothing --------------------------
        for hostile in ["https://evil.example", "null", "http://code.mrday.one"] {
            let res = get_session(port, hostile);
            assert!(res.starts_with("HTTP/1.1 403"), "{hostile} must be refused: {res}");
            assert!(
                !res.contains("Access-Control-Allow-Origin"),
                "a refused origin must not be handed a CORS grant: {res}"
            );
            assert!(!res.contains("dev@example.com"), "{hostile} learned who is signed in: {res}");

            let res = request(
                port,
                &format!("POST /handoff HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {hostile}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
            );
            assert!(!res.contains("secret-token"), "{hostile} lifted the session token: {res}");
        }

        // A request with no Origin at all is not the sign-in page either.
        let res = request(port, "GET /session HTTP/1.1\r\nHost: 127.0.0.1\r\nX-MrDay-Handoff: 1\r\n\r\n");
        assert!(res.starts_with("HTTP/1.1 403"), "{res}");

        // --- the header that forces the preflight ---------------------------
        let res = request(port, &format!("GET /session HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\n\r\n"));
        assert!(res.starts_with("HTTP/1.1 400"), "the handoff header is required: {res}");

        // --- preflight ------------------------------------------------------
        let res = request(
            port,
            &format!("OPTIONS /handoff HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\nAccess-Control-Request-Method: POST\r\nAccess-Control-Request-Private-Network: true\r\n\r\n"),
        );
        assert!(res.starts_with("HTTP/1.1 200"), "{res}");
        assert!(
            res.contains("Access-Control-Allow-Private-Network: true"),
            "without this Chrome refuses a public page reaching loopback: {res}"
        );
        assert!(res.contains("Access-Control-Allow-Headers: content-type, x-mrday-handoff"), "{res}");

        // --- everything else --------------------------------------------------
        let res = request(
            port,
            &format!("GET /../etc/passwd HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
        );
        assert!(res.starts_with("HTTP/1.1 404"), "{res}");

        // --- signing out revokes it immediately -------------------------------
        handoff_set_session(None, None);
        let res = request(
            port,
            &format!("POST /handoff HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {GOOD}\r\nX-MrDay-Handoff: 1\r\n\r\n"),
        );
        assert!(res.starts_with("HTTP/1.1 409"), "{res}");
        assert!(!res.contains("secret-token"), "a logged-out app still handed over the token: {res}");
    }

    #[test]
    fn a_release_build_trusts_only_the_sign_in_page() {
        assert_eq!(ALLOWED_ORIGINS, &["https://code.mrday.one"]);
        #[cfg(not(debug_assertions))]
        assert!(DEV_ORIGINS.is_empty(), "a shipped app must not trust a local dev server");
    }

    /// Not part of the suite. Run it to hold the real listener open on the real ports
    /// so a browser can be pointed at the live sign-in page and the whole handoff
    /// exercised end to end:
    ///   cargo test --lib handoff::tests::live_listener -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live_listener_for_browser_testing() {
        start();
        handoff_set_session(Some("live-test-token".into()), Some("dev@example.com".into()));
        println!("handoff listening; signed in as dev@example.com");
        std::thread::sleep(Duration::from_secs(120));
    }

    #[test]
    fn candidate_ports_sit_below_the_ephemeral_range() {
        // Above 49152 the OS could hand one of these to an unrelated outbound socket
        // first, which would silently disable handoff on a busy machine.
        assert!(PORTS.iter().all(|p| *p > 1024 && *p < 49152), "{PORTS:?}");
    }
}
