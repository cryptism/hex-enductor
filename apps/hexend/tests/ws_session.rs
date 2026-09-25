use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

async fn start_server() -> u16 {
    let app = hexend::server::app(hexend::session::new_sessions());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    port
}

#[tokio::test]
async fn round_trips_a_command_over_ws_and_persists_it() {
    let port = start_server().await;

    let tmp = tempfile_project().await;
    let url = format!(
        "ws://127.0.0.1:{port}/ws?path={}",
        urlencoding_lite(tmp.to_str().unwrap())
    );

    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.expect("connect");

    let initial = ws.next().await.expect("initial message").expect("ok");
    let Message::Text(initial_text) = initial else { panic!("expected text") };
    assert!(initial_text.contains("Demo Realm"), "{initial_text}");

    let command = serde_json::json!({
        "command": {
            "saveLocationContent": {
                "locationId": "notice-board",
                "patch": { "title": "Updated Notice Board" }
            }
        }
    });
    ws.send(Message::Text(command.to_string())).await.expect("send command");

    let updated = ws.next().await.expect("updated message").expect("ok");
    let Message::Text(updated_text) = updated else { panic!("expected text") };
    assert!(updated_text.contains("Updated Notice Board"), "{updated_text}");

    // persist() fires the write on a spawned task, not inline with the
    // broadcast — give it a moment to land before reading the file back.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let saved = tokio::fs::read_to_string(&tmp).await.unwrap();
    assert!(saved.contains("Updated Notice Board"));
}

#[tokio::test]
async fn rebroadcasts_a_ping_to_every_socket_without_touching_state() {
    let port = start_server().await;

    let tmp = tempfile_project().await;
    let url = format!(
        "ws://127.0.0.1:{port}/ws?path={}",
        urlencoding_lite(tmp.to_str().unwrap())
    );

    let (mut a, _) = tokio_tungstenite::connect_async(&url).await.expect("connect a");
    a.next().await.expect("a initial").expect("ok");
    let (mut b, _) = tokio_tungstenite::connect_async(&url).await.expect("connect b");
    b.next().await.expect("b initial").expect("ok");

    let ping = serde_json::json!({
        "ping": { "locationId": "town", "x": 12.5, "y": 34.5 }
    });
    a.send(Message::Text(ping.to_string())).await.expect("send ping");

    for ws in [&mut a, &mut b] {
        let msg = ws.next().await.expect("ping broadcast").expect("ok");
        let Message::Text(text) = msg else { panic!("expected text") };
        assert!(text.contains("\"ping\""), "{text}");
        assert!(text.contains("town") && text.contains("12.5"), "{text}");
    }

    // Never touched the project — nothing should have been (re)written to disk.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let saved = tokio::fs::read_to_string(&tmp).await.unwrap();
    assert!(!saved.contains("12.5"));
}

