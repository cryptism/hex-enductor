use std::collections::HashMap;
use std::time::Duration;

use hexen_proto::hexen::v1::OpenedProjectData;
use hexen_web::live_session::{self, Event, LiveSession};
use hexen_web::map_canvas::{MapCanvas, MapView, PingMark, PING_EFFECT_DURATION_MS};
use leptos::prelude::*;
use map_core::link_icons::find_link_icon;

use crate::recents::{add_recent_target, recent_targets, Target};

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

fn encode(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}

#[derive(Clone, PartialEq)]
enum Connection {
    Connecting,
    Failed(String),
    Live,
}

/// A projector/second-monitor display is the whole point of this app —
/// "f" toggles fullscreen without hunting for the browser's own button,
/// the same key most video players use.
fn toggle_fullscreen_on_f() {
    let handle = window_event_listener(leptos::ev::keydown, |e| {
        if !e.key().eq_ignore_ascii_case("f")
            || e.repeat()
            || e.meta_key()
            || e.ctrl_key()
            || e.alt_key()
        {
            return;
        }
        let document = document();
        if document.fullscreen_element().is_some() {
            document.exit_fullscreen();
        } else if let Some(root) = document.document_element() {
            let _ = root.request_fullscreen();
        }
    });
    on_cleanup(move || handle.remove());
}

