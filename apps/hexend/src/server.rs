//! HTTP + WS surface.
//!
//! The one way a client learns or changes a project's live state:
//! connect to /ws, get the current state immediately, then send
//! commands and receive a fresh "state" broadcast — including every
//! other client watching the same path — after each one lands.
//! hexend is authoritative: it applies and persists a command before
//! anyone (including the sender) sees its effect.

use std::path::Path;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;

use crate::image_size::read_image_size;
use crate::pathutil::{resolve, resolve_cwd};
use crate::pb::hexen::v1::{client_message, ClientMessage, ServerMessage};
use crate::router::{create_project, list_directory};
use crate::session::{
    add_socket, apply_and_broadcast, get_or_create_session, get_session, open_project_dirs, redo, remove_socket,
    session_state, undo, Sessions,
};

#[derive(Clone)]
pub struct AppState {
    pub sessions: Sessions,
}

pub fn app(sessions: Sessions) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/image", get(get_image).post(post_image))
        .route("/project", post(create_project))
        .route("/directory", get(list_directory))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(AppState { sessions })
}

/// The read-only surface for other devices (a player-facing screen on
/// the LAN): no directory listing, no project creation, no uploads, and
/// a /ws that only attaches to projects already open through [`app`]
/// and ignores anything a client sends. Images, as on [`app`], only
/// from open projects' directories. Shares `sessions` with the full app, so
/// viewers see every change live. Callers add their own static files
/// (e.g. the presentation app) on top.
pub fn viewer_app(sessions: Sessions) -> Router {
    Router::new()
        .route("/image", get(get_image))
        .route("/ws", get(viewer_ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(AppState { sessions })
}

async fn root() -> &'static str {
    "hexend is awake, and watching your maps."
}

#[derive(Deserialize)]
struct WsQuery {
    path: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Access {
    /// Opens the project if needed, applies commands.
    Full,
    /// Only attaches to an open project; incoming messages are dropped.
    ReadOnly,
}

async fn ws_handler(Query(query): Query<WsQuery>, State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, query.path, state, Access::Full))
}

async fn viewer_ws_handler(
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, query.path, state, Access::ReadOnly))
}

async fn handle_socket(mut socket: WebSocket, path: Option<String>, state: AppState, access: Access) {
    let Some(path) = path else {
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: 1008,
                reason: "Missing path query param".into(),
            })))
            .await;
        return;
    };

    let session = match access {
        Access::Full => get_or_create_session(&state.sessions, &path).await.map_err(|e| e.to_string()),
        Access::ReadOnly => get_session(&state.sessions, &path)
            .await
            .ok_or_else(|| "That project isn't open on this server".to_string()),
    };
    let session = match session {
        Ok(session) => session,
        Err(err) => {
            let _ = socket
                .send(Message::Close(Some(CloseFrame {
                    code: 1011,
                    reason: err.into(),
                })))
                .await;
            return;
        }
    };

    let initial = {
        let guard = session.lock().await;
        ServerMessage {
            state: Some(session_state(&guard)),
        }
    };
    let text = serde_json::to_string(&initial).expect("ServerMessage always serializes");
    if socket.send(Message::Text(text)).await.is_err() {
        return;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let socket_id = {
        let mut guard = session.lock().await;
        add_socket(&mut guard, tx)
    };

    let (mut sink, mut stream) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    let recv_session = session.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let Message::Text(text) = msg else { continue };
            if access == Access::ReadOnly {
                continue;
            }
            let Ok(client_message) = serde_json::from_str::<ClientMessage>(&text) else {
                continue;
            };

            let mut guard = recv_session.lock().await;
            match client_message.kind {
                Some(client_message::Kind::Command(command)) => {
                    if let Err(err) = apply_and_broadcast(&mut guard, command) {
                        eprintln!("command failed: {err}");
                    }
                }
                Some(client_message::Kind::Undo(_)) => undo(&mut guard),
                Some(client_message::Kind::Redo(_)) => redo(&mut guard),
                None => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let mut guard = session.lock().await;
    remove_socket(&mut guard, socket_id);
}

const UPLOAD_EXTENSIONS: &[(&str, &str)] = &[("png", "png"), ("jpg", "jpg"), ("jpeg", "jpg")];

fn upload_ext(ext: &str) -> Option<&'static str> {
    UPLOAD_EXTENSIONS
        .iter()
        .find(|(from, _)| *from == ext)
        .map(|(_, to)| *to)
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn sanitize_filename(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' })
        .collect()
}

// No auth, so image reads and uploads are limited to the directories
// of projects that are open in a session: a caller can't point `dir`
// at an arbitrary folder, and `file` can't walk back out of it.
#[derive(Deserialize)]
struct ImageGetQuery {
    dir: Option<String>,
    file: Option<String>,
}

/// `dir` resolved, if it's the directory of an open project.
async fn open_project_dir(sessions: &Sessions, dir: &str) -> Option<std::path::PathBuf> {
    let dir = resolve_cwd(Path::new(dir));
    open_project_dirs(sessions).await.contains(&dir).then_some(dir)
}

async fn get_image(State(state): State<AppState>, Query(q): Query<ImageGetQuery>) -> Response {
    let (Some(dir), Some(file)) = (q.dir, q.file) else {
        return (StatusCode::BAD_REQUEST, "Missing dir or file query param").into_response();
    };
    let Some(base) = open_project_dir(&state.sessions, &dir).await else {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    };
    let target = resolve(&base, Path::new(&file));
    if !target.starts_with(&base) {
        return (StatusCode::BAD_REQUEST, "file escapes the project directory").into_response();
    }

    match tokio::fs::read(&target).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, mime_for(&target))], bytes).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

// A location's base map image: the client posts raw bytes for a given
// project dir + location id, we settle on a filename ourselves (so
// re-uploading for the same location always replaces the same file
// rather than accumulating orphans) and hand back what a saveImage
// command needs to write into the .hexen.yml.
#[derive(Deserialize)]
struct ImagePostQuery {
    dir: String,
    #[serde(rename = "locationId")]
    location_id: String,
    ext: String,
}

async fn post_image(
    State(state): State<AppState>,
    Query(q): Query<ImagePostQuery>,
    body: axum::body::Bytes,
) -> Response {
    let Some(ext) = upload_ext(&q.ext.to_lowercase()) else {
        return (
            StatusCode::BAD_REQUEST,
            format!("Unsupported image extension \"{}\" — use png or jpg/jpeg", q.ext),
        )
            .into_response();
    };

    let Some(project_dir) = open_project_dir(&state.sessions, &q.dir).await else {
        return (StatusCode::NOT_FOUND, "No open project in that directory").into_response();
    };
    let safe_name = format!("{}.{ext}", sanitize_filename(&q.location_id));
    let assets_dir = project_dir.join("_assets");
    let target = assets_dir.join(&safe_name);
    if !target.starts_with(&assets_dir) {
        return (StatusCode::BAD_REQUEST, "file escapes the project directory").into_response();
    }

    let Some(size) = read_image_size(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            "Couldn't read image dimensions — is this really a PNG or JPEG?",
        )
            .into_response();
    };

    if let Err(err) = tokio::fs::create_dir_all(&assets_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
    }
    if let Err(err) = tokio::fs::write(&target, &body).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
    }

    Json(serde_json::json!({
        "file": format!("_assets/{safe_name}"),
        "width": size.width,
        "height": size.height,
    }))
    .into_response()
}
