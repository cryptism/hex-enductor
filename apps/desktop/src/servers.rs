//! The servers the desktop app runs in-process — no sidecar binaries,
//! since hexend is a library:
//!
//! - **local**: hexend's full API on 127.0.0.1, random port, for the
//!   editor window and local presentation windows. Never reachable from
//!   another machine.
//! - **LAN** (opt-in, off by default): hexend's read-only viewer
//!   surface on every interface, for player-facing screens. Shares the
//!   local server's sessions, so it only ever shows projects the editor
//!   has open, live, and can't change anything.
//!
//! Both also serve the presentation app under `/presentation/`.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;
use hexend::session::{new_sessions, Sessions};
use rust_embed::RustEmbed;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};

/// apps/presentation-rs, built with `--public-url /presentation/` into
/// `presentation-dist/` (tauri.conf.json's beforeBuildCommand does it).
#[derive(RustEmbed)]
#[folder = "presentation-dist/"]
struct Presentation;

/// Preferred LAN port — stable so a bookmarked player-screen URL keeps
/// working between sessions; falls back to any free port.
const LAN_PORT: u16 = 4747;

async fn presentation_file(uri: Uri) -> Response {
    // strip_prefix, not trim_start_matches: the app's own files are
    // named presentation-rs-*, and repeated trimming would eat that too.
    let path = uri
        .path()
        .strip_prefix("/presentation")
        .unwrap_or("")
        .trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Presentation::get(path).or_else(|| Presentation::get("index.html")) {
        Some(file) => (
            [(header::CONTENT_TYPE, file.metadata.mimetype().to_string())],
            file.data,
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "The presentation app wasn't built into this binary",
        )
            .into_response(),
    }
}

fn with_presentation(router: Router) -> Router {
    router
        .route("/presentation", axum::routing::get(presentation_file))
        .route("/presentation/", axum::routing::get(presentation_file))
        .route("/presentation/*path", axum::routing::get(presentation_file))
}

async fn serve(listener: TcpListener, app: Router, shutdown: Option<oneshot::Receiver<()>>) {
    let result = match shutdown {
        Some(rx) => {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
        }
        None => axum::serve(listener, app).await,
    };
    if let Err(err) = result {
        eprintln!("server error: {err}");
    }
}

/// This machine's address on its LAN, as other devices would reach it:
/// the source address the OS would pick for an outbound route. Nothing
/// is sent — connecting a UDP socket only chooses a route.
fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    Some(socket.local_addr().ok()?.ip()).filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// The full API, for this machine only.
    pub server_url: String,
    /// The viewer surface other devices use, while LAN sharing is on.
    pub lan_url: Option<String>,
    pub version: &'static str,
}

struct Lan {
    port: u16,
    stop: oneshot::Sender<()>,
}

pub struct Servers {
    sessions: Sessions,
    local: SocketAddr,
    lan: Mutex<Option<Lan>>,
}

impl Servers {
    pub async fn start() -> io::Result<Servers> {
        let sessions = new_sessions();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let local = listener.local_addr()?;
        let app = with_presentation(hexend::server::app(sessions.clone()));
        tokio::spawn(serve(listener, app, None));
        Ok(Servers {
            sessions,
            local,
            lan: Mutex::new(None),
        })
    }

    pub fn server_url(&self) -> String {
        format!("http://{}", self.local)
    }

    pub async fn info(&self) -> ServerInfo {
        let lan_port = self.lan.lock().await.as_ref().map(|lan| lan.port);
        ServerInfo {
            server_url: self.server_url(),
            lan_url: lan_port.map(|port| {
                let host = lan_ip().map_or_else(|| "localhost".to_string(), |ip| ip.to_string());
                format!("http://{host}:{port}")
            }),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Starts or stops the LAN viewer server; idempotent.
    pub async fn set_lan(&self, enabled: bool) -> io::Result<ServerInfo> {
        {
            let mut lan = self.lan.lock().await;
            match (enabled, lan.take()) {
                (true, Some(running)) => *lan = Some(running),
                (false, Some(running)) => {
                    let _ = running.stop.send(());
                }
                (false, None) => {}
                (true, None) => {
                    let listener = match TcpListener::bind((Ipv4Addr::UNSPECIFIED, LAN_PORT)).await
                    {
                        Ok(listener) => listener,
                        Err(_) => TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).await?,
                    };
                    let port = listener.local_addr()?.port();
                    let (stop, stopped) = oneshot::channel();
                    let app = with_presentation(hexend::server::viewer_app(self.sessions.clone()));
                    tokio::spawn(serve(listener, app, Some(stopped)));
                    *lan = Some(Lan { port, stop });
                }
            }
        }
        Ok(self.info().await)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    async fn get(addr: &str, path: &str) -> Option<String> {
        let mut stream = tokio::net::TcpStream::connect(addr).await.ok()?;
        let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.ok()?;
        Some(String::from_utf8_lossy(&response).into_owned())
    }

    #[tokio::test]
    async fn local_server_is_loopback_only_with_the_full_api_and_presentation() {
        let servers = Servers::start().await.unwrap();
        assert!(servers.local.ip().is_loopback());
        let addr = servers.local.to_string();
        assert!(get(&addr, "/").await.unwrap().contains("hexend is awake"));
        assert!(get(&addr, "/presentation/")
            .await
            .unwrap()
            .starts_with("HTTP/1.1 200"));
        assert_eq!(servers.info().await.lan_url, None);

        // Every file the page references is served as itself, not as the
        // index.html fallback.
        let index =
            String::from_utf8(Presentation::get("index.html").unwrap().data.into_owned()).unwrap();
        let assets: Vec<&str> = index
            .split('"')
            .filter(|s| s.starts_with("/presentation/") && s.len() > "/presentation/".len())
            .collect();
        assert!(!assets.is_empty());
        for asset in assets {
            let response = get(&addr, asset).await.unwrap();
            assert!(
                !response.contains("content-type: text/html"),
                "{asset} fell back to index.html"
            );
        }
    }

    #[tokio::test]
    async fn lan_sharing_serves_only_the_viewer_surface_and_stops() {
        let servers = Servers::start().await.unwrap();
        let info = servers.set_lan(true).await.unwrap();
        let port = info
            .lan_url
            .as_deref()
            .and_then(|u| u.rsplit(':').next())
            .unwrap()
            .to_string();
        let lan = format!("127.0.0.1:{port}");

        // Idempotent: a second enable keeps the same server.
        assert_eq!(servers.set_lan(true).await.unwrap(), info);

        assert!(get(&lan, "/presentation/")
            .await
            .unwrap()
            .starts_with("HTTP/1.1 200"));
        assert!(get(&lan, "/directory")
            .await
            .unwrap()
            .starts_with("HTTP/1.1 404"));
        assert!(get(&lan, "/").await.unwrap().starts_with("HTTP/1.1 404"));

        let info = servers.set_lan(false).await.unwrap();
        assert_eq!(info.lan_url, None);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(
            get(&lan, "/presentation/").await.is_none(),
            "LAN server still up after disabling"
        );
    }
}
