//! Live per-project state: the WS session's command log, undo/redo
//! cursor, and broadcast to every connected client.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::ws::Message;
use tokio::sync::{mpsc, Mutex};

use crate::commands::apply_command;
use crate::mutations::MutationError;
use crate::pb::hexen::v1::{
    location_content, server_message, Command, HexenProject, OpenedProjectData, Ping, ResolvedContent, ServerMessage,
};
use crate::project_io::{open_project, save_project, OpenError};
use crate::resolver::resolve_inline_content;

static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(0);

pub struct LogEntry {
    pub command: Command,
    pub snapshot: HexenProject,
}

/// Sessions live for the server process's lifetime, keyed by absolute
/// project path — no eviction. Fine at single-user/dev-server scale.
pub struct ProjectSession {
    pub path: PathBuf,
    pub project: HexenProject,
    pub resolved_content: HashMap<String, ResolvedContent>,
    pub resolve_errors: HashMap<String, String>,
    pub warnings: Vec<String>,
    pub sockets: HashMap<u64, mpsc::UnboundedSender<Message>>,
    /// Command history with a cursor into it — undo/redo just move the cursor.
    pub log: Vec<LogEntry>,
    pub cursor: i64,
    /// The project as it stood before this session's first command — where undo bottoms out.
    pub base_snapshot: HexenProject,
}

pub type SessionHandle = Arc<Mutex<ProjectSession>>;
pub type Sessions = Arc<Mutex<HashMap<String, SessionHandle>>>;

pub fn new_sessions() -> Sessions {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn get_or_create_session(sessions: &Sessions, path: &str) -> Result<SessionHandle, OpenError> {
    {
        let map = sessions.lock().await;
        if let Some(existing) = map.get(path) {
            return Ok(existing.clone());
        }
    }

    let opened = open_project(Path::new(path)).await?;
    let base_snapshot = opened.project.clone();
    let session = Arc::new(Mutex::new(ProjectSession {
        path: PathBuf::from(path),
        project: opened.project,
        resolved_content: opened.resolved_content,
        resolve_errors: opened.resolve_errors,
        warnings: opened.warnings,
        sockets: HashMap::new(),
        log: Vec::new(),
        cursor: -1,
        base_snapshot,
    }));

    let mut map = sessions.lock().await;
    // Another connection may have raced us to create the same session.
    if let Some(existing) = map.get(path) {
        return Ok(existing.clone());
    }
    map.insert(path.to_string(), session.clone());
    Ok(session)
}

pub fn add_socket(session: &mut ProjectSession, tx: mpsc::UnboundedSender<Message>) -> u64 {
    let id = NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
    session.sockets.insert(id, tx);
    id
}

pub fn remove_socket(session: &mut ProjectSession, id: u64) {
    session.sockets.remove(&id);
}

pub fn session_state(session: &ProjectSession) -> OpenedProjectData {
    OpenedProjectData {
        project: Some(session.project.clone()),
        warnings: session.warnings.clone(),
        resolved_content: session.resolved_content.clone(),
        resolve_errors: session.resolve_errors.clone(),
    }
}

// Inline content lives directly on the Location, so it never needs a
// vault file re-read — only the five commands' locations can ever
// change, and only inline ones. Obsidian-resolved entries are stable
// across every command this session handles, so they're never touched.
fn refresh_inline_content(session: &mut ProjectSession) {
    let locations = session.project.locations.clone();
    for location in locations {
        let Some(content) = location.content else { continue };
        if let Some(location_content::Kind::Inline(inline)) = content.kind {
            session
                .resolved_content
                .insert(location.id.clone(), resolve_inline_content(&inline));
            session.resolve_errors.remove(&location.id);
        }
    }
}

fn persist(session: &ProjectSession) {
    let path = session.path.clone();
    let project = session.project.clone();
    tokio::spawn(async move {
        if let Err(err) = save_project(&path, &project).await {
            eprintln!("Failed to persist \"{}\": {err}", path.display());
        }
    });
}

fn broadcast(session: &ProjectSession) {
    let message = ServerMessage {
        kind: Some(server_message::Kind::State(session_state(session))),
    };
    let text = serde_json::to_string(&message).expect("ServerMessage always serializes");
    for tx in session.sockets.values() {
        let _ = tx.send(Message::Text(text.clone()));
    }
}

/// Ephemeral — re-broadcast verbatim to every socket watching this
/// session, sender included. Never touches `session.project`, never
/// persisted, no undo/redo entry.
pub fn broadcast_ping(session: &ProjectSession, ping: Ping) {
    let message = ServerMessage {
        kind: Some(server_message::Kind::Ping(ping)),
    };
    let text = serde_json::to_string(&message).expect("ServerMessage always serializes");
    for tx in session.sockets.values() {
        let _ = tx.send(Message::Text(text.clone()));
    }
}

fn settle(session: &mut ProjectSession, project: HexenProject) {
    session.project = project;
    refresh_inline_content(session);
    broadcast(session);
    persist(session);
}

pub fn apply_and_broadcast(session: &mut ProjectSession, command: Command) -> Result<(), MutationError> {
    // A command issued after an undo discards whatever redo branch existed.
    if session.cursor < session.log.len() as i64 - 1 {
        session.log.truncate((session.cursor + 1) as usize);
    }

    let mut next = session.project.clone();
    apply_command(&mut next, command.clone())?;
    session.log.push(LogEntry {
        command,
        snapshot: next.clone(),
    });
    session.cursor += 1;
    settle(session, next);
    Ok(())
}

pub fn undo(session: &mut ProjectSession) {
    if session.cursor < 0 {
        return;
    }
    session.cursor -= 1;
    let snapshot = if session.cursor == -1 {
        session.base_snapshot.clone()
    } else {
        session.log[session.cursor as usize].snapshot.clone()
    };
    settle(session, snapshot);
}

pub fn redo(session: &mut ProjectSession) {
    if session.cursor >= session.log.len() as i64 - 1 {
        return;
    }
    session.cursor += 1;
    let snapshot = session.log[session.cursor as usize].snapshot.clone();
    settle(session, snapshot);
}
