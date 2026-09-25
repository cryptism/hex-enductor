//! Everywhere in the editor that touches project data goes through a
//! [`Storage`] instead of a transport directly — port of apps/editor's
//! ProjectStorage. `execute` never returns the resulting state: every
//! state change, this backend's own or (server) another connected
//! client's, arrives through the `on_update` callback given at connect.

mod local_fs;
mod server;

use std::rc::Rc;

use hexen_proto::hexen::v1::{Command, OpenedProjectData, Ping};
use web_sys::FileSystemDirectoryHandle;

pub use local_fs::{request_readwrite, show_directory_picker, supports_local_fs, LocalFsStorage};
pub use server::ServerStorage;

pub type OnUpdate = Rc<dyn Fn(OpenedProjectData)>;
pub type OnError = Rc<dyn Fn(String)>;
/// A ping from anyone on the session, this client included.
pub type OnPing = Rc<dyn Fn(Ping)>;

/// What to open — kept by the app between opens, and what a project's
/// editor is keyed on.
#[derive(Clone)]
pub enum Source {
    /// A .hexen.yml on a running hexend, by absolute path.
    Server(String),
    /// A project folder picked in this browser (File System Access API).
    Folder(FileSystemDirectoryHandle),
}

#[derive(Clone)]
pub enum Storage {
    Server(Rc<ServerStorage>),
    Local(Rc<LocalFsStorage>),
}

/// A displayable URL for a project image. A folder-backed image is a
/// blob: URL, revoked when this is dropped.
pub struct ImageUrl {
    pub url: String,
    revoke: bool,
}

impl Drop for ImageUrl {
    fn drop(&mut self) {
        if self.revoke {
            let _ = web_sys::Url::revoke_object_url(&self.url);
        }
    }
}

impl Storage {
    pub fn connect(
        source: &Source,
        on_update: OnUpdate,
        on_ping: OnPing,
        on_error: OnError,
    ) -> Storage {
        match source {
            Source::Server(path) => {
                Storage::Server(ServerStorage::connect(path, on_update, on_ping, on_error))
            }
            Source::Folder(dir) => {
                Storage::Local(LocalFsStorage::connect(dir.clone(), on_update, on_error))
            }
        }
    }

    pub fn execute(&self, command: Command) {
        match self {
            Storage::Server(s) => s.execute(command),
            Storage::Local(s) => s.execute(command),
        }
    }

    /// Ephemeral, never touches the project. A no-op for a browser
    /// folder, which has no session to broadcast over.
    pub fn ping(&self, location_id: &str, x: f64, y: f64) {
        if let Storage::Server(s) = self {
            s.ping(location_id, x, y);
        }
    }

    /// Follow mode: this map's view. A no-op for a browser folder.
    pub fn follow_view(&self, location_id: &str, x: f64, y: f64, zoom: f64) {
        if let Storage::Server(s) = self {
            s.follow_view(location_id, x, y, zoom);
        }
    }

    pub fn undo(&self) {
        match self {
            Storage::Server(s) => s.undo(),
            Storage::Local(s) => s.undo(),
        }
    }

    pub fn redo(&self) {
        match self {
            Storage::Server(s) => s.redo(),
            Storage::Local(s) => s.redo(),
        }
    }

    pub async fn image_url(&self, file: &str) -> Result<ImageUrl, String> {
        match self {
            Storage::Server(s) => Ok(ImageUrl {
                url: s.image_url(file),
                revoke: false,
            }),
            Storage::Local(s) => Ok(ImageUrl {
                url: s.image_url(file).await?,
                revoke: true,
            }),
        }
    }

    pub async fn upload_image(&self, location_id: &str, file: web_sys::File) -> Result<(), String> {
        match self {
            Storage::Server(s) => s.upload_image(location_id, file).await,
            Storage::Local(s) => s.upload_image(location_id, file).await,
        }
    }
}

/// "png"/"jpeg" from a picked file's MIME type or, failing that, its
/// name; `None` for anything else.
pub(crate) fn image_extension(file: &web_sys::File) -> Option<&'static str> {
    match file.type_().as_str() {
        "image/png" => return Some("png"),
        "image/jpeg" => return Some("jpeg"),
        _ => {}
    }
    let name = file.name().to_ascii_lowercase();
    if name.ends_with(".png") {
        Some("png")
    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        Some("jpeg")
    } else {
        None
    }
}

pub(crate) const UNSUPPORTED_IMAGE: &str = "Only PNG or JPEG images are supported.";

pub(crate) fn dirname(path: &str) -> &str {
    path.rfind('/').map_or(".", |i| &path[..i])
}
