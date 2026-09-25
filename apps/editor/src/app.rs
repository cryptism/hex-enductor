use leptos::prelude::*;
use web_sys::FileSystemDirectoryHandle;

use crate::editor::Editor;
use crate::logo::BrandMark;
use crate::picker::ProjectPicker;
use crate::recents::add_recent_project;
use crate::storage::Source;
use crate::ui_state::UiState;

/// App-wide state, provided as context: which project is open (a local
/// signal — a folder handle isn't Send) and the UI switches.
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub source: RwSignal<Option<Source>, LocalStorage>,
    pub ui: RwSignal<UiState>,
}

impl AppCtx {
    /// The server-backed path — open is keyed to an absolute path string.
    pub fn open_server_project(&self, path: String) {
        add_recent_project(&path);
        self.open(Source::Server(path));
    }

    /// A folder picked in this browser — no path string to remember.
    pub fn open_folder(&self, dir: FileSystemDirectoryHandle) {
        self.open(Source::Folder(dir));
    }

    fn open(&self, source: Source) {
        self.ui.update(UiState::open_project);
        self.source.set(Some(source));
    }
}

pub fn use_app() -> AppCtx {
    expect_context::<AppCtx>()
}

#[component]
pub fn App() -> impl IntoView {
    let ctx = AppCtx {
        source: RwSignal::new_local(None),
        ui: RwSignal::new(UiState::default()),
    };
    provide_context(ctx);

    // Keyed on each open (not just on "is something open"), so switching
    // projects tears the old editor — and its connection — down.
    let generation = Memo::new(move |previous: Option<&u64>| {
        ctx.source.track();
        previous.map_or(0, |g| g + 1)
    });

    move || {
        generation.track();
        if ctx.source.with(Option::is_none) {
            view! {
                <div class="open-project">
                    <h1 class="brand-heading">
                        <BrandMark size=64 />
                    </h1>
                    <ProjectPicker />
                </div>
            }
            .into_any()
        } else {
            view! { <Editor /> }.into_any()
        }
    }
}
