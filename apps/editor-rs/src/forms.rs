//! The editor's side-panel forms. Each keeps a draft of its fields and
//! is "dirty" (Save enabled) only while the draft differs from what the
//! project currently holds — when the project updates underneath (a
//! save landing, another client's edit) the draft resets to match.

use hexen_proto::hexen::v1::{
    grid, Grid, GridStyle, HexGrid, ImageRef, InlineContentPatch, Link, LinkPatch, Point,
    SquareGrid,
};
use leptos::prelude::*;
use map_core::link_icons::{find_link_icon, IconCategory, LINK_ICONS};

fn icon_swatch(slug: Option<&str>) -> Option<impl IntoView> {
    find_link_icon(slug).map(|icon| view! { <span class="icon-swatch" inner_html=icon.svg></span> })
}

const CATEGORIES: [(IconCategory, &str); 5] = [
    (IconCategory::Settlement, "Settlement"),
    (IconCategory::Landmark, "Landmark"),
    (IconCategory::Ruin, "Ruin"),
    (IconCategory::Hazard, "Hazard"),
    (IconCategory::Waypoint, "Waypoint"),
];

#[component]
pub fn IconPicker(
    #[prop(into)] value: Signal<Option<String>>,
    on_change: Callback<Option<String>>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());

    let groups = move || {
        let q = query.get().trim().to_lowercase();
        CATEGORIES
            .iter()
            .filter_map(|(category, label)| {
                let icons: Vec<_> = LINK_ICONS
                    .iter()
                    .filter(|icon| icon.category == *category)
                    .filter(|icon| {
                        q.is_empty()
                            || icon.label.to_lowercase().contains(&q)
                            || icon.slug.contains(&q)
                    })
                    .collect();
                (!icons.is_empty()).then_some((*label, icons))
            })
            .collect::<Vec<_>>()
    };
    // The picker sits inside a <label> in LinkForm. Picking removes the
    // clicked button from the DOM mid-event, after which the label no
    // longer counts it as its own interactive child and would forward
    // the click to the toggle (reopening the panel) — so every button
    // here cancels the click's default action.
    let choose = move |e: leptos::ev::MouseEvent, slug: Option<String>| {
        e.prevent_default();
        on_change.run(slug);
        open.set(false);
    };

    view! {
        <div class="icon-picker">
            <button type="button" class="icon-picker-toggle" on:click=move |e| {
                e.prevent_default();
                open.update(|o| *o = !*o);
            }>
                {move || match find_link_icon(value.get().as_deref()) {
                    Some(icon) => {
                        view! {
                            <span class="icon-swatch" inner_html=icon.svg></span>
                            {icon.label}
                        }
                            .into_any()
                    }
                    None => "(none)".into_any(),
                }}
                <span class="icon-picker-caret">{move || if open.get() { "▲" } else { "▼" }}</span>
            </button>

            <Show when=move || open.get()>
                <div class="icon-picker-panel">
                    <input type="text" placeholder="Search icons…" autofocus bind:value=query />
                    <button type="button" class="icon-picker-none" on:click=move |e| choose(e, None)>
                        "None"
                    </button>
                    {move || {
                        let groups = groups();
                        let empty = groups.is_empty();
                        view! {
                            {groups
                                .into_iter()
                                .map(|(label, icons)| {
                                    view! {
                                        <div class="icon-picker-group">
                                            <h4>{label}</h4>
                                            <div class="icon-picker-grid">
                                                {icons
                                                    .into_iter()
                                                    .map(|icon| {
                                                        let selected = move || value.get().as_deref() == Some(icon.slug);
                                                        view! {
                                                            <button
                                                                type="button"
                                                                title=icon.label
                                                                class=move || {
                                                                    if selected() { "icon-picker-item selected" } else { "icon-picker-item" }
                                                                }
                                                                on:click=move |e| choose(e, Some(icon.slug.to_string()))
                                                                inner_html=icon.svg
                                                            ></button>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        </div>
                                    }
                                })
                                .collect_view()}
                            {empty.then(|| view! { <p class="muted">"No icons match \"" {query.get()} "\"."</p> })}
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

#[derive(Clone, PartialEq)]
struct LinkDraft {
    x: String,
    y: String,
    r#type: String,
    icon: Option<String>,
    color: String,
    hidden: bool,
}

impl LinkDraft {
    fn of(link: &Link) -> Self {
        LinkDraft {
            x: link.x.to_string(),
            y: link.y.to_string(),
            r#type: link.r#type.clone(),
            icon: link.icon.clone(),
            color: link.color.clone().unwrap_or_default(),
            hidden: link.hidden.unwrap_or(false),
        }
    }
}

#[component]
pub fn LinkForm(
    #[prop(into)] link: Signal<Link>,
    #[prop(into)] title: Signal<String>,
    #[prop(into)] saving: Signal<bool>,
    on_save: Callback<LinkPatch>,
) -> impl IntoView {
    let draft = RwSignal::new(LinkDraft::of(&link.get_untracked()));
    // A save landing (or anyone else's edit) resets the form to the
    // project's values — the same thing react-hook-form's `values` did.
    Effect::new(move |_| draft.set(link.with(LinkDraft::of)));

    let dirty = move || link.with(LinkDraft::of) != draft.get();
    let type_missing = move || draft.with(|d| d.r#type.trim().is_empty());
    let parsed = move || {
        draft.with(|d| {
            let x = d.x.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
            let y = d.y.trim().parse::<f64>().ok().filter(|v| v.is_finite())?;
            (!d.r#type.trim().is_empty()).then(|| LinkPatch {
                x: Some(x),
                y: Some(y),
                r#type: Some(d.r#type.clone()),
                // "" clears — see LinkPatch in schema/hexen/v1/command.proto.
                icon: Some(d.icon.clone().unwrap_or_default()),
                color: Some(d.color.trim().to_string()),
                hidden: Some(d.hidden),
                ..Default::default()
            })
        })
    };
    let field = move |get: fn(&LinkDraft) -> String, set: fn(&mut LinkDraft, String)| {
        (
            move || draft.with(get),
            move |ev| draft.update(|d| set(d, event_target_value(&ev))),
        )
    };
    let (x_value, x_input) = field(|d| d.x.clone(), |d, v| d.x = v);
    let (y_value, y_input) = field(|d| d.y.clone(), |d, v| d.y = v);
    let (type_value, type_input) = field(|d| d.r#type.clone(), |d, v| d.r#type = v);
    let (color_value, color_input) = field(|d| d.color.clone(), |d, v| d.color = v);

    view! {
        <form
            class="link-form"
            on:submit=move |e| {
                e.prevent_default();
                if let Some(patch) = parsed() {
                    on_save.run(patch);
                }
            }
        >
            <h3 class="link-form-title">
                {move || draft.with(|d| icon_swatch(d.icon.as_deref()))}
                {title}
            </h3>
            <p class="link-form-id">"→ " {move || link.with(|l| l.target.clone())}</p>

            <label>"X" <input type="number" step="any" prop:value=x_value on:input=x_input /></label>
            <label>"Y" <input type="number" step="any" prop:value=y_value on:input=y_input /></label>
            <label>
                "Type" <input type="text" prop:value=type_value on:input=type_input />
                {move || type_missing().then(|| view! { <span class="field-error">"required"</span> })}
            </label>
            <label>
                "Icon"
                <IconPicker
                    value=Signal::derive(move || draft.with(|d| d.icon.clone()))
                    on_change=Callback::new(move |slug| draft.update(|d| d.icon = slug))
                />
            </label>
            <label>
                "Color" <input type="text" placeholder="(default)" prop:value=color_value on:input=color_input />
            </label>
            <label class="checkbox-label">
                <input
                    type="checkbox"
                    prop:checked=move || draft.with(|d| d.hidden)
                    on:change=move |ev| draft.update(|d| d.hidden = event_target_checked(&ev))
                />
                "Hidden"
            </label>

            <button type="submit" disabled=move || !dirty() || parsed().is_none() || saving.get()>
                {move || if saving.get() { "Saving…" } else { "Save" }}
            </button>
        </form>
    }
}

/// A location's own title/body — editable here only because its content
/// is (or will become) inline. Obsidian-backed locations are read-only
/// in this app; that prose lives in the vault.
#[component]
pub fn LocationContentForm(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] body: Signal<String>,
    #[prop(into)] saving: Signal<bool>,
    on_save: Callback<InlineContentPatch>,
) -> impl IntoView {
    let draft_title = RwSignal::new(title.get_untracked());
    let draft_body = RwSignal::new(body.get_untracked());
    Effect::new(move |_| draft_title.set(title.get()));
    Effect::new(move |_| draft_body.set(body.get()));
    let dirty = move || draft_title.get() != title.get() || draft_body.get() != body.get();

    view! {
        <form
            class="link-form location-content-form"
            on:submit=move |e| {
                e.prevent_default();
                on_save.run(InlineContentPatch { title: Some(draft_title.get()), body: Some(draft_body.get()) });
            }
        >
            <label>"Title" <input type="text" placeholder="Untitled" bind:value=draft_title /></label>
            <label>
                "Body" <textarea rows="6" placeholder="Write something about this place…" bind:value=draft_body></textarea>
            </label>
            <button type="submit" disabled=move || !dirty() || saving.get()>
                {move || if saving.get() { "Saving…" } else { "Save" }}
            </button>
        </form>
    }
}

/// Deliberately just two fields — a click already fixed x/y, and
/// icon/color/hidden all have sensible defaults editable afterward via
/// the ordinary LinkForm once the pin exists.
#[component]
pub fn AddLocationForm(
    #[prop(into)] saving: Signal<bool>,
    on_save: Callback<(String, String)>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let location_id = RwSignal::new(String::new());
    let link_type = RwSignal::new(String::new());
    let can_save =
        move || !location_id.get().trim().is_empty() && !link_type.get().trim().is_empty();

    view! {
        <form
            class="link-form"
            on:submit=move |e| {
                e.prevent_default();
                if can_save() {
                    on_save.run((location_id.get().trim().into(), link_type.get().trim().into()));
                }
            }
        >
            <h3>"New location"</h3>
            <label>
                "Location id" <input type="text" placeholder="e.g. old-mill" autofocus bind:value=location_id />
            </label>
            <label>
                "Type" <input type="text" placeholder="e.g. npc, trap, chest, landmark…" bind:value=link_type />
            </label>
            <div class="form-actions">
                <button type="submit" disabled=move || !can_save() || saving.get()>
                    {move || if saving.get() { "Adding…" } else { "Add" }}
                </button>
                <button type="button" class="link-button" on:click=move |_| on_cancel.run(())>
                    "Cancel"
                </button>
            </div>
        </form>
    }
}

fn default_style() -> GridStyle {
    GridStyle {
        color: "#c19a5f".into(),
        weight: Some(1.0),
        opacity: Some(0.45),
    }
}

fn pt(x: f64, y: f64) -> Option<Point> {
    Some(Point { x, y })
}

fn default_grid_for(image: &ImageRef) -> Grid {
    Grid {
        kind: Some(grid::Kind::Hex(HexGrid {
            origin: pt(
                (image.width as f64 / 2.0).round(),
                (image.height as f64 / 2.0).round(),
            ),
            b1: pt(60.0, 0.0),
            b2: pt(30.0, 52.0),
            distance_per_cell: None,
            style: Some(default_style()),
        })),
    }
}

/// The parts every grid kind shares, whichever it is.
fn common(g: &Grid) -> (Point, Option<f64>, GridStyle) {
    match &g.kind {
        Some(grid::Kind::Hex(h)) => (
            h.origin.unwrap_or_default(),
            h.distance_per_cell,
            h.style.clone().unwrap_or_else(default_style),
        ),
        Some(grid::Kind::Square(s)) => (
            s.origin.unwrap_or_default(),
            s.distance_per_cell,
            s.style.clone().unwrap_or_else(default_style),
        ),
        None => (Point::default(), None, default_style()),
    }
}

fn with_common(
    g: &mut Grid,
    f: impl FnOnce(&mut Option<Point>, &mut Option<f64>, &mut Option<GridStyle>),
) {
    match &mut g.kind {
        Some(grid::Kind::Hex(h)) => f(&mut h.origin, &mut h.distance_per_cell, &mut h.style),
        Some(grid::Kind::Square(s)) => f(&mut s.origin, &mut s.distance_per_cell, &mut s.style),
        None => {}
    }
}

fn number(ev: &leptos::ev::Event) -> f64 {
    // Number("") is 0 in the TS form this mirrors; keep that.
    event_target_value(ev).trim().parse().unwrap_or(0.0)
}

/// A document property, not a tool — this is the one thing in the
/// toolbox that gets a Cancel button, because getting the lattice wrong
/// is only obvious by looking at it against the real image. Every edit
/// goes straight to `on_preview` so the map shows it live.
#[component]
pub fn ConfigureGridForm(
    initial_grid: Option<Grid>,
    image: ImageRef,
    on_preview: Callback<Grid>,
    on_apply: Callback<Grid>,
    on_cancel: Callback<()>,
    #[prop(into)] saving: Signal<bool>,
) -> impl IntoView {
    let grid = RwSignal::new(
        initial_grid
            .filter(|g| g.kind.is_some())
            .unwrap_or_else(|| default_grid_for(&image)),
    );
    // Push the starting value (possibly the just-computed default) into
    // the live preview immediately, rather than waiting on a first edit.
    on_preview.run(grid.get_untracked());
    let set = move |f: &dyn Fn(&mut Grid)| {
        grid.update(|g| f(g));
        on_preview.run(grid.get_untracked());
    };

    let is_hex = move || grid.with(|g| matches!(g.kind, Some(grid::Kind::Hex(_))));
    let point_input =
        move |label: &'static str, read: fn(&Grid) -> f64, write: fn(&mut Grid, f64)| {
            view! {
                <label>
                    {label}
                    <input
                        type="number"
                        prop:value=move || grid.with(read).to_string()
                        on:input=move |ev| {
                            let v = number(&ev);
                            set(&|g| write(g, v));
                        }
                    />
                </label>
            }
        };
    fn hex(g: &Grid) -> Option<&HexGrid> {
        match &g.kind {
            Some(grid::Kind::Hex(h)) => Some(h),
            _ => None,
        }
    }
    fn hex_mut(g: &mut Grid) -> Option<&mut HexGrid> {
        match &mut g.kind {
            Some(grid::Kind::Hex(h)) => Some(h),
            _ => None,
        }
    }
    fn square(g: &Grid) -> Option<&SquareGrid> {
        match &g.kind {
            Some(grid::Kind::Square(s)) => Some(s),
            _ => None,
        }
    }
    fn square_mut(g: &mut Grid) -> Option<&mut SquareGrid> {
        match &mut g.kind {
            Some(grid::Kind::Square(s)) => Some(s),
            _ => None,
        }
    }
    fn get(p: Option<Point>) -> Point {
        p.unwrap_or_default()
    }

    view! {
        <form
            class="link-form configure-grid"
            on:submit=move |e| {
                e.prevent_default();
                on_apply.run(grid.get());
            }
        >
            <h3>"Configure grid"</h3>

            <label>
                "Type"
                <select
                    prop:value=move || if is_hex() { "hex" } else { "square" }
                    on:change=move |ev| {
                        let want_hex = event_target_value(&ev) == "hex";
                        if want_hex == is_hex() {
                            return;
                        }
                        set(&|g| {
                            let (origin, distance_per_cell, style) = common(g);
                            g.kind = Some(if want_hex {
                                grid::Kind::Hex(HexGrid {
                                    origin: Some(origin),
                                    b1: pt(60.0, 0.0),
                                    b2: pt(30.0, 52.0),
                                    distance_per_cell,
                                    style: Some(style),
                                })
                            } else {
                                grid::Kind::Square(SquareGrid {
                                    origin: Some(origin),
                                    cell_size: pt(64.0, 64.0),
                                    distance_per_cell,
                                    style: Some(style),
                                })
                            });
                        });
                    }
                >
                    <option value="hex">"Hex"</option>
                    <option value="square">"Square"</option>
                </select>
            </label>

            <fieldset>
                <legend>"Origin"</legend>
                {point_input("X", |g| common(g).0.x, |g, v| with_common(g, |o, _, _| o.get_or_insert_default().x = v))}
                {point_input("Y", |g| common(g).0.y, |g, v| with_common(g, |o, _, _| o.get_or_insert_default().y = v))}
            </fieldset>

            {move || {
                if is_hex() {
                    view! {
                        <fieldset>
                            <legend>"Basis 1 (to a neighbouring hex)"</legend>
                            {point_input("X", |g| get(hex(g).and_then(|h| h.b1)).x, |g, v| {
                                if let Some(h) = hex_mut(g) { h.b1.get_or_insert_default().x = v }
                            })}
                            {point_input("Y", |g| get(hex(g).and_then(|h| h.b1)).y, |g, v| {
                                if let Some(h) = hex_mut(g) { h.b1.get_or_insert_default().y = v }
                            })}
                        </fieldset>
                        <fieldset>
                            <legend>"Basis 2"</legend>
                            {point_input("X", |g| get(hex(g).and_then(|h| h.b2)).x, |g, v| {
                                if let Some(h) = hex_mut(g) { h.b2.get_or_insert_default().x = v }
                            })}
                            {point_input("Y", |g| get(hex(g).and_then(|h| h.b2)).y, |g, v| {
                                if let Some(h) = hex_mut(g) { h.b2.get_or_insert_default().y = v }
                            })}
                        </fieldset>
                    }
                        .into_any()
                } else {
                    view! {
                        <fieldset>
                            <legend>"Cell size"</legend>
                            {point_input("Width", |g| get(square(g).and_then(|s| s.cell_size)).x, |g, v| {
                                if let Some(s) = square_mut(g) { s.cell_size.get_or_insert_default().x = v }
                            })}
                            {point_input("Height", |g| get(square(g).and_then(|s| s.cell_size)).y, |g, v| {
                                if let Some(s) = square_mut(g) { s.cell_size.get_or_insert_default().y = v }
                            })}
                        </fieldset>
                    }
                        .into_any()
                }
            }}

            <label>
                "Distance per cell"
                <input
                    type="number"
                    placeholder="(none)"
                    prop:value=move || grid.with(|g| common(g).1.map(|d| d.to_string()).unwrap_or_default())
                    on:input=move |ev| {
                        let raw = event_target_value(&ev);
                        let value = (!raw.trim().is_empty()).then(|| raw.trim().parse().unwrap_or(0.0));
                        set(&|g| with_common(g, |_, d, _| *d = value));
                    }
                />
            </label>

            <fieldset>
                <legend>"Style"</legend>
                <label>
                    "Color"
                    <input
                        type="text"
                        prop:value=move || grid.with(|g| common(g).2.color)
                        on:input=move |ev| {
                            let color = event_target_value(&ev);
                            set(&|g| with_common(g, |_, _, s| s.get_or_insert_with(default_style).color = color.clone()));
                        }
                    />
                </label>
                {point_input("Weight", |g| common(g).2.weight.unwrap_or(1.0), |g, v| {
                    with_common(g, |_, _, s| s.get_or_insert_with(default_style).weight = Some(v))
                })}
                <label>
                    "Opacity"
                    <input
                        type="number"
                        step="0.05"
                        min="0"
                        max="1"
                        prop:value=move || grid.with(|g| common(g).2.opacity.unwrap_or(0.45).to_string())
                        on:input=move |ev| {
                            let v = number(&ev);
                            set(&|g| with_common(g, |_, _, s| s.get_or_insert_with(default_style).opacity = Some(v)));
                        }
                    />
                </label>
            </fieldset>

            <div class="form-actions">
                <button type="submit" disabled=move || saving.get()>
                    {move || if saving.get() { "Applying…" } else { "Apply" }}
                </button>
                <button type="button" class="link-button" on:click=move |_| on_cancel.run(())>
                    "Cancel"
                </button>
            </div>
        </form>
    }
}
