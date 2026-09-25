//! An open project: sidebar, map, and the side panel — port of
//! apps/editor's App.tsx. Mounted once per open (see app.rs), so its
//! storage connection lives exactly as long as it does.

use std::collections::HashMap;
use std::rc::Rc;

use hexen_proto::hexen::v1::{
    command, location_content, AddLocationCommand, AddLocationLinkCommand, Command, FogOfWar, Grid,
    OpenedProjectData, Point, SaveGridCommand, SaveImageCommand, SaveLinkCommand,
    SaveLocationContentCommand, SetFogCellsCommand, SetFogCommand,
};
use hexen_web::map_canvas::MapCanvas;
use leptos::html::Input;
use leptos::prelude::*;
use leptos::task::spawn_local;
use map_core::link_icons::find_link_icon;

use crate::about::AboutModal;
use crate::app::use_app;
use crate::desktop::is_desktop;
use crate::fog_controls::FogControls;
use crate::forms::{AddLocationForm, ConfigureGridForm, LinkForm, LocationContentForm};
use crate::location_browser::LocationBrowser;
use crate::logo::{LoadingScreen, Logo};
use crate::picker::ProjectPicker;
use crate::server_panel::ServerPanel;
use crate::storage::{ImageUrl, Source, Storage};

fn icon_swatch(slug: Option<&str>) -> Option<impl IntoView> {
    find_link_icon(slug).map(|icon| view! { <span class="icon-swatch" inner_html=icon.svg></span> })
}

#[derive(Clone, PartialEq)]
enum Phase {
    Loading,
    Failed(String),
    UnknownLocation(String),
    Ready,
}

