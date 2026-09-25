//! The landing screen's guts, also opened as a panel from inside the
//! editor — the same open/browse/create/recent surface either way.

use hexen_proto::hexen::v1::{
    project_content, InlineProjectContent, ObsidianProjectContent, ProjectContent,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::app::use_app;
use crate::desktop::{is_desktop, pick_project_file};
use crate::http::{encode, fetch_text, server_url};
use crate::recents::{add_local_recent, local_recents, recent_projects, LocalRecent};
use crate::storage::{request_readwrite, show_directory_picker, supports_local_fs};

fn join_path(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.trim().to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "location".into()
    } else {
        slug.into()
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DirEntry {
    name: String,
    is_directory: bool,
}

#[derive(Deserialize, Clone)]
struct DirectoryListing {
    path: String,
    parent: Option<String>,
    entries: Vec<DirEntry>,
}

#[component]
fn DirectoryBrowser(on_open: Callback<String>) -> impl IntoView {
    let browse_path = RwSignal::new(None::<String>);
    let listing = LocalResource::new(move || {
        let path = browse_path.get();
        async move {
            let mut url = format!("{}/directory", server_url());
            if let Some(path) = path {
                url.push_str(&format!("?path={}", encode(&path)));
            }
            let text = fetch_text("GET", &url, None, false).await?;
            serde_json::from_str::<DirectoryListing>(&text).map_err(|e| e.to_string())
        }
    });

    view! {
        <div class="browser">
            <Suspense fallback=|| view! { <p class="status">"Loading…"</p> }>
                {move || {
                    listing
                        .get()
                        .map(|result| match result {
                            Err(err) => view! { <p class="status error">{err}</p> }.into_any(),
                            Ok(listing) => {
                                let DirectoryListing { path, parent, entries } = listing;
                                let empty = entries.is_empty();
                                view! {
                                    <div class="browser-path">{path.clone()}</div>
                                    <ul class="browser-list">
                                        {parent
                                            .map(|parent| {
                                                view! {
                                                    <li>
                                                        <button on:click=move |_| {
                                                            browse_path.set(Some(parent.clone()))
                                                        }>".. (up)"</button>
                                                    </li>
                                                }
                                            })}
                                        {entries
                                            .into_iter()
                                            .map(|entry| {
                                                let full = join_path(&path, &entry.name);
                                                let label = if entry.is_directory {
                                                    format!("{}/", entry.name)
                                                } else {
                                                    entry.name.clone()
                                                };
                                                view! {
                                                    <li>
                                                        <button on:click=move |_| {
                                                            if entry.is_directory {
                                                                browse_path.set(Some(full.clone()))
                                                            } else {
                                                                on_open.run(full.clone())
                                                            }
                                                        }>{label}</button>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                        {empty.then(|| view! { <li class="muted">"Nothing to open here."</li> })}
                                    </ul>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[derive(Clone)]
struct NewProjectValues {
    path: String,
    title: String,
    vault_root: Option<String>,
}

#[component]
fn NewProjectForm(
    on_create: Callback<NewProjectValues>,
    #[prop(into)] saving: Signal<bool>,
    #[prop(into)] error: Signal<Option<String>>,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let path = RwSignal::new(String::new());
    let obsidian = RwSignal::new(false);
    let vault_root = RwSignal::new(".".to_string());
    let can_create = move || {
        !title.get().trim().is_empty()
            && !path.get().trim().is_empty()
            && (!obsidian.get() || !vault_root.get().trim().is_empty())
    };

    view! {
        <form
            class="link-form"
            on:submit=move |e| {
                e.prevent_default();
                if can_create() {
                    on_create
                        .run(NewProjectValues {
                            title: title.get().trim().into(),
                            path: path.get().trim().into(),
                            vault_root: obsidian.get().then(|| vault_root.get().trim().into()),
                        });
                }
            }
        >
            <label>
                "Title" <input type="text" placeholder="My Realm" autofocus bind:value=title />
            </label>
            <label>
                "Save as" <input type="text" placeholder="/path/to/my-realm.hexen.yml" bind:value=path />
            </label>

            <fieldset class="content-type-choice">
                <legend>"Content"</legend>
                <label class="radio-label">
                    <input
                        type="radio"
                        name="content-type"
                        prop:checked=move || !obsidian.get()
                        on:change=move |_| obsidian.set(false)
                    />
                    "Inline — write title/body straight into the .hexen.yml"
                </label>
                <label class="radio-label">
                    <input
                        type="radio"
                        name="content-type"
                        prop:checked=move || obsidian.get()
                        on:change=move |_| obsidian.set(true)
                    />
                    "Obsidian vault — locations resolve to markdown files"
                </label>
                <Show when=move || obsidian.get()>
                    <label>
                        "Vault root"
                        <input type="text" placeholder=". (relative to the .hexen.yml file)" bind:value=vault_root />
                    </label>
                </Show>
            </fieldset>

            {move || error.get().map(|e| view! { <span class="field-error">{e}</span> })}
            <button type="submit" disabled=move || !can_create() || saving.get()>
                {move || if saving.get() { "Creating…" } else { "Create" }}
            </button>
        </form>
    }
}

#[component]
pub fn ProjectPicker(
    /// Passed when mounted as an in-editor overlay rather than the
    /// full-page landing state — closes itself once a project opens.
    #[prop(optional, into)]
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    let app = use_app();
    let path_input = RwSignal::new(String::new());
    let browsing = RwSignal::new(false);
    let creating = RwSignal::new(false);
    let recent = recent_projects();
    let local_recent = RwSignal::new_local(Vec::<LocalRecent>::new());
    let local_error = RwSignal::new(None::<String>);
    let creating_project = RwSignal::new(false);
    let create_error = RwSignal::new(None::<String>);
    // The desktop app opens projects through its own server via the
    // native file dialog; the browser-folder backend is for the web
    // build (and not every system webview supports it anyway).
    let desktop = is_desktop();
    let local_fs = !desktop && supports_local_fs();

    if local_fs {
        spawn_local(async move { local_recent.set(local_recents().await) });
    }

    let close = move || {
        if let Some(on_close) = on_close {
            on_close.run(());
        }
    };
    let open_and_close = Callback::new(move |path: String| {
        app.open_server_project(path);
        close();
    });

    let create_project = Callback::new(move |values: NewProjectValues| {
        creating_project.set(true);
        create_error.set(None);
        spawn_local(async move {
            let content = ProjectContent {
                kind: Some(match &values.vault_root {
                    Some(vault_root) => project_content::Kind::Obsidian(ObsidianProjectContent {
                        vault_root: vault_root.clone(),
                    }),
                    None => project_content::Kind::Inline(InlineProjectContent {}),
                }),
            };
            let body = serde_json::json!({
                "path": values.path,
                "title": values.title,
                "defaultLocationId": slugify(&values.title),
                "content": content,
            });
            let result = fetch_text(
                "POST",
                &format!("{}/project", server_url()),
                Some(&body.to_string().into()),
                true,
            )
            .await;
            creating_project.set(false);
            match result {
                Ok(_) => open_and_close.run(values.path),
                Err(err) => create_error.set(Some(err)),
            }
        });
    });

    let open_from_browser = move |_| {
        local_error.set(None);
        spawn_local(async move {
            match show_directory_picker().await {
                Ok(Some(dir)) => {
                    let _ = add_local_recent(&dir).await;
                    app.open_folder(dir);
                    close();
                }
                Ok(None) => {} // closing the picker without choosing isn't an error
                Err(err) => local_error.set(Some(err)),
            }
        });
    };

    let open_file_dialog = move |_| {
        local_error.set(None);
        spawn_local(async move {
            match pick_project_file().await {
                Ok(Some(path)) => open_and_close.run(path),
                Ok(None) => {}
                Err(err) => local_error.set(Some(err)),
            }
        });
    };

    let open_local_recent = move |entry: LocalRecent| {
        local_error.set(None);
        spawn_local(async move {
            match request_readwrite(&entry.handle).await {
                Ok(true) => {
                    let _ = add_local_recent(&entry.handle).await;
                    app.open_folder(entry.handle);
                    close();
                }
                Ok(false) => local_error.set(Some(format!(
                    "Permission to \"{}\" was denied.",
                    entry.name
                ))),
                Err(err) => local_error.set(Some(err)),
            }
        });
    };

    view! {
        <div class="project-picker">
            {on_close
                .map(|_| {
                    view! {
                        <button type="button" class="link-button picker-close" on:click=move |_| close()>
                            "Cancel"
                        </button>
                    }
                })}

            {if desktop {
                view! {
                    <button type="button" class="tool-button" on:click=open_file_dialog>
                        "Open a project file…"
                    </button>
                }
                    .into_any()
            } else if local_fs {
                view! {
                    <button type="button" class="tool-button" on:click=open_from_browser>
                        "Open from this browser…"
                    </button>
                }
                    .into_any()
            } else {
                view! {
                    <p class="muted">
                        "This browser can't open a project folder directly (Chrome and Edge can) — use a locally-running server instead, below."
                    </p>
                }
                    .into_any()
            }}
            {move || local_error.get().map(|e| view! { <span class="field-error">{e}</span> })}

            <Show when=move || local_recent.with(|r| !r.is_empty())>
                <div class="recent-projects">
                    <h3>"Recent (this browser)"</h3>
                    <ul>
                        {move || {
                            local_recent
                                .get()
                                .into_iter()
                                .map(|entry| {
                                    let name = entry.name.clone();
                                    view! {
                                        <li>
                                            <button
                                                class="link-button"
                                                on:click=move |_| open_local_recent(entry.clone())
                                            >
                                                {name}
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </div>
            </Show>

            <p>
                {if desktop {
                    "Or open one by its absolute path."
                } else {
                    "Or, open a .hexen.yml project on a locally-running server, by its absolute path."
                }}
            </p>
            <form on:submit=move |e| {
                e.prevent_default();
                let path = path_input.get().trim().to_string();
                if !path.is_empty() {
                    open_and_close.run(path);
                }
            }>
                <input type="text" placeholder="/path/to/project.hexen.yml" bind:value=path_input />
                <button type="submit">"Open"</button>
            </form>

            <button class="link-button" on:click=move |_| browsing.update(|b| *b = !*b)>
                {move || if browsing.get() { "Hide browser" } else { "Browse for a project…" }}
            </button>
            <Show when=move || browsing.get()>
                <DirectoryBrowser on_open=open_and_close />
            </Show>

            <button class="link-button" on:click=move |_| creating.update(|c| *c = !*c)>
                {move || if creating.get() { "Cancel new project" } else { "New project…" }}
            </button>
            <Show when=move || creating.get()>
                <NewProjectForm on_create=create_project saving=creating_project error=create_error />
            </Show>

            {(!recent.is_empty())
                .then(|| {
                    view! {
                        <div class="recent-projects">
                            <h3>"Recent (server)"</h3>
                            <ul>
                                {recent
                                    .into_iter()
                                    .map(|path| {
                                        let label = path.clone();
                                        view! {
                                            <li>
                                                <button
                                                    class="link-button"
                                                    on:click=move |_| open_and_close.run(path.clone())
                                                >
                                                    {label}
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    }
                })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_matches_the_ts_editor() {
        assert_eq!(slugify("  My Realm! "), "my-realm");
        assert_eq!(slugify("Ünïcode & Co."), "n-code-co");
        assert_eq!(slugify("!!!"), "location");
    }
}
