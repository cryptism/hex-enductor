//! The desktop shell. Everything interesting is in [`servers`]; this is
//! the Tauri wiring around it:
//!
//! - one editor window, loading apps/editor-rs's build, with the local
//!   server's details injected as `window.__HEXEN_DESKTOP__` before any
//!   of its code runs. The editor treats that global as its "running in
//!   the desktop app" flag: it points itself at this server and shows
//!   its Server panel. The same editor build works in a plain browser,
//!   where the global is simply absent.
//! - four commands the editor's Server panel and picker call.

pub mod servers;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use servers::{ServerInfo, Servers};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

type ServersState<'a> = State<'a, Arc<Servers>>;

#[tauri::command]
async fn server_info(servers: ServersState<'_>) -> Result<ServerInfo, String> {
    Ok(servers.info().await)
}

#[tauri::command]
async fn set_lan_sharing(servers: ServersState<'_>, enabled: bool) -> Result<ServerInfo, String> {
    servers.set_lan(enabled).await.map_err(|e| e.to_string())
}

/// A second native window showing the presentation view of `path`,
/// served by the local server — a plain web page with no IPC access.
#[tauri::command]
async fn open_presentation_window(
    app: AppHandle,
    servers: ServersState<'_>,
    path: String,
) -> Result<(), String> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let server = servers.server_url();
    let mut url: tauri::Url = format!("{server}/presentation/")
        .parse()
        .map_err(|e| format!("{e}"))?;
    url.query_pairs_mut()
        .append_pair("server", &server)
        .append_pair("path", &path);
    let label = format!("presentation-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title("Hex Enductor — presentation")
        .inner_size(1280.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// The native "open file" dialog, filtered to project files — the
/// desktop stand-in for typing a path or the browser folder picker
/// (which the system webviews don't all support).
#[tauri::command]
async fn pick_project_file(app: AppHandle) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("Hex Enductor project", &["yml", "yaml"])
        .blocking_pick_file();
    Ok(picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let servers = tauri::async_runtime::block_on(Servers::start())?;
            let info = tauri::async_runtime::block_on(servers.info());
            app.manage(Arc::new(servers));

            let init = format!(
                "window.__HEXEN_DESKTOP__ = {};",
                serde_json::to_string(&info)?
            );
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hex Enductor")
                .inner_size(1280.0, 800.0)
                .initialization_script(&init)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            server_info,
            set_lan_sharing,
            open_presentation_window,
            pick_project_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Hex Enductor");
}