#[component]
pub fn Editor() -> impl IntoView {
    let app = use_app();
    let ui = app.ui;
    let source = app
        .source
        .get_untracked()
        .expect("Editor is only mounted with a project open");

    let data = RwSignal::new(None::<OpenedProjectData>);
    let load_error = RwSignal::new(None::<String>);
    let runtime_error = RwSignal::new(None::<String>);
    let saving_link = RwSignal::new(false);
    let saving_content = RwSignal::new(false);
    let adding_location = RwSignal::new(false);
    let saving_grid = RwSignal::new(false);

    // Every state change — this storage's own commands settling, or
    // (server) another connected client's — arrives here.
    let storage = Storage::connect(
        &source,
        Rc::new(move |d| {
            data.set(Some(d));
            runtime_error.set(None);
            saving_link.set(false);
            saving_content.set(false);
            adding_location.set(false);
            saving_grid.set(false);
        }),
        Rc::new(move |err| {
            if data.with_untracked(Option::is_none) {
                load_error.set(Some(err));
            } else {
                runtime_error.set(Some(err));
            }
        }),
    );
    let storage = StoredValue::new_local(storage);
    let execute =
        move |kind: command::Kind| storage.with_value(|s| s.execute(Command { kind: Some(kind) }));

    let project = Memo::new(move |_| data.with(|d| d.as_ref().and_then(|d| d.project.clone())));
    let resolved = Memo::new(move |_| {
        data.with(|d| {
            d.as_ref()
                .map(|d| d.resolved_content.clone())
                .unwrap_or_default()
        })
    });
    let title_of = move |id: &str| resolved.with(|r| r.get(id).map(|c| c.title.clone()));

    let current_location_id = Memo::new(move |_| ui.with(|u| u.current_location_id.clone()));
    let selected_link_id = Memo::new(move |_| ui.with(|u| u.selected_link_id.clone()));
    let edit_mode = Memo::new(move |_| ui.with(|u| u.edit_mode));
    let gm_mode = Memo::new(move |_| ui.with(|u| u.gm_mode));
    let painting_fog = Memo::new(move |_| ui.with(|u| u.painting_fog));
    let grid_visible = Memo::new(move |_| ui.with(|u| u.grid_visible));
    let placing_location = Memo::new(move |_| ui.with(|u| u.placing_location));
    let set_location = move |id: String| ui.update(|u| u.set_current_location(Some(id)));

    // Default to the project's defaultLocation once it loads.
    Effect::new(move |_| {
        if let Some(default) = project.with(|p| p.as_ref().map(|p| p.default_location.clone())) {
            if current_location_id.with_untracked(Option::is_none) {
                ui.update(|u| u.set_current_location(Some(default)));
            }
        }
    });

    let location = Memo::new(move |_| {
        let id = current_location_id.get()?;
        project.with(|p| p.as_ref()?.locations.iter().find(|l| l.id == id).cloned())
    });
    let location_id =
        Memo::new(move |_| location.with(|l| l.as_ref().map(|l| l.id.clone()).unwrap_or_default()));
    let image = Memo::new(move |_| location.with(|l| l.as_ref().and_then(|l| l.image.clone())));
    let links = Memo::new(move |_| {
        location.with(|l| l.as_ref().map(|l| l.links.clone()).unwrap_or_default())
    });
    let link_titles = Memo::new(move |_| {
        links.with(|links| {
            links
                .iter()
                .map(|l| {
                    (
                        l.target.clone(),
                        title_of(&l.target).unwrap_or_else(|| l.target.clone()),
                    )
                })
                .collect::<HashMap<_, _>>()
        })
    });
    let is_inline = Memo::new(move |_| {
        location.with(|l| {
            l.as_ref()
                .is_some_and(|l| match l.content.as_ref().and_then(|c| c.kind.as_ref()) {
                    None => true,
                    Some(location_content::Kind::Inline(_)) => true,
                    Some(location_content::Kind::Obsidian(_)) => false,
                })
        })
    });
    let location_title = Memo::new(move |_| title_of(&location_id.get()).unwrap_or_default());
    let location_body = Memo::new(move |_| {
        resolved.with(|r| {
            r.get(&location_id.get())
                .map(|c| c.body.clone())
                .unwrap_or_default()
        })
    });

    let phase = Memo::new(move |_| {
        if let Some(err) = load_error.get() {
            return Phase::Failed(err);
        }
        if project.with(Option::is_none) {
            return Phase::Loading;
        }
        match current_location_id.get() {
            None => Phase::Loading,
            Some(id) if location.with(Option::is_none) => Phase::UnknownLocation(id),
            Some(_) => Phase::Ready,
        }
    });

    // Purely a local view toggle — like Krita's layer-visibility eye, it
    // lets the GM peek at the raw map under the fog without changing what
    // players see. Reset whenever GM mode is (re)entered.
    let fog_layer_visible = RwSignal::new(true);
    Effect::new(move |_| {
        if gm_mode.get() {
            fog_layer_visible.set(true);
        }
    });

    // Undo/redo shortcuts, only while editing.
    let keys = window_event_listener(leptos::ev::keydown, move |e| {
        if !edit_mode.get_untracked()
            || !(e.meta_key() || e.ctrl_key())
            || e.key().to_lowercase() != "z"
        {
            return;
        }
        e.prevent_default();
        storage.with_value(|s| if e.shift_key() { s.redo() } else { s.undo() });
    });
    on_cleanup(move || keys.remove());

    // Where the Add Location tool's pending click landed — meaningless the
    // moment the tool is disarmed.
    let pending_point = RwSignal::new(None::<Point>);
    Effect::new(move |_| {
        if !placing_location.get() {
            pending_point.set(None);
        }
    });

    // Configure Grid is a command, not a tool: opening it hands the map a
    // draft grid to render live, and only Apply commits it.
    let configuring_grid = RwSignal::new(false);
    let draft_grid = RwSignal::new(None::<Grid>);
    Effect::new(move |_| {
        current_location_id.track();
        edit_mode.track();
        gm_mode.track();
        configuring_grid.set(false);
        draft_grid.set(None);
    });
    let stop_configuring = move || {
        configuring_grid.set(false);
        draft_grid.set(None);
    };

    let picker_open = RwSignal::new(false);
    let server_open = RwSignal::new(false);
    let project_path = match &source {
        Source::Server(path) => Some(path.clone()),
        Source::Folder(_) => None,
    };
    let about_open = RwSignal::new(false);
    let browser_open = RwSignal::new(false);

    // The current image resolved to something displayable — async, since
    // a folder-backed image is read through its handle into a blob: URL
    // (revoked when replaced; see ImageUrl).
    let image_file = Memo::new(move |_| image.with(|i| i.as_ref().map(|i| i.file.clone())));
    let image_url = RwSignal::new(None::<String>);
    let image_url_handle = StoredValue::new_local(None::<ImageUrl>);
    let image_request = StoredValue::new(0u64);
    Effect::new(move |_| {
        let file = image_file.get();
        image_request.update_value(|n| *n += 1);
        let request = image_request.get_value();
        image_url.set(None);
        image_url_handle.set_value(None);
        let Some(file) = file else { return };
        let storage = storage.get_value();
        spawn_local(async move {
            let result = storage.image_url(&file).await;
            if image_request.try_get_value() != Some(request) {
                return; // superseded (or unmounted) while loading
            }
            match result {
                Ok(url) => {
                    image_url.set(Some(url.url.clone()));
                    image_url_handle.set_value(Some(url));
                }
                Err(err) => runtime_error.set(Some(err)),
            }
        });
    });

    let map_ready =
        Memo::new(move |_| image.with(Option::is_some) && image_url.with(Option::is_some));
    let has_image = Memo::new(move |_| image.with(Option::is_some));

    // ——— sidebar pieces ———

    let fog_controls = move || {
        location_id.track();
        (gm_mode.get() && has_image.get()).then(|| {
            view! {
                <FogControls
                    image=Signal::derive(move || image.get().unwrap_or_default())
                    fog=Signal::derive(move || location.with(|l| l.as_ref().and_then(|l| l.fog.clone())))
                    edit_mode
                    layer_visible=fog_layer_visible
                    painting_fog
                    on_set_painting_fog=Callback::new(move |painting: bool| {
                        if painting {
                            ui.update(|u| u.set_placing_location(false));
                            stop_configuring();
                            fog_layer_visible.set(true);
                        }
                        ui.update(|u| u.set_painting_fog(painting));
                    })
                    on_set_fog=Callback::new(move |fog: Option<FogOfWar>| {
                        execute(command::Kind::SetFog(SetFogCommand { location_id: location_id.get_untracked(), fog }))
                    })
                />
            }
        })
    };

    let image_upload = move || {
        location_id.track();
        let input = NodeRef::<Input>::new();
        let uploading = RwSignal::new(false);
        let error = RwSignal::new(None::<String>);
        let on_file = move |_| {
            let Some(el) = input.get() else { return };
            let file = el.files().and_then(|f| f.get(0));
            el.set_value("");
            let Some(file) = file else { return };
            uploading.set(true);
            error.set(None);
            let storage = storage.get_value();
            let id = location_id.get_untracked();
            spawn_local(async move {
                let result = storage.upload_image(&id, file).await;
                let _ = uploading.try_set(false);
                if let Err(err) = result {
                    let _ = error.try_set(Some(err));
                }
            });
        };
        view! {
            <div class="image-upload">
                <input node_ref=input type="file" accept="image/png,image/jpeg" hidden on:change=on_file />
                <button
                    type="button"
                    class="tool-button"
                    disabled=move || uploading.get()
                    on:click=move |_| {
                        if let Some(el) = input.get() {
                            el.click();
                        }
                    }
                >
                    {move || {
                        if uploading.get() {
                            "Uploading…"
                        } else if has_image.get() {
                            "Replace image…"
                        } else {
                            "Upload image…"
                        }
                    }}
                </button>
                <Show when=move || has_image.get()>
                    <button
                        type="button"
                        class="link-button"
                        disabled=move || uploading.get()
                        on:click=move |_| {
                            error.set(None);
                            execute(command::Kind::SaveImage(SaveImageCommand {
                                location_id: location_id.get_untracked(),
                                image: None,
                            }));
                        }
                    >
                        "Remove image"
                    </button>
                </Show>
                {move || error.get().map(|e| view! { <span class="field-error">{e}</span> })}
            </div>
        }
    };

    let content_block = move || {
        location_id.track();
        if edit_mode.get() && is_inline.get() {
            view! {
                <LocationContentForm
                    title=location_title
                    body=location_body
                    saving=saving_content
                    on_save=Callback::new(move |patch| {
                        saving_content.set(true);
                        execute(command::Kind::SaveLocationContent(SaveLocationContentCommand {
                            location_id: location_id.get_untracked(),
                            patch: Some(patch),
                        }));
                    })
                />
            }
            .into_any()
        } else {
            view! {
                <div class="location-heading">
                    <h2>
                        {move || {
                            let title = location_title.get();
                            if title.is_empty() { location_id.get() } else { title }
                        }}
                    </h2>
                    {move || {
                        let body = location_body.get();
                        (is_inline.get() && !body.is_empty()).then(|| view! { <p class="location-body">{body}</p> })
                    }}
                </div>
            }
            .into_any()
        }
    };

    let warnings = move || {
        data.with(|d| {
            let d = d.as_ref()?;
            let count = d.warnings.len() + d.resolve_errors.len();
            (count > 0).then(|| {
                let mut errors: Vec<_> = d.resolve_errors.iter().map(|(id, e)| format!("{id}: {e}")).collect();
                errors.sort();
                view! {
                    <details class="warnings">
                        <summary>{count} " warning(s)"</summary>
                        <ul>
                            {d.warnings.iter().chain(errors.iter()).map(|w| view! { <li>{w.clone()}</li> }).collect_view()}
                        </ul>
                    </details>
                }
            })
        })
    };

    let link_list = move || {
        let links = links.get();
        let empty = links.is_empty();
        view! {
            <ul class="link-list">
                {links
                    .into_iter()
                    .map(|link| {
                        let id = link.id.clone();
                        let selected = {
                            let id = id.clone();
                            move || selected_link_id.get().as_deref() == Some(id.as_str())
                        };
                        let title = link_titles.with(|t| t.get(&link.target).cloned()).unwrap_or_default();
                        view! {
                            <li>
                                <button
                                    class=move || if selected() { "selected" } else { "" }
                                    on:click=move |_| ui.update(|u| u.select_link(Some(id.clone())))
                                >
                                    {icon_swatch(link.icon.as_deref())}
                                    {title}
                                    {link.hidden.unwrap_or(false).then_some(" (hidden)")}
                                </button>
                            </li>
                        }
                    })
                    .collect_view()}
                {empty.then(|| view! { <li class="muted">"No locations pinned here yet."</li> })}
            </ul>
        }
    };

    let sidebar = view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <button
                    type="button"
                    class="logo-button"
                    aria-label="About Hex Enductor"
                    on:click=move |_| about_open.set(true)
                >
                    <Logo size=28 />
                </button>
                <button type="button" class="project-switcher" on:click=move |_| picker_open.set(true)>
                    {move || project.with(|p| p.as_ref().map(|p| p.title.clone()).unwrap_or_default())}
                </button>
            </div>
            <button type="button" class="link-button sidebar-spaced" on:click=move |_| browser_open.set(true)>
                "Browse all locations…"
            </button>
            {is_desktop()
                .then(|| {
                    view! {
                        <button type="button" class="link-button sidebar-spaced" on:click=move |_| server_open.set(true)>
                            "Server & presentation…"
                        </button>
                    }
                })}

            <label class="mode-toggle">
                <input
                    type="checkbox"
                    prop:checked=edit_mode
                    on:change=move |ev| ui.update(|u| u.set_edit_mode(event_target_checked(&ev)))
                />
                <span class="mode-toggle-track" aria-hidden="true"></span>
                <span class="mode-toggle-label">{move || if edit_mode.get() { "Editing" } else { "Viewing" }}</span>
            </label>

            <label class="mode-toggle">
                <input
                    type="checkbox"
                    prop:checked=gm_mode
                    on:change=move |ev| ui.update(|u| u.set_gm_mode(event_target_checked(&ev)))
                />
                <span class="mode-toggle-track" aria-hidden="true"></span>
                <span class="mode-toggle-label">
                    {move || if gm_mode.get() { "GM mode on" } else { "GM mode off" }}
                </span>
            </label>

            {fog_controls}

            <Show when=move || edit_mode.get()>
                <div class="tool-row">
                    <button type="button" class="tool-button" on:click=move |_| storage.with_value(|s| s.undo())>
                        "Undo"
                    </button>
                    <button type="button" class="tool-button" on:click=move |_| storage.with_value(|s| s.redo())>
                        "Redo"
                    </button>
                </div>

                <button
                    type="button"
                    class=move || if placing_location.get() { "tool-button active" } else { "tool-button" }
                    on:click=move |_| {
                        stop_configuring();
                        ui.update(|u| {
                            u.set_painting_fog(false);
                            let placing = !u.placing_location;
                            u.set_placing_location(placing);
                        });
                    }
                >
                    {move || if placing_location.get() { "Click the map…" } else { "Add Location" }}
                </button>

                {image_upload}

                <Show when=move || has_image.get()>
                    <button
                        type="button"
                        class="tool-button"
                        on:click=move |_| {
                            ui.update(|u| {
                                u.set_placing_location(false);
                                u.set_painting_fog(false);
                            });
                            draft_grid.set(location.with_untracked(|l| l.as_ref().and_then(|l| l.grid.clone())));
                            configuring_grid.set(true);
                        }
                    >
                        "Configure grid…"
                    </button>
                </Show>
            </Show>

            {content_block}

            {move || {
                let default = project.with(|p| p.as_ref().map(|p| (p.default_location.clone(), p.title.clone())));
                default
                    .filter(|(default, _)| *default != location_id.get())
                    .map(|(default, title)| {
                        view! {
                            <button class="link-button" on:click=move |_| set_location(default.clone())>
                                "← "
                                {title}
                            </button>
                        }
                    })
            }}

            {move || runtime_error.get().map(|e| view! { <p class="field-error">{e}</p> })}
            {warnings}
            {link_list}
        </aside>
    };

    // ——— map ———

    let map = view! {
        <main class="map-area">
            <Show
                when=move || map_ready.get()
                fallback=move || {
                    if has_image.get() {
                        view! { <div class="status">"Loading image…"</div> }.into_any()
                    } else {
                        view! {
                            <div class="status">"\"" {location_id} "\" has no image — nothing to render."</div>
                        }
                            .into_any()
                    }
                }
            >
                <MapCanvas
                    image=Signal::derive(move || image.get().unwrap_or_default())
                    image_url=Signal::derive(move || image_url.get().unwrap_or_default())
                    grid=Signal::derive(move || {
                        if configuring_grid.get() {
                            draft_grid.get()
                        } else {
                            location.with(|l| l.as_ref().and_then(|l| l.grid.clone()))
                        }
                    })
                    grid_visible=Signal::derive(move || grid_visible.get() || configuring_grid.get())
                    links
                    link_titles
                    selected_link_id
                    on_select_link=Callback::new(move |id| ui.update(|u| u.select_link(Some(id))))
                    placing=Signal::derive(move || placing_location.get() && !configuring_grid.get())
                    on_place=Callback::new(move |p| pending_point.set(Some(p)))
                    fog=Signal::derive(move || {
                        if gm_mode.get() && fog_layer_visible.get() {
                            location.with(|l| l.as_ref().and_then(|l| l.fog.clone()))
                        } else {
                            None
                        }
                    })
                    fog_editable=painting_fog
                    on_paint_fog_cells=Callback::new(move |(cells, revealed): (Vec<String>, bool)| {
                        execute(command::Kind::SetFogCells(SetFogCellsCommand {
                            location_id: location_id.get_untracked(),
                            cells,
                            revealed,
                        }))
                    })
                />
                <label class="grid-toggle">
                    <input
                        type="checkbox"
                        prop:checked=grid_visible
                        on:change=move |ev| ui.update(|u| u.set_grid_visible(event_target_checked(&ev)))
                    />
                    "Show grid"
                </label>
            </Show>
        </main>
    };

    // ——— side panel ———

    let selected_link = Memo::new(move |_| {
        let id = selected_link_id.get()?;
        links.with(|links| links.iter().find(|l| l.id == id).cloned())
    });
    let selected_link_key =
        Memo::new(move |_| selected_link.with(|l| l.as_ref().map(|l| l.id.clone())));

    #[derive(Clone, Copy, PartialEq)]
    enum Panel {
        None,
        Grid,
        AddLocation,
        Link,
    }
    let panel = Memo::new(move |_| {
        if configuring_grid.get() && has_image.get() {
            Panel::Grid
        } else if placing_location.get() && pending_point.with(Option::is_some) {
            Panel::AddLocation
        } else if selected_link_key.with(Option::is_some) {
            Panel::Link
        } else {
            Panel::None
        }
    });

    let link_panel = move || {
        selected_link_key.track();
        edit_mode.track();
        let link = Signal::derive(move || selected_link.get().unwrap_or_default());
        let title = Signal::derive(move || {
            link.with(|l| {
                link_titles
                    .with(|t| t.get(&l.target).cloned())
                    .unwrap_or_else(|| l.target.clone())
            })
        });
        let target_has_image = move || {
            let target = link.with(|l| l.target.clone());
            project.with(|p| {
                p.as_ref().is_some_and(|p| {
                    p.locations
                        .iter()
                        .any(|l| l.id == target && l.image.is_some())
                })
            })
        };
        view! {
            {if edit_mode.get_untracked() {
                view! {
                    <LinkForm
                        link
                        title
                        saving=saving_link
                        on_save=Callback::new(move |patch| {
                            saving_link.set(true);
                            execute(command::Kind::SaveLink(SaveLinkCommand {
                                location_id: location_id.get_untracked(),
                                link_id: link.with_untracked(|l| l.id.clone()),
                                patch: Some(patch),
                            }));
                        })
                    />
                }
                    .into_any()
            } else {
                view! {
                    <div class="location-heading">
                        <h2 class="link-form-title">
                            {move || link.with(|l| icon_swatch(l.icon.as_deref()))}
                            {title}
                        </h2>
                        <p class="location-body">{move || link.with(|l| l.r#type.clone())}</p>
                    </div>
                }
                    .into_any()
            }}
            // Navigation, not a mutation — available in view mode too.
            <Show when=target_has_image>
                <button class="link-button" on:click=move |_| set_location(link.with_untracked(|l| l.target.clone()))>
                    "View map →"
                </button>
            </Show>
        }
    };

    let side_panel = move || match panel.get() {
        Panel::None => None,
        Panel::Grid => Some(
            view! {
                <aside class="edit-panel">
                    <ConfigureGridForm
                        initial_grid=draft_grid.get_untracked()
                        image=image.get_untracked().unwrap_or_default()
                        on_preview=Callback::new(move |g| draft_grid.set(Some(g)))
                        saving=saving_grid
                        on_apply=Callback::new(move |grid: Grid| {
                            saving_grid.set(true);
                            execute(command::Kind::SaveGrid(SaveGridCommand {
                                location_id: location_id.get_untracked(),
                                grid: Some(grid),
                            }));
                            stop_configuring();
                        })
                        on_cancel=Callback::new(move |_| stop_configuring())
                    />
                </aside>
            }
            .into_any(),
        ),
        Panel::AddLocation => Some(
            view! {
                <aside class="edit-panel">
                    <AddLocationForm
                        saving=adding_location
                        on_save=Callback::new(move |(target, link_type): (String, String)| {
                            let Some(point) = pending_point.get_untracked() else { return };
                            adding_location.set(true);
                            execute(command::Kind::AddLocationLink(AddLocationLinkCommand {
                                parent_location_id: location_id.get_untracked(),
                                target_location_id: target,
                                x: point.x,
                                y: point.y,
                                link_type,
                            }));
                            ui.update(|u| u.set_placing_location(false));
                        })
                        on_cancel=Callback::new(move |_| pending_point.set(None))
                    />
                </aside>
            }
            .into_any(),
        ),
        Panel::Link => Some(view! { <aside class="edit-panel">{link_panel}</aside> }.into_any()),
    };

    // Built once and hidden, never unmounted, while a status shows: a
    // transient state (navigating to a just-created location before the
    // update adding it arrives) mustn't tear down the map.
    let is_ready = Memo::new(move |_| phase.get() == Phase::Ready);
    let ready = view! {
        <div class="app" style=move || if is_ready.get() { "" } else { "display: none" }>
            {sidebar}
            {map}
            {side_panel}
        </div>
    };

    view! {
        <LoadingScreen active=Signal::derive(move || project.with(Option::is_none) && load_error.with(Option::is_none)) />
        {move || match phase.get() {
            Phase::Loading | Phase::Ready => None,
            Phase::Failed(err) => Some(view! { <div class="status error">{err}</div> }.into_any()),
            Phase::UnknownLocation(id) => {
                Some(view! { <div class="status error">"Unknown location \"" {id} "\""</div> }.into_any())
            }
        }}
        {ready}
        <Show when=move || about_open.get()>
            <AboutModal on_close=Callback::new(move |_| about_open.set(false)) />
        </Show>
        <Show when=move || server_open.get()>
            <ServerPanel project_path=project_path.clone() on_close=Callback::new(move |_| server_open.set(false)) />
        </Show>
        <Show when=move || picker_open.get()>
            <div class="modal-backdrop">
                <div class="modal-panel">
                    <ProjectPicker on_close=Callback::new(move |_| picker_open.set(false)) />
                </div>
            </div>
        </Show>
        <Show when=move || browser_open.get() && project.with(Option::is_some)>
            <div class="modal-backdrop">
                <div class="modal-panel">
                    <LocationBrowser
                        project=Signal::derive(move || project.get().unwrap_or_default())
                        resolved_content=resolved
                        current_location_id
                        on_select=Callback::new(set_location)
                        on_close=Callback::new(move |_| browser_open.set(false))
                        edit_mode
                        on_create=Callback::new(move |id: String| {
                            execute(command::Kind::AddLocation(AddLocationCommand { location_id: id }))
                        })
                    />
                </div>
            </div>
        </Show>
    }
}