#[tokio::test]
async fn rebroadcasts_a_follow_view_to_every_socket_without_touching_state() {
    let port = start_server().await;

    let tmp = tempfile_project().await;
    let url = format!(
        "ws://127.0.0.1:{port}/ws?path={}",
        urlencoding_lite(tmp.to_str().unwrap())
    );

    let (mut a, _) = tokio_tungstenite::connect_async(&url).await.expect("connect a");
    a.next().await.expect("a initial").expect("ok");
    let (mut b, _) = tokio_tungstenite::connect_async(&url).await.expect("connect b");
    b.next().await.expect("b initial").expect("ok");

    let view = serde_json::json!({
        "followView": { "locationId": "town", "x": 100.0, "y": 200.0, "zoom": 1.5 }
    });
    a.send(Message::Text(view.to_string())).await.expect("send followView");

    for ws in [&mut a, &mut b] {
        let msg = ws.next().await.expect("followView broadcast").expect("ok");
        let Message::Text(text) = msg else { panic!("expected text") };
        assert!(text.contains("\"followView\""), "{text}");
        assert!(text.contains("town") && text.contains("1.5"), "{text}");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    let saved = tokio::fs::read_to_string(&tmp).await.unwrap();
    assert!(!saved.contains("1.5"));
}

#[tokio::test]
async fn adds_an_orphan_location_over_ws() {
    let port = start_server().await;

    let tmp = tempfile_project().await;
    let url = format!(
        "ws://127.0.0.1:{port}/ws?path={}",
        urlencoding_lite(tmp.to_str().unwrap())
    );

    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.expect("connect");
    ws.next().await.expect("initial message").expect("ok");

    let add = serde_json::json!({
        "command": { "addLocation": { "locationId": "staged-map" } }
    });
    ws.send(Message::Text(add.to_string())).await.expect("send addLocation");
    let after_add = ws.next().await.expect("state after addLocation").expect("ok");
    let Message::Text(after_add_text) = after_add else { panic!("expected text") };
    assert!(after_add_text.contains("staged-map"), "{after_add_text}");

    tokio::time::sleep(Duration::from_millis(200)).await;
    let saved = tokio::fs::read_to_string(&tmp).await.unwrap();
    assert!(saved.contains("staged-map"));
}

#[tokio::test]
async fn starts_and_paints_fog_over_ws() {
    let port = start_server().await;

    let tmp = tempfile_project().await;
    let url = format!(
        "ws://127.0.0.1:{port}/ws?path={}",
        urlencoding_lite(tmp.to_str().unwrap())
    );

    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.expect("connect");
    ws.next().await.expect("initial message").expect("ok");

    let start_fog = serde_json::json!({
        "command": { "setFog": { "locationId": "town", "fog": { "revealedCells": [] } } }
    });
    ws.send(Message::Text(start_fog.to_string())).await.expect("send setFog");
    let after_start = ws.next().await.expect("state after setFog").expect("ok");
    let Message::Text(after_start_text) = after_start else { panic!("expected text") };
    assert!(after_start_text.contains("\"fog\""), "{after_start_text}");

    // A whole click-drag stroke lands as one batch, not one command per cell.
    let reveal = serde_json::json!({
        "command": { "setFogCells": { "locationId": "town", "cells": ["2,3", "2,4"], "revealed": true } }
    });
    ws.send(Message::Text(reveal.to_string())).await.expect("send setFogCells");
    let after_reveal = ws.next().await.expect("state after setFogCells").expect("ok");
    let Message::Text(after_reveal_text) = after_reveal else { panic!("expected text") };
    assert!(after_reveal_text.contains("2,3") && after_reveal_text.contains("2,4"), "{after_reveal_text}");

    // Painting the same cells revealed again is idempotent — no duplicate entries.
    ws.send(Message::Text(reveal.to_string())).await.expect("send setFogCells again");
    let after_repaint = ws.next().await.expect("state after re-painting").expect("ok");
    let Message::Text(after_repaint_text) = after_repaint else { panic!("expected text") };
    assert_eq!(after_repaint_text.matches("2,3").count(), 1, "{after_repaint_text}");

    let hide = serde_json::json!({
        "command": { "setFogCells": { "locationId": "town", "cells": ["2,3", "2,4"], "revealed": false } }
    });
    ws.send(Message::Text(hide.to_string())).await.expect("send setFogCells hide");
    let after_hide = ws.next().await.expect("state after hiding").expect("ok");
    let Message::Text(after_hide_text) = after_hide else { panic!("expected text") };
    assert!(!after_hide_text.contains("2,3") && !after_hide_text.contains("2,4"), "{after_hide_text}");
}

async fn tempfile_project() -> std::path::PathBuf {
    let src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo/demo.hexen.yml");
    let vault_src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo/_vault");

    let dir = std::env::temp_dir().join(format!("hexend-test-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let dest = dir.join("demo.hexen.yml");
    tokio::fs::copy(&src, &dest).await.unwrap();

    let dest_vault = dir.join("_vault");
    tokio::fs::create_dir_all(&dest_vault).await.unwrap();
    let mut entries = tokio::fs::read_dir(&vault_src).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        tokio::fs::copy(entry.path(), dest_vault.join(entry.file_name())).await.unwrap();
    }

    dest
}

fn urlencoding_lite(s: &str) -> String {
    s.replace('/', "%2F")
}

async fn start_viewer(sessions: hexend::session::Sessions) -> u16 {
    let app = hexend::server::viewer_app(sessions);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    port
}

#[tokio::test]
async fn viewer_surface_is_read_only_and_limited_to_open_projects() {
    let sessions = hexend::session::new_sessions();
    let full_app = hexend::server::app(sessions.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let full = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, full_app).await.unwrap() });
    let viewer = start_viewer(sessions).await;

    let tmp = tempfile_project().await;
    let encoded = urlencoding_lite(tmp.to_str().unwrap());

    // Not open yet: the viewer won't open it on a LAN client's say-so.
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{viewer}/ws?path={encoded}"))
            .await
            .expect("connect");
    assert!(matches!(
        ws.next().await,
        Some(Ok(Message::Close(_))) | None
    ));

    // The GM opens it through the full API...
    let (mut gm, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{full}/ws?path={encoded}"))
            .await
            .expect("connect");
    gm.next().await.expect("initial").expect("ok");

    // ...now a viewer can attach, but its commands are ignored.
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{viewer}/ws?path={encoded}"))
            .await
            .expect("connect");
    let Some(Ok(Message::Text(initial))) = ws.next().await else {
        panic!("expected state")
    };
    assert!(initial.contains("Demo Realm"));
    let command = serde_json::json!({ "command": { "addLocation": { "locationId": "sneaky" } } });
    ws.send(Message::Text(command.to_string())).await.unwrap();
    let nothing = tokio::time::timeout(Duration::from_millis(300), ws.next()).await;
    assert!(
        nothing.is_err(),
        "a viewer's command must not produce a broadcast"
    );

    // The GM's changes still reach it live.
    let command = serde_json::json!({ "command": { "addLocation": { "locationId": "from-gm" } } });
    gm.send(Message::Text(command.to_string())).await.unwrap();
    let Some(Ok(Message::Text(update))) = ws.next().await else {
        panic!("expected broadcast")
    };
    assert!(update.contains("from-gm") && !update.contains("sneaky"));

    // Images: only from the open project's own directory.
    let dir = tmp.parent().unwrap().to_str().unwrap();
    let get = |url: String| async move { reqwest_lite(&url).await };
    assert_eq!(
        get(format!(
            "http://127.0.0.1:{viewer}/image?dir={}&file=demo.hexen.yml",
            urlencoding_lite(dir)
        ))
        .await,
        200
    );
    assert_eq!(
        get(format!(
            "http://127.0.0.1:{viewer}/image?dir=%2Fetc&file=hostname"
        ))
        .await,
        404
    );
    assert_eq!(
        get(format!("http://127.0.0.1:{viewer}/directory")).await,
        404
    );
}

/// Just the status code of a GET, over a bare TCP connection.
async fn reqwest_lite(url: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let rest = url.strip_prefix("http://").unwrap();
    let (host, path) = rest.split_once('/').unwrap();
    let mut stream = tokio::net::TcpStream::connect(host).await.unwrap();
    let request = format!("GET /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    let mut buf = vec![0; 4096];
    let n = stream.read(&mut buf).await.unwrap();
    response.push_str(&String::from_utf8_lossy(&buf[..n]));
    response.split_whitespace().nth(1).unwrap().parse().unwrap()
}

/// Status code of a raw HTTP request with an optional body.
async fn http_status(port: u16, method: &str, path: &str, body: &[u8]) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    String::from_utf8_lossy(&response).split_whitespace().nth(1).unwrap().parse().unwrap()
}

#[tokio::test]
async fn image_reads_and_uploads_are_limited_to_open_projects() {
    let port = start_server().await;
    let tmp = tempfile_project().await;
    let dir = urlencoding_lite(tmp.parent().unwrap().to_str().unwrap());
    let mut png = vec![0u8; 24];
    png[12..16].copy_from_slice(b"IHDR");
    png[16..20].copy_from_slice(&4u32.to_be_bytes());
    png[20..24].copy_from_slice(&3u32.to_be_bytes());

    // Nothing open yet: neither an arbitrary directory nor the project's own.
    assert_eq!(http_status(port, "GET", "/image?dir=%2Fetc&file=hostname", b"").await, 404);
    assert_eq!(http_status(port, "GET", &format!("/image?dir={dir}&file=demo.hexen.yml"), b"").await, 404);
    let upload = format!("/image?dir={dir}&locationId=town&ext=png");
    assert_eq!(http_status(port, "POST", &upload, &png).await, 404);

    // Open it over /ws, as the editor does before touching images.
    let url = format!("ws://127.0.0.1:{port}/ws?path={}", urlencoding_lite(tmp.to_str().unwrap()));
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.expect("connect");
    ws.next().await.expect("initial").expect("ok");

    assert_eq!(http_status(port, "GET", &format!("/image?dir={dir}&file=demo.hexen.yml"), b"").await, 200);
    assert_eq!(http_status(port, "GET", &format!("/image?dir={dir}&file=..%2F..%2Fetc%2Fhostname"), b"").await, 400);
    assert_eq!(http_status(port, "GET", "/image?dir=%2Fetc&file=hostname", b"").await, 404);
    assert_eq!(http_status(port, "POST", &upload, &png).await, 200);
    assert!(tmp.parent().unwrap().join("_assets/town.png").exists());
}
