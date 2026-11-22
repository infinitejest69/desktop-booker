//! Dev-only **demo bridge** (Phase 11): runs the app's real commands, with a
//! real database, behind a small local web server, so Playwright can drive
//! the app in Chromium for screenshots and demo videos. Never shipped: it is
//! only built with `--features demo-bridge`.
//!
//! The browser side is `demo/bridge-client.js`: it stands in for Tauri's
//! `invoke` and POSTs each command to `http://127.0.0.1:<port>/invoke/<name>`
//! with the arguments as JSON. Here each request goes through Tauri's mock
//! runtime (`tauri::test`), so the same permission checks and code run as in
//! the app.
//!
//! ```text
//! cargo run -p desktop-booker --bin demo-bridge --features demo-bridge -- \
//!     --db /tmp/demo.db --fresh --port 1430
//! ```

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use booker_store::SqliteStore;
use desktop_booker_lib::{DbPath, Session, invoke_handler};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;

struct Options {
    db: PathBuf,
    port: u16,
    fresh: bool,
}

fn options() -> Options {
    let mut o = Options {
        db: std::env::temp_dir().join("desktop-booker-demo.db"),
        port: 1430,
        fresh: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--db" => o.db = args.next().map(PathBuf::from).unwrap_or(o.db),
            "--port" => o.port = args.next().and_then(|p| p.parse().ok()).unwrap_or(o.port),
            "--fresh" => o.fresh = true,
            other => eprintln!("[demo-bridge] ignoring {other}"),
        }
    }
    o
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = options();
    if o.fresh {
        // A brand-new database each run (and its WAL side files).
        for suffix in ["", "-wal", "-shm"] {
            let mut path = o.db.clone().into_os_string();
            path.push(suffix);
            let _ = std::fs::remove_file(path);
        }
    }
    let store = tauri::async_runtime::block_on(SqliteStore::open(&o.db))?;
    let app = mock_builder()
        // Never lock for being idle: a recording can pause for a while.
        .manage(Session::new(0))
        .manage(store)
        .manage(DbPath(o.db.clone()))
        .invoke_handler(invoke_handler())
        // The app's own config and permissions, so commands are allowed
        // exactly as they are in the app.
        .build(tauri::generate_context!("tauri.conf.json", test = true))?;
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;

    let listener = TcpListener::bind(("127.0.0.1", o.port))?;
    println!(
        "[demo-bridge] database {} · listening on http://127.0.0.1:{}",
        o.db.display(),
        o.port
    );
    // One request at a time is plenty for a scripted browser.
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[demo-bridge] connection failed: {e}");
                continue;
            }
        };
        if let Err(e) = serve(stream, &webview) {
            eprintln!("[demo-bridge] request failed: {e}");
        }
    }
    Ok(())
}

/// Reads one HTTP request, runs the command, writes the JSON answer.
fn serve(
    mut stream: TcpStream,
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    let (status, answer) = match (method, path.strip_prefix("/invoke/")) {
        // The browser asks first whether it may POST from another port.
        ("OPTIONS", _) => ("204 No Content", String::new()),
        ("POST", Some(cmd)) => invoke(webview, cmd, &body),
        _ => (
            "404 Not Found",
            r#"{"kind":"internal","message":"POST /invoke/<command>"}"#.into(),
        ),
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: content-type\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{answer}",
        answer.len()
    )?;
    stream.flush()
}

fn invoke(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: &[u8],
) -> (&'static str, String) {
    let args: serde_json::Value = if body.is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                let error = serde_json::json!({ "kind": "internal", "message": e.to_string() });
                return ("400 Bad Request", error.to_string());
            }
        }
    };
    let request = InvokeRequest {
        cmd: cmd.to_string(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        // The webview's own address, so Tauri treats the call as coming
        // from the app itself (a remote page couldn't call app commands).
        url: webview
            .url()
            .unwrap_or_else(|_| "tauri://localhost".parse().expect("a valid URL")),
        body: InvokeBody::Json(args),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    };
    match get_ipc_response(webview, request) {
        Ok(response) => match response.deserialize::<serde_json::Value>() {
            Ok(value) => ("200 OK", value.to_string()),
            Err(e) => ("500 Internal Server Error", e.to_string()),
        },
        // The command's own error, e.g. {"kind":"validation","message":"…"}.
        Err(error) => ("400 Bad Request", error.to_string()),
    }
}
