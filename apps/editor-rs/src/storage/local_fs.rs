//! A project opened as a folder in the browser, no server involved —
//! the File System Access API backend. Runs crates/project-ops in wasm,
//! so it reads and writes exactly the YAML dialect hexend does.
//!
//! There's no server to be authoritative, so this keeps the state
//! itself: a command updates the in-memory History and notifies at
//! once, then the file is rewritten in the background. Writes are
//! coalesced — while one is in flight, further commands just replace
//! the pending snapshot — so a fast fog stroke can't interleave two
//! writes and leave an older state on disk.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use hexen_proto::hexen::v1::{
    command, location_content, project_content, Command, HexenProject, ImageRef, OpenedProjectData,
    ResolvedContent, SaveImageCommand,
};
use js_sys::Reflect;
use project_ops::content::{
    no_vault_error, parse_obsidian_note, resolve_inline_content, vault_relative_path,
};
use project_ops::document::{parse_hexen_project, serialize_hexen_project, validate_references};
use project_ops::history::History;
use project_ops::image_size::read_image_size;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions, FileSystemHandleKind, FileSystemWritableFileStream,
};

use super::{image_extension, OnError, OnUpdate, UNSUPPORTED_IMAGE};
use crate::http::describe;

// The two File System Access calls web-sys still gates behind
// `web_sys_unstable_apis`; bound by hand rather than setting that cfg
// for the whole build.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_name = showDirectoryPicker)]
    async fn show_directory_picker_js() -> Result<JsValue, JsValue>;

    #[wasm_bindgen(extends = web_sys::FileSystemHandle)]
    type PermissionedHandle;

    #[wasm_bindgen(method, catch, js_name = requestPermission)]
    async fn request_permission_js(
        this: &PermissionedHandle,
        descriptor: &JsValue,
    ) -> Result<JsValue, JsValue>;
}

/// File System Access API support is Chromium-only today.
pub fn supports_local_fs() -> bool {
    web_sys::window()
        .is_some_and(|w| Reflect::has(&w, &"showDirectoryPicker".into()).unwrap_or(false))
}

/// `Ok(None)` if the person closed the picker without choosing.
pub async fn show_directory_picker() -> Result<Option<FileSystemDirectoryHandle>, String> {
    match show_directory_picker_js().await {
        Ok(handle) => Ok(Some(handle.unchecked_into())),
        Err(err)
            if err
                .dyn_ref::<web_sys::DomException>()
                .is_some_and(|e| e.name() == "AbortError") =>
        {
            Ok(None)
        }
        Err(err) => Err(describe(err)),
    }
}

/// Re-asks for read/write access to a remembered folder.
pub async fn request_readwrite(dir: &FileSystemDirectoryHandle) -> Result<bool, String> {
    let descriptor = js_sys::Object::new();
    Reflect::set(&descriptor, &"mode".into(), &"readwrite".into()).map_err(describe)?;
    let handle: &PermissionedHandle = dir.unchecked_ref();
    let state = handle
        .request_permission_js(&descriptor)
        .await
        .map_err(describe)?;
    Ok(state.as_string().as_deref() == Some("granted"))
}

async fn find_hexen_file(dir: &FileSystemDirectoryHandle) -> Result<FileSystemFileHandle, String> {
    let entries = dir.values();
    loop {
        let step = JsFuture::from(entries.next().map_err(describe)?)
            .await
            .map_err(describe)?;
        if Reflect::get(&step, &"done".into())
            .ok()
            .and_then(|d| d.as_bool())
            .unwrap_or(true)
        {
            break;
        }
        let entry: web_sys::FileSystemHandle = Reflect::get(&step, &"value".into())
            .map_err(describe)?
            .unchecked_into();
        if entry.kind() == FileSystemHandleKind::File && entry.name().ends_with(".hexen.yml") {
            return Ok(entry.unchecked_into());
        }
    }
    Err("No .hexen.yml file found in this folder.".into())
}

async fn file_at(
    root: &FileSystemDirectoryHandle,
    path: &str,
    create: bool,
) -> Result<FileSystemFileHandle, String> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let (name, dirs) = parts
        .split_last()
        .ok_or_else(|| format!("Empty path \"{path}\""))?;
    let mut dir = root.clone();
    for part in dirs {
        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(create);
        dir = JsFuture::from(dir.get_directory_handle_with_options(part, &options))
            .await
            .map_err(describe)?
            .unchecked_into();
    }
    let options = FileSystemGetFileOptions::new();
    options.set_create(create);
    Ok(
        JsFuture::from(dir.get_file_handle_with_options(name, &options))
            .await
            .map_err(describe)?
            .unchecked_into(),
    )
}

