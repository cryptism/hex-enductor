use std::collections::{HashMap, HashSet};

use hexen_proto::hexen::v1::{HexenProject, ResolvedContent};
use leptos::prelude::*;

#[derive(Clone, PartialEq)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub has_image: bool,
    pub is_orphan: bool,
}

/// Every Location, filtered and sorted by title. An orphan is anything
/// no Link points at, other than the project's defaultLocation (which
/// is reachable by definition).
pub fn rows(
    project: &HexenProject,
    resolved: &HashMap<String, ResolvedContent>,
    query: &str,
    maps_only: bool,
    orphans_only: bool,
) -> Vec<Row> {
    let linked: HashSet<&str> = project
        .locations
        .iter()
        .flat_map(|l| &l.links)
        .map(|link| link.target.as_str())
        .collect();
    let q = query.trim().to_lowercase();
    let mut rows: Vec<Row> = project
        .locations
        .iter()
        .map(|l| Row {
            id: l.id.clone(),
            title: resolved
                .get(&l.id)
                .map_or_else(|| l.id.clone(), |c| c.title.clone()),
            has_image: l.image.is_some(),
            is_orphan: l.id != project.default_location && !linked.contains(l.id.as_str()),
        })
        .filter(|r| !maps_only || r.has_image)
        .filter(|r| !orphans_only || r.is_orphan)
        .filter(|r| {
            q.is_empty() || r.id.to_lowercase().contains(&q) || r.title.to_lowercase().contains(&q)
        })
        .collect();
    rows.sort_by(|a, b| a.title.cmp(&b.title));
    rows
}

/// The pin-by-pin hierarchy is the normal way through a project, but
/// there's no way to jump straight to "that one shop three maps deep"
/// without following it — this is every Location, flat, filterable.
/// "Maps only" and "Orphans only" narrow it to exactly what a GM wants
/// when staging new maps ahead of linking them in.
#[component]
pub fn LocationBrowser(
    #[prop(into)] project: Signal<HexenProject>,
    #[prop(into)] resolved_content: Signal<HashMap<String, ResolvedContent>>,
    #[prop(into)] current_location_id: Signal<Option<String>>,
    on_select: Callback<String>,
    on_close: Callback<()>,
    /// Gates "New map…" — creating a Location is a mutation like any other.
    #[prop(into)]
    edit_mode: Signal<bool>,
    on_create: Callback<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let maps_only = RwSignal::new(false);
    let orphans_only = RwSignal::new(false);
    let creating = RwSignal::new(false);
    let new_id = RwSignal::new(String::new());
    let create_error = RwSignal::new(None::<String>);

    let visible = move || {
        project.with(|p| {
            resolved_content.with(|r| rows(p, r, &query.get(), maps_only.get(), orphans_only.get()))
        })
    };
    let pick = move |id: String| {
        on_select.run(id);
        on_close.run(());
    };

    view! {
        <div class="location-browser">
            <button type="button" class="link-button picker-close" on:click=move |_| on_close.run(())>
                "Cancel"
            </button>
            <h3>"All locations (" {move || project.with(|p| p.locations.len())} ")"</h3>
            <input type="text" placeholder="Search by name or id…" autofocus bind:value=query />

            <div class="browser-filters">
                <label>
                    <input type="checkbox" bind:checked=maps_only />
                    "Maps only (has an image)"
                </label>
                <label>
                    <input type="checkbox" bind:checked=orphans_only />
                    "Orphans only (not linked from anywhere)"
                </label>
            </div>

            <div class="browser">
                <ul class="link-list">
                    {move || {
                        let rows = visible();
                        let empty = rows.is_empty();
                        view! {
                            {rows
                                .into_iter()
                                .map(|row| {
                                    let selected = current_location_id.get().as_deref() == Some(row.id.as_str());
                                    let id = row.id.clone();
                                    view! {
                                        <li>
                                            <button class=if selected { "selected" } else { "" } on:click=move |_| pick(id.clone())>
                                                {row.title}
                                                {row.is_orphan.then_some(" (orphan)")}
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()}
                            {empty.then(|| view! { <li class="muted">"No locations match."</li> })}
                        }
                    }}
                </ul>
            </div>

            <Show when=move || edit_mode.get()>
                {move || {
                    if creating.get() {
                        view! {
                            <form
                                class="link-form"
                                on:submit=move |e| {
                                    e.prevent_default();
                                    let id = new_id.get().trim().to_string();
                                    if id.is_empty() {
                                        return;
                                    }
                                    if project.with(|p| p.locations.iter().any(|l| l.id == id)) {
                                        create_error.set(Some(format!("\"{id}\" already exists.")));
                                        return;
                                    }
                                    on_create.run(id.clone());
                                    pick(id);
                                }
                            >
                                <label>
                                    "New location id"
                                    <input
                                        type="text"
                                        placeholder="e.g. old-mill-cellar"
                                        autofocus
                                        prop:value=move || new_id.get()
                                        on:input=move |ev| {
                                            new_id.set(event_target_value(&ev));
                                            create_error.set(None);
                                        }
                                    />
                                </label>
                                {move || create_error.get().map(|e| view! { <span class="field-error">{e}</span> })}
                                <div class="form-actions">
                                    <button type="submit" disabled=move || new_id.get().trim().is_empty()>
                                        "Create"
                                    </button>
                                    <button type="button" class="link-button" on:click=move |_| creating.set(false)>
                                        "Cancel"
                                    </button>
                                </div>
                            </form>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button type="button" class="link-button" on:click=move |_| creating.set(true)>
                                "New map…"
                            </button>
                        }
                            .into_any()
                    }
                }}
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use hexen_proto::hexen::v1::{ImageRef, Link, Location};

    use super::*;

    fn project() -> HexenProject {
        let loc = |id: &str, image: bool, links: &[&str]| Location {
            id: id.into(),
            image: image.then(ImageRef::default),
            links: links
                .iter()
                .map(|t| Link {
                    target: (*t).into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        HexenProject {
            default_location: "town".into(),
            locations: vec![
                loc("town", true, &["inn"]),
                loc("inn", false, &[]),
                loc("cellar", true, &[]),
            ],
            ..Default::default()
        }
    }

    fn ids(rows: Vec<Row>) -> Vec<String> {
        rows.into_iter().map(|r| r.id).collect()
    }

    #[test]
    fn sorts_by_title_falling_back_to_id() {
        let mut resolved = HashMap::new();
        resolved.insert(
            "town".to_string(),
            ResolvedContent {
                title: "A Town".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            ids(rows(&project(), &resolved, "", false, false)),
            ["town", "cellar", "inn"]
        );
    }

    #[test]
    fn filters_maps_orphans_and_text() {
        let none = HashMap::new();
        assert_eq!(
            ids(rows(&project(), &none, "", true, false)),
            ["cellar", "town"]
        );
        // the default location is never an orphan
        assert_eq!(ids(rows(&project(), &none, "", false, true)), ["cellar"]);
        assert_eq!(
            ids(rows(&project(), &none, "CELL", false, false)),
            ["cellar"]
        );
    }
}
