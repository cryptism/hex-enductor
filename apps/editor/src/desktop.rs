//! Running inside the desktop app (apps/desktop)? The shell injects
//! `window.__HEXEN_DESKTOP__` before this app loads; its presence is the
//! whole feature flag — the same build works in a plain browser, where
//! it's absent and none of this shows.

use js_sys::{Function, Promise, Reflect, JSON};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::http::describe;

/// What the desktop app's in-process servers look like right now —
/// apps/desktop's `servers::ServerInfo`.
#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// hexend's full API, reachable from this machine only.
    pub server_url: String,
    /// The read-only viewer server, while LAN sharing is on.
    pub lan_url: Option<String>,
    pub version: String,
}

fn global(name: &str) -> Option<JsValue> {
    Reflect::get(&web_sys::window()?.into(), &name.into())
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
}

fn from_js<T: DeserializeOwned>(value: &JsValue) -> Result<T, String> {
    if value.is_undefined() || value.is_null() {
        return serde_json::from_str("null").map_err(|e| e.to_string());
    }
    let json: String = JSON::stringify(value).map_err(describe)?.into();
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// The server details the desktop app started with, if this is it.
pub fn desktop_info() -> Option<ServerInfo> {
    from_js(&global("__HEXEN_DESKTOP__")?).ok()
}

pub fn is_desktop() -> bool {
    desktop_info().is_some()
}

/// Calls one of apps/desktop's Tauri commands.
pub async fn invoke<T: DeserializeOwned>(
    command: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    let invoke: Function = global("__TAURI__")
        .and_then(|t| Reflect::get(&t, &"core".into()).ok())
        .and_then(|core| Reflect::get(&core, &"invoke".into()).ok())
        .and_then(|f| f.dyn_into().ok())
        .ok_or("Not running in the desktop app")?;
    let args = JSON::parse(&args.to_string()).map_err(describe)?;
    let promise: Promise = invoke
        .call2(&JsValue::NULL, &command.into(), &args)
        .map_err(describe)?
        .dyn_into()
        .map_err(describe)?;
    from_js(&JsFuture::from(promise).await.map_err(describe)?)
}

pub async fn server_info() -> Result<ServerInfo, String> {
    invoke("server_info", serde_json::json!({})).await
}

pub async fn set_lan_sharing(enabled: bool) -> Result<ServerInfo, String> {
    invoke("set_lan_sharing", serde_json::json!({ "enabled": enabled })).await
}

pub async fn open_presentation_window(path: &str) -> Result<(), String> {
    invoke(
        "open_presentation_window",
        serde_json::json!({ "path": path }),
    )
    .await
}

/// The native open-file dialog; `None` if cancelled.
pub async fn pick_project_file() -> Result<Option<String>, String> {
    invoke("pick_project_file", serde_json::json!({})).await
}

/// The presentation page for `path`, as served by `server` (the local or
/// the LAN server — both serve it under /presentation/).
pub fn presentation_url(server: &str, path: &str) -> String {
    format!(
        "{server}/presentation/?server={}&path={}",
        crate::http::encode(server),
        crate::http::encode(path)
    )
}