#[component]
fn Landing() -> impl IntoView {
    let recent = recent_targets();
    view! {
        <div class="status landing">
            <p>"Add " <code>"?server=http://localhost:4000&path=/abs/project.hexen.yml"</code> " to the URL."</p>
            {(!recent.is_empty())
                .then(|| {
                    view! {
                        <div class="recent-targets">
                            <h3>"Recently opened"</h3>
                            <ul>
                                {recent
                                    .into_iter()
                                    .map(|t| {
                                        let href = format!("?server={}&path={}", encode(&t.server), encode(&t.path));
                                        view! {
                                            <li>
                                                <a href=href>
                                                    {t.path}
                                                    <span class="recent-target-server">" — " {t.server}</span>
                                                </a>
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

// A read-only rider on hexend's live session: it only ever listens — so
// it renders whatever the editor (or anyone else connected to the same
// project) does, live, fog of war included. Pings and Follow mode both
// have a side effect here: one for a different location switches this
// window to it, since neither makes sense unless everyone's looking at
// the same map.
#[component]
pub fn App() -> impl IntoView {
    toggle_fullscreen_on_f();
    let Some(target) = target_from_url() else {
        return view! { <Landing /> }.into_any();
    };

    let connection = RwSignal::new(Connection::Connecting);
    let data = RwSignal::new(None::<OpenedProjectData>);
    let current_location_id = RwSignal::new(None::<String>);
    let ping_at = RwSignal::new(None::<PingMark>);
    let ping_key = StoredValue::new(0u64);
    // The last view broadcast while Follow mode was on — kept once
    // updates stop (Follow switched off at the GM's end): the map only
    // re-applies it when it *changes*, so this window is free to pan and
    // zoom on its own from then on.
    let follow_view = RwSignal::new(None::<MapView>);

    let on_event = {
        let target = target.clone();
        move |event| match event {
            Event::State(state) => {
                if connection.get_untracked() != Connection::Live {
                    add_recent_target(&target);
                    connection.set(Connection::Live);
                }
                data.set(Some(state));
            }
            Event::Ping(ping) => {
                current_location_id.set(Some(ping.location_id));
                ping_key.update_value(|k| *k += 1);
                let key = ping_key.get_value();
                ping_at.set(Some(PingMark {
                    x: ping.x,
                    y: ping.y,
                    key,
                }));
                set_timeout(
                    move || {
                        if ping_at
                            .try_get_untracked()
                            .flatten()
                            .is_some_and(|p| p.key == key)
                        {
                            ping_at.set(None);
                        }
                    },
                    Duration::from_millis(PING_EFFECT_DURATION_MS),
                );
            }
            Event::FollowView(view) => {
                current_location_id.set(Some(view.location_id));
                follow_view.set(Some(MapView {
                    x: view.x,
                    y: view.y,
                    zoom: view.zoom,
                }));
            }
        }
    };
    let session = live_session::connect(&target.server, &target.path, on_event, move |err| {
        connection.set(Connection::Failed(err))
    });
    let session: Option<LiveSession> = match session {
        Ok(session) => Some(session),
        Err(err) => {
            connection.set(Connection::Failed(err));
            None
        }
    };
    let session = StoredValue::new_local(session);
    on_cleanup(move || session.update_value(|s| drop(s.take())));

    // Start on the project's default location once it's known.
    Effect::new(move |_| {
        if let Some(default) = data.with(|d| {
            d.as_ref()?
                .project
                .as_ref()
                .map(|p| p.default_location.clone())
        }) {
            if current_location_id.get_untracked().is_none() {
                current_location_id.set(Some(default));
            }
        }
    });

    let is_live = Memo::new(move |_| connection.get() == Connection::Live);
    let status = move || match connection.get() {
        Connection::Connecting => {
            Some(view! { <div class="status">"Connecting…"</div> }.into_any())
        }
        Connection::Failed(err) => Some(view! { <div class="status error">{err}</div> }.into_any()),
        Connection::Live => None,
    };
    view! {
        {status}
        <Show when=move || is_live.get()>
            <Presentation target=target.clone() data current_location_id ping_at follow_view />
        </Show>
    }
    .into_any()
}

/// Built once per connection and updated in place: the map stays mounted
/// across every state update and location change, so a GM's fog strokes
/// don't reset a player's pan and zoom.
#[component]
fn Presentation(
    target: Target,
    data: RwSignal<Option<OpenedProjectData>>,
    current_location_id: RwSignal<Option<String>>,
    ping_at: RwSignal<Option<PingMark>>,
    follow_view: RwSignal<Option<MapView>>,
) -> impl IntoView {
    let selected_link_id = RwSignal::new(None::<String>);
    let project = Memo::new(move |_| {
        data.with(|d| d.as_ref().and_then(|d| d.project.clone()))
            .unwrap_or_default()
    });
    let location = Memo::new(move |_| {
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
    let location_id =
        Memo::new(move |_| location.with(|l| l.as_ref().map(|l| l.id.clone()).unwrap_or_default()));
    let known = Memo::new(move |_| location.with(Option::is_some));
    let image = Memo::new(move |_| location.with(|l| l.as_ref().and_then(|l| l.image.clone())));
    let has_image = Memo::new(move |_| image.with(Option::is_some));
    let links = Memo::new(move |_| {
        location.with(|l| l.as_ref().map(|l| l.links.clone()).unwrap_or_default())
    });
    let link_titles = Memo::new(move |_| {
        links.with(|links| {
            links
                .iter()
                .map(|l| (l.target.clone(), title_of(&l.target)))
                .collect::<HashMap<_, _>>()
        })
    });

    let go_to_link = Callback::new(move |link_id: String| {
        selected_link_id.set(Some(link_id.clone()));
        let Some(target_id) = links.with_untracked(|ls| {
            ls.iter()
                .find(|l| l.id == link_id)
                .map(|l| l.target.clone())
        }) else {
            return;
        };
        let has_image = project.with_untracked(|p| {
            p.locations
                .iter()
                .any(|l| l.id == target_id && l.image.is_some())
        });
        if has_image {
            current_location_id.set(Some(target_id));
        }
    });

    let back = move || {
        let (default, title) = project.with(|p| (p.default_location.clone(), p.title.clone()));
        (location_id.get() != default).then(|| {
            view! {
                <button type="button" class="link-button" on:click=move |_| current_location_id.set(Some(default.clone()))>
                    "← "
                    {title}
                </button>
            }
        })
    };

    let image_url = {
        let target = target.clone();
        Signal::derive(move || {
            image.with(|i| {
                i.as_ref().map_or_else(String::new, |i| {
                    format!(
                        "{}/image?dir={}&file={}",
                        target.server,
                        encode(dirname(&target.path)),
                        encode(&i.file)
                    )
                })
            })
        })
    };

    let nav = move || {
        let visible: Vec<_> = links.with(|ls| {
            ls.iter()
                .filter(|l| !l.hidden.unwrap_or(false))
                .cloned()
                .collect()
        });
        (!visible.is_empty()).then(|| {
            let titles = link_titles.get();
            view! {
                <nav class="presentation-links">
                    <ul>
                        {visible
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
        })
    };

    view! {
        <Show
            when=move || known.get()
            fallback=move || {
                let id = current_location_id.get().unwrap_or_default();
                view! { <div class="status error">"Unknown location \"" {id} "\""</div> }
            }
        >
            <div class="presentation">
                <header class="presentation-header">
                    <h2>{move || title_of(&location_id.get())}</h2>
                    {back}
                </header>
                <main class="presentation-map">
                    <Show
                        when=move || has_image.get()
                        fallback=move || {
                            view! {
                                <div class="status">"\"" {location_id} "\" has no image — nothing to render."</div>
                            }
                        }
                    >
                        <MapCanvas
                            image=Signal::derive(move || image.get().unwrap_or_default())
                            image_url
                            grid=Signal::derive(move || location.with(|l| l.as_ref().and_then(|l| l.grid.clone())))
                            links
                            link_titles
                            selected_link_id
                            on_select_link=go_to_link
                            fog=Signal::derive(move || location.with(|l| l.as_ref().and_then(|l| l.fog.clone())))
                            ping_at
                            follow_view
                        />
                    </Show>
                </main>
                {nav}
            </div>
        </Show>
    }
}