async fn read_file(handle: &FileSystemFileHandle) -> Result<web_sys::File, String> {
    Ok(JsFuture::from(handle.get_file())
        .await
        .map_err(describe)?
        .unchecked_into())
}

async fn read_text(handle: &FileSystemFileHandle) -> Result<String, String> {
    let file = read_file(handle).await?;
    Ok(JsFuture::from(file.text())
        .await
        .map_err(describe)?
        .as_string()
        .unwrap_or_default())
}

async fn write(
    handle: &FileSystemFileHandle,
    write: impl FnOnce(&FileSystemWritableFileStream) -> Result<js_sys::Promise, JsValue>,
) -> Result<(), String> {
    let stream: FileSystemWritableFileStream = JsFuture::from(handle.create_writable())
        .await
        .map_err(describe)?
        .unchecked_into();
    JsFuture::from(write(&stream).map_err(describe)?)
        .await
        .map_err(describe)?;
    JsFuture::from(stream.close()).await.map_err(describe)?;
    Ok(())
}

/// The old "type"-discriminated dialect (packages/hexen-schema, and
/// what the TS editor's browser-folder storage wrote) doesn't parse
/// here; say how to convert it rather than just failing.
fn parse_error_hint(text: &str) -> &'static str {
    let old_dialect = ["inline", "obsidian", "hex", "square"]
        .iter()
        .any(|kind| text.contains(&format!("type: {kind}")));
    if old_dialect {
        " — this looks like the old \"type:\"-style project format; convert it with \
         `bun scripts/migrate-project-to-wire-format.ts`"
    } else {
        ""
    }
}

struct State {
    file: FileSystemFileHandle,
    history: History,
    warnings: Vec<String>,
    resolved_content: HashMap<String, ResolvedContent>,
    resolve_errors: HashMap<String, String>,
    /// The newest state not yet on disk, and whether a write is running.
    unsaved: Option<HexenProject>,
    writing: bool,
}

pub struct LocalFsStorage {
    dir: FileSystemDirectoryHandle,
    state: RefCell<Option<State>>,
    on_update: OnUpdate,
    on_error: OnError,
}

impl LocalFsStorage {
    pub fn connect(
        dir: FileSystemDirectoryHandle,
        on_update: OnUpdate,
        on_error: OnError,
    ) -> Rc<Self> {
        let storage = Rc::new(LocalFsStorage {
            dir,
            state: RefCell::new(None),
            on_update,
            on_error,
        });
        let opening = storage.clone();
        spawn_local(async move {
            if let Err(err) = opening.open().await {
                (opening.on_error)(err);
            }
        });
        storage
    }

    async fn open(&self) -> Result<(), String> {
        let file = find_hexen_file(&self.dir).await?;
        let text = read_text(&file).await?;
        let parsed =
            parse_hexen_project(&text).map_err(|e| format!("{e}{}", parse_error_hint(&text)))?;
        let (resolved_content, resolve_errors) = self.resolve_content(&parsed.project).await;
        *self.state.borrow_mut() = Some(State {
            file,
            history: History::new(parsed.project),
            warnings: parsed.warnings,
            resolved_content,
            resolve_errors,
            unsaved: None,
            writing: false,
        });
        self.notify();
        Ok(())
    }

