//! The hexend-backed storage: its live /ws session, plus plain HTTP for
//! image files.

use std::rc::Rc;

use hexen_proto::hexen::v1::{command, Command, ImageRef, SaveImageCommand};
use hexen_web::live_session::{self, LiveSession};

use super::{dirname, image_extension, OnError, OnUpdate, UNSUPPORTED_IMAGE};
use crate::http::{encode, fetch_text, server_url};

pub struct ServerStorage {
    path: String,
    session: Option<LiveSession>,
}

impl ServerStorage {
    pub fn connect(path: &str, on_update: OnUpdate, on_error: OnError) -> Rc<Self> {
        let session =
            match live_session::connect(&server_url(), path, move |data| on_update(data), {
                let on_error = on_error.clone();
                move |err| on_error(err)
            }) {
                Ok(session) => Some(session),
                Err(err) => {
                    on_error(err);
                    None
                }
            };
        Rc::new(ServerStorage {
            path: path.to_owned(),
            session,
        })
    }

    pub fn execute(&self, command: Command) {
        if let Some(s) = &self.session {
            s.execute(command);
        }
    }

    pub fn undo(&self) {
        if let Some(s) = &self.session {
            s.undo();
        }
    }

    pub fn redo(&self) {
        if let Some(s) = &self.session {
            s.redo();
        }
    }

    pub fn image_url(&self, file: &str) -> String {
        format!(
            "{}/image?dir={}&file={}",
            server_url(),
            encode(dirname(&self.path)),
            encode(file)
        )
    }

    /// Posts the bytes to hexend, which stores them next to the project
    /// and measures them; the saveImage that points the Location at them
    /// goes over the session like any other command.
    pub async fn upload_image(&self, location_id: &str, file: web_sys::File) -> Result<(), String> {
        let ext = image_extension(&file).ok_or(UNSUPPORTED_IMAGE)?;
        let url = format!(
            "{}/image?dir={}&locationId={}&ext={ext}",
            server_url(),
            encode(dirname(&self.path)),
            encode(location_id)
        );
        let body = fetch_text("POST", &url, Some(&file.into()), false).await?;
        let image: ImageRef = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        self.execute(Command {
            kind: Some(command::Kind::SaveImage(SaveImageCommand {
                location_id: location_id.to_owned(),
                image: Some(image),
            })),
        });
        Ok(())
    }
}
