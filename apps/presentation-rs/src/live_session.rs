//! A read-only connection to one project's session on hexend's /ws —
//! the Rust counterpart of packages/live-session's `connectLiveSession`,
//! minus execute/undo/redo, which nothing here sends yet (the editor
//! port will add them). hexend is authoritative: every state, the first
//! included, arrives as a `ServerMessage`, parsed with the same
//! pbjson-generated types hexend serializes it with — so unlike the TS
//! client there's no wire-format conversion step to keep in sync.

use hexen_proto::hexen::v1::{OpenedProjectData, ServerMessage};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

/// Keeps the socket and its JS callbacks alive; dropping it closes the
/// connection.
pub struct LiveSession {
    socket: WebSocket,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_failure: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.socket.set_onmessage(None);
        self.socket.set_onerror(None);
        self.socket.set_onclose(None);
        let _ = self.socket.close();
    }
}

fn ws_url(server_url: &str, path: &str) -> String {
    let base = server_url
        .strip_prefix("http")
        .map_or_else(|| server_url.to_owned(), |rest| format!("ws{rest}"));
    format!("{base}/ws?path={}", js_sys::encode_uri_component(path))
}

/// Calls `on_state` with every state hexend sends, starting with the
/// handshake. `on_error` fires at most once, and only if the connection
/// fails or closes before that first state — same contract as the TS
/// client's rejected promise; after that, a dropped connection just
/// stops the updates.
pub fn connect(
    server_url: &str,
    path: &str,
    on_state: impl Fn(OpenedProjectData) + 'static,
    on_error: impl Fn(String) + 'static,
) -> Result<LiveSession, String> {
    let socket = WebSocket::new(&ws_url(server_url, path)).map_err(|e| format!("{e:?}"))?;
    let opened = std::rc::Rc::new(std::cell::Cell::new(false));

    let on_message = {
        let opened = opened.clone();
        Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
            let Some(text) = evt.data().as_string() else {
                return;
            };
            let Ok(ServerMessage { state: Some(state) }) = serde_json::from_str(&text) else {
                return;
            };
            opened.set(true);
            on_state(state);
        })
    };

    let on_failure = {
        let server_url = server_url.to_owned();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |evt: web_sys::Event| {
            if opened.replace(true) {
                return;
            }
            on_error(if evt.type_() == "error" {
                format!("Couldn't connect to {server_url}")
            } else {
                format!("Connection to {server_url} closed before it opened")
            });
        })
    };

    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    socket.set_onerror(Some(on_failure.as_ref().unchecked_ref()));
    socket.set_onclose(Some(on_failure.as_ref().unchecked_ref()));

    Ok(LiveSession {
        socket,
        _on_message: on_message,
        _on_failure: on_failure,
    })
}