    async fn resolve_content(
        &self,
        project: &HexenProject,
    ) -> (HashMap<String, ResolvedContent>, HashMap<String, String>) {
        let vault_root = match project.content.as_ref().and_then(|c| c.kind.as_ref()) {
            Some(project_content::Kind::Obsidian(o)) => Some(o.vault_root.as_str()),
            _ => None,
        };
        let mut resolved = HashMap::new();
        let mut errors = HashMap::new();
        for location in &project.locations {
            let result = match location.content.as_ref().and_then(|c| c.kind.as_ref()) {
                None => continue,
                Some(location_content::Kind::Inline(inline)) => Ok(resolve_inline_content(inline)),
                Some(location_content::Kind::Obsidian(note)) => match vault_root {
                    None => Err(no_vault_error(&location.id)),
                    Some(root) => {
                        match file_at(&self.dir, &vault_relative_path(root, &note.r#ref), false)
                            .await
                        {
                            Ok(handle) => read_text(&handle)
                                .await
                                .map(|raw| parse_obsidian_note(&raw, &note.r#ref)),
                            Err(err) => Err(err),
                        }
                    }
                },
            };
            match result {
                Ok(content) => {
                    resolved.insert(location.id.clone(), content);
                }
                Err(err) => {
                    errors.insert(location.id.clone(), err);
                }
            }
        }
        (resolved, errors)
    }

    /// Sends the current state to the editor. Inline content is
    /// re-resolved (commands can change it); vault notes can't be
    /// changed by any command, so their resolution from open() stands.
    fn notify(&self) {
        let data = {
            let mut state = self.state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            let project = state.history.current().clone();
            for location in &project.locations {
                if let Some(location_content::Kind::Inline(inline)) =
                    location.content.as_ref().and_then(|c| c.kind.as_ref())
                {
                    state
                        .resolved_content
                        .insert(location.id.clone(), resolve_inline_content(inline));
                    state.resolve_errors.remove(&location.id);
                }
            }
            state.warnings = validate_references(&project);
            OpenedProjectData {
                project: Some(project),
                warnings: state.warnings.clone(),
                resolved_content: state.resolved_content.clone(),
                resolve_errors: state.resolve_errors.clone(),
            }
        };
        (self.on_update)(data);
    }

    fn settle(self: &Rc<Self>, project: HexenProject) {
        self.notify();
        let start_writer = {
            let mut state = self.state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            state.unsaved = Some(project);
            !std::mem::replace(&mut state.writing, true)
        };
        if start_writer {
            let this = self.clone();
            spawn_local(async move { this.write_until_saved().await });
        }
    }

    async fn write_until_saved(&self) {
        loop {
            let next = {
                let mut state = self.state.borrow_mut();
                let Some(state) = state.as_mut() else { return };
                match state.unsaved.take() {
                    Some(project) => (state.file.clone(), project),
                    None => {
                        state.writing = false;
                        return;
                    }
                }
            };
            let (file, project) = next;
            let result = match serialize_hexen_project(&project) {
                Ok(yaml) => write(&file, |s| s.write_with_str(&yaml)).await,
                Err(err) => Err(err.to_string()),
            };
            if let Err(err) = result {
                (self.on_error)(format!("Couldn't save the project file: {err}"));
            }
        }
    }

    pub fn execute(self: &Rc<Self>, command: Command) {
        let result = {
            let mut state = self.state.borrow_mut();
            let Some(state) = state.as_mut() else { return };
            state.history.execute(command).cloned()
        };
        match result {
            Ok(project) => self.settle(project),
            Err(err) => web_sys::console::error_1(&format!("Command failed: {err}").into()),
        }
    }

    pub fn undo(self: &Rc<Self>) {
        let project = self
            .state
            .borrow_mut()
            .as_mut()
            .and_then(|s| s.history.undo().cloned());
        if let Some(project) = project {
            self.settle(project);
        }
    }

    pub fn redo(self: &Rc<Self>) {
        let project = self
            .state
            .borrow_mut()
            .as_mut()
            .and_then(|s| s.history.redo().cloned());
        if let Some(project) = project {
            self.settle(project);
        }
    }

    pub async fn image_url(&self, file: &str) -> Result<String, String> {
        let file = read_file(&file_at(&self.dir, file, false).await?).await?;
        web_sys::Url::create_object_url_with_blob(&file).map_err(describe)
    }

    /// Writes the image into the folder's `_assets/`, named after the
    /// Location (so re-uploading replaces it), then points the Location
    /// at it with an ordinary saveImage.
    pub async fn upload_image(
        self: &Rc<Self>,
        location_id: &str,
        file: web_sys::File,
    ) -> Result<(), String> {
        let ext = image_extension(&file).ok_or(UNSUPPORTED_IMAGE)?;
        let buffer = JsFuture::from(file.array_buffer())
            .await
            .map_err(describe)?;
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        let size = read_image_size(&bytes)
            .ok_or("Couldn't read image dimensions — is this really a PNG or JPEG?")?;

        let safe_ext = if ext == "jpeg" { "jpg" } else { ext };
        let safe_id: String = location_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let path = format!("_assets/{safe_id}.{safe_ext}");
        write(&file_at(&self.dir, &path, true).await?, |s| {
            s.write_with_u8_array(&bytes)
        })
        .await?;

        self.execute(Command {
            kind: Some(command::Kind::SaveImage(SaveImageCommand {
                location_id: location_id.to_owned(),
                image: Some(ImageRef {
                    file: path,
                    width: size.width as i32,
                    height: size.height as i32,
                }),
            })),
        });
        Ok(())
    }
}
