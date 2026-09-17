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
