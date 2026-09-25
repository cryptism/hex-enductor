use std::collections::HashMap;

use hexen_proto::hexen::v1::{Location, OpenedProjectData};
use leptos::prelude::*;
use map_core::link_icons::find_link_icon;

use crate::live_session::{self, LiveSession};
use crate::map_canvas::MapCanvas;

#[derive(Clone, PartialEq)]
struct Target {
    server: String,
    path: String,
}

// This app never edits and never browses the filesystem — it's handed
// exactly which project to watch via its own URL query string, the
// same idea as an embed link:
// ?server=http://localhost:4000&path=/abs/project.hexen.yml
fn target_from_url() -> Option<Target> {
    let search = window().location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let server = params.get("server").filter(|s| !s.is_empty())?;
    let path = params.get("path").filter(|s| !s.is_empty())?;
    Some(Target { server, path })
}

fn dirname(path: &str) -> &str {
    path.rfind('/').map_or(".", |i| &path[..i])
}

#[derive(Clone, PartialEq)]
enum Connection {
    Connecting,
    Failed(String),
    Live(OpenedProjectData),
}

// A read-only rider on hexend's live session: it only ever listens —
// so it renders whatever the editor (or anyone else connected to the
// same project) does, live, fog of war included, with no separate
// "read-only mode" to keep in sync elsewhere. Fog is applied/removed
// from the editor's own GM mode (see apps/editor), not here — this
// window just shows the result, same as it would any other command.
#[component]
pub fn App() -> impl IntoView {
    let Some(target) = target_from_url() else {
        return view! {
            <div class="status">
                "Add " <code>"?server=http://localhost:4000&path=/abs/project.hexen.yml"</code> " to the URL."
            </div>
        }
        .into_any();
    };

    let connection = RwSignal::new(Connection::Connecting);
    let session = live_session::connect(
        &target.server,
        &target.path,
        move |data| connection.set(Connection::Live(data)),
        move |err| connection.set(Connection::Failed(err)),
    );
    let session: Option<LiveSession> = match session {
        Ok(session) => Some(session),
        Err(err) => {
            connection.set(Connection::Failed(err));
            None
        }
    };
    let session = StoredValue::new_local(session);
    on_cleanup(move || session.update_value(|s| drop(s.take())));

    let data = Memo::new(move |_| match connection.get() {
        Connection::Live(data) => Some(data),
        _ => None,
    });

    let current_location_id = RwSignal::new(None::<String>);
    let selected_link_id = RwSignal::new(None::<String>);

    // Start on the project's default location once it's known.
    Effect::new(move |_| {
        if let Some(project) = data.with(|d| d.as_ref().and_then(|d| d.project.clone())) {
            if current_location_id.get_untracked().is_none() {
                current_location_id.set(Some(project.default_location));
            }
        }
    });

    (move || match connection.get() {
        Connection::Connecting => view! { <div class="status">"Connecting…"</div> }.into_any(),
        Connection::Failed(err) => view! { <div class="status error">{err}</div> }.into_any(),
        Connection::Live(_) => {
            view! { <Presentation target=target.clone() data current_location_id selected_link_id /> }.into_any()
        }
    })
    .into_any()
}

#[component]
fn Presentation(
    target: Target,
    data: Memo<Option<OpenedProjectData>>,
    current_location_id: RwSignal<Option<String>>,
    selected_link_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let project = Memo::new(move |_| data.get().and_then(|d| d.project).unwrap_or_default());
    let current_location = Memo::new(move |_| {
        let id = current_location_id.get()?;
        project.with(|p| p.locations.iter().find(|l| l.id == id).cloned())
    });
    let title_of = move |id: &str| -> String {
        data.with(|d| {
            d.as_ref()
                .and_then(|d| d.resolved_content.get(id))
                .map_or_else(|| id.to_owned(), |c| c.title.clone())
        })
    };
    let link_titles = Memo::new(move |_| {
        current_location.with(|loc| {
            loc.iter()
                .flat_map(|loc| &loc.links)
                .map(|link| (link.target.clone(), title_of(&link.target)))
                .collect::<HashMap<_, _>>()
        })
    });

    let go_to_link = Callback::new(move |link_id: String| {
        selected_link_id.set(Some(link_id.clone()));
        let target_id = current_location.with_untracked(|loc| {
            loc.as_ref()?
                .links
                .iter()
                .find(|l| l.id == link_id)
                .map(|l| l.target.clone())
        });
        let Some(target_id) = target_id else { return };
        let has_image = project.with_untracked(|p| {
            p.locations
                .iter()
                .any(|l| l.id == target_id && l.image.is_some())
        });
        if has_image {
            current_location_id.set(Some(target_id));
        }
    });

    move || {
        let Some(location) = current_location.get() else {
            let id = current_location_id.get().unwrap_or_default();
            return view! { <div class="status error">"Unknown location \"" {id} "\""</div> }
                .into_any();
        };
        let (default_location, project_title) =
            project.with(|p| (p.default_location.clone(), p.title.clone()));
        let Location {
            id: location_id, ..
        } = &location;
        let location_title = title_of(location_id);
        let back = (*location_id != default_location).then(|| {
            view! {
                <button
                    type="button"
                    class="link-button"
                    on:click=move |_| current_location_id.set(Some(default_location.clone()))
                >
                    "← "
                    {project_title}
                </button>
            }
        });

        let map = match &location.image {
            Some(image) => {
                let image_url = format!(
                    "{}/image?dir={}&file={}",
                    target.server,
                    js_sys::encode_uri_component(dirname(&target.path)),
                    js_sys::encode_uri_component(&image.file),
                );
                let image = Memo::new(move |_| current_location.get().and_then(|l| l.image).unwrap_or_default());
                let grid = Memo::new(move |_| current_location.get().and_then(|l| l.grid));
                let links = Memo::new(move |_| current_location.get().map(|l| l.links).unwrap_or_default());
                let fog = Memo::new(move |_| current_location.get().and_then(|l| l.fog));
                view! {
                    <MapCanvas
                        image
                        image_url=Signal::derive(move || image_url.clone())
                        grid
                        links
                        link_titles
                        selected_link_id
                        on_select_link=go_to_link
                        fog
                    />
                }
                .into_any()
            }
            None => {
                view! { <div class="status">"\"" {location_id.clone()} "\" has no image — nothing to render."</div> }
                    .into_any()
            }
        };

        let visible_links: Vec<_> = location
            .links
            .iter()
            .filter(|l| !l.hidden.unwrap_or(false))
            .cloned()
            .collect();
        let nav = (!visible_links.is_empty()).then(|| {
            let titles = link_titles.get();
            view! {
                <nav class="presentation-links">
                    <ul>
                        {visible_links
                            .into_iter()
                            .map(|link| {
                                let icon_svg = find_link_icon(link.icon.as_deref()).map(|icon| icon.svg);
                                let title = titles.get(&link.target).cloned().unwrap_or_else(|| link.target.clone());
                                let id = link.id.clone();
                                let is_selected = move || selected_link_id.get().as_deref() == Some(id.as_str());
                                let id = link.id.clone();
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            class=move || if is_selected() { "selected" } else { "" }
                                            on:click=move |_| go_to_link.run(id.clone())
                                        >
                                            {icon_svg.map(|svg| view! { <span class="icon-swatch" inner_html=svg></span> })}
                                            {title}
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                </nav>
            }
        });

        view! {
            <div class="presentation">
                <header class="presentation-header">
                    <h2>{location_title}</h2>
                    {back}
                </header>
                <main class="presentation-map">{map}</main>
                {nav}
            </div>
        }
        .into_any()
    }
}
