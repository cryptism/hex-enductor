//! The map: base image, terrain grid, link pins, fog of war — the
//! read-only subset of packages/map-core's `MapCanvas.tsx` (no Add
//! Location placement, no fog painting; those come with the editor port),
//! with no map library underneath.
//!
//! Everything in image space — the image, the grid, the fog — is one
//! `<svg>` whose content group is transformed by the current
//! [`Viewport`], so panning and zooming is a single attribute change.
//! Pins and their popups are HTML laid over it at the pins' screen
//! positions, so they stay a constant on-screen size at any zoom.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

use hexen_proto::hexen::v1::{grid, FogOfWar, Grid, ImageRef, Link};
use leptos::html::Div;
use leptos::prelude::*;
use map_core::fog::{fog_cell_corners, hidden_fog_cells};
use map_core::fog_texture::{fog_noise_rgba, FOG_TEXTURE_SIZE};
use map_core::hex_math::{build_hex_polygons, load_hex_basis};
use map_core::link_icons::find_link_icon;
use map_core::square_math::build_square_polygons;
use map_core::viewport::{wheel_zoom_delta, Viewport};
use map_core::Point;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;

const DEFAULT_MARKER_COLOR: &str = "#c19a5f";
const FOG_PATTERN_ID: &str = "hexenductor-fog-noise";
const FOG_OPACITY: f64 = 0.92;
/// A pointer that moves less than this between down and up is a click
/// (closes the open popup), not a pan.
const CLICK_SLOP_PX: f64 = 4.0;

fn svg_path(polygons: impl IntoIterator<Item = impl AsRef<[Point]>>) -> String {
    let mut d = String::new();
    for polygon in polygons {
        for (i, p) in polygon.as_ref().iter().enumerate() {
            let _ = write!(d, "{}{:.2} {:.2}", if i == 0 { "M" } else { "L" }, p.x, p.y);
        }
        d.push('Z');
    }
    d
}

fn grid_path(grid: Option<&Grid>, width: f64, height: f64) -> String {
    match grid.and_then(|g| g.kind.as_ref()) {
        Some(grid::Kind::Hex(hex)) => svg_path(
            load_hex_basis(hex)
                .map(|basis| build_hex_polygons(&basis, width, height))
                .unwrap_or_default(),
        ),
        Some(grid::Kind::Square(square)) => svg_path(build_square_polygons(square, width, height)),
        None => String::new(),
    }
}

/// Every hidden fog cell as one path — one DOM node however many cells,
/// and no hairline seams between adjacent cells.
fn fog_path(fog: &FogOfWar, width: f64, height: f64) -> String {
    svg_path(
        hidden_fog_cells(width, height, &fog.revealed_cells)
            .into_iter()
            .map(|(col, row)| fog_cell_corners(col, row, width, height)),
    )
}

fn marker_glyph(link: &Link, glyph_size: f64) -> String {
    find_link_icon(link.icon.as_deref())
        .map(|icon| {
            icon.svg.replacen(
                "<svg ",
                &format!("<svg width=\"{glyph_size}\" height=\"{glyph_size}\" "),
                1,
            )
        })
        .unwrap_or_default()
}

thread_local! {
    static FOG_TEXTURE_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The fog's Perlin tile as a PNG data: URL, rendered once per page load.
fn fog_texture_data_url() -> String {
    FOG_TEXTURE_URL.with(|cached| {
        cached
            .borrow_mut()
            .get_or_insert_with(|| {
                let canvas: web_sys::HtmlCanvasElement = document()
                    .create_element("canvas")
                    .unwrap_throw()
                    .unchecked_into();
                canvas.set_width(FOG_TEXTURE_SIZE);
                canvas.set_height(FOG_TEXTURE_SIZE);
                let ctx: web_sys::CanvasRenderingContext2d = canvas
                    .get_context("2d")
                    .unwrap_throw()
                    .expect_throw("2D canvas context unavailable")
                    .unchecked_into();
                let rgba = fog_noise_rgba();
                let image = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                    Clamped(&rgba),
                    FOG_TEXTURE_SIZE,
                    FOG_TEXTURE_SIZE,
                )
                .unwrap_throw();
                ctx.put_image_data(&image, 0.0, 0.0).unwrap_throw();
                canvas.to_data_url_with_type("image/png").unwrap_throw()
            })
            .clone()
    })
}

/// In-progress pointer gesture: one pointer pans, two pinch-zoom.
#[derive(Default)]
struct Gesture {
    pointers: HashMap<i32, Point>,
    /// Total movement since the first pointer went down.
    travel: f64,
}

fn centroid_and_spread(pointers: &HashMap<i32, Point>) -> Option<(Point, f64)> {
    let mut it = pointers.values();
    let (a, b) = (it.next()?, it.next()?);
    Some((
        Point {
            x: (a.x + b.x) / 2.0,
            y: (a.y + b.y) / 2.0,
        },
        (a.x - b.x).hypot(a.y - b.y),
    ))
}

type Listeners = (
    web_sys::HtmlDivElement,
    Closure<dyn FnMut(web_sys::WheelEvent)>,
    Closure<dyn FnMut()>,
);

#[component]
pub fn MapCanvas(
    #[prop(into)] image: Signal<ImageRef>,
    /// Wherever the caller has made image.file reachable.
    #[prop(into)]
    image_url: Signal<String>,
    #[prop(into)] grid: Signal<Option<Grid>>,
    #[prop(into)] links: Signal<Vec<Link>>,
    /// link.target -> the referenced Location's resolved title.
    #[prop(into)]
    link_titles: Signal<HashMap<String, String>>,
    #[prop(into)] selected_link_id: Signal<Option<String>>,
    #[prop(into)] on_select_link: Callback<String>,
    /// `None` means fog is off for this Location — nothing is drawn.
    #[prop(into)]
    fog: Signal<Option<FogOfWar>>,
) -> impl IntoView {
    let container = NodeRef::<Div>::new();
    let viewport = RwSignal::new(Viewport::default());
    let open_popup = RwSignal::new(None::<String>);
    let gesture = StoredValue::new_local(Gesture::default());
    let listeners = StoredValue::new_local(None::<Listeners>);

    let container_rect = move || {
        container
            .get_untracked()
            .map(|el| el.get_bounding_client_rect())
    };
    let local_point = move |client_x: i32, client_y: i32| -> Point {
        let (left, top) = container_rect().map_or((0.0, 0.0), |r| (r.left(), r.top()));
        Point {
            x: client_x as f64 - left,
            y: client_y as f64 - top,
        }
    };

    // Fit the whole image into view on mount, and again whenever it's a
    // different image (i.e. navigating to another Location).
    Effect::new(move |previous: Option<Option<(String, i32, i32)>>| {
        let key = (
            image_url.get(),
            image.with(|i| i.width),
            image.with(|i| i.height),
        );
        let rect = container.get()?.get_bounding_client_rect();
        if previous.flatten().as_ref() != Some(&key) {
            viewport.set(Viewport::fit(
                key.1 as f64,
                key.2 as f64,
                rect.width(),
                rect.height(),
            ));
            open_popup.set(None);
        }
        Some(key)
    });

    // Wheel zoom needs a non-passive listener to preventDefault page
    // scrolling, which Leptos's delegated `on:wheel` can't give; and a
    // window resize keeps the view's centre where it was.
    Effect::new(move |_| {
        let Some(el) = container.get() else { return };
        if listeners.with_value(Option::is_some) {
            return;
        }

        let on_wheel =
            Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |e: web_sys::WheelEvent| {
                e.prevent_default();
                let px = match e.delta_mode() {
                    web_sys::WheelEvent::DOM_DELTA_LINE => e.delta_y() * 20.0,
                    web_sys::WheelEvent::DOM_DELTA_PAGE => e.delta_y() * 400.0,
                    _ => e.delta_y(),
                };
                let anchor = local_point(e.client_x(), e.client_y());
                viewport.update(|v| *v = v.zoom_around(&anchor, v.zoom + wheel_zoom_delta(px)));
            });
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        el.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            on_wheel.as_ref().unchecked_ref(),
            &options,
        )
        .unwrap_throw();

        let last_size = StoredValue::new_local(container_rect().map(|r| (r.width(), r.height())));
        let on_resize = Closure::<dyn FnMut()>::new(move || {
            let Some(rect) = container_rect() else { return };
            let size = (rect.width(), rect.height());
            if let Some((w, h)) = last_size.get_value() {
                viewport.update(|v| *v = v.pan_by((size.0 - w) / 2.0, (size.1 - h) / 2.0));
            }
            last_size.set_value(Some(size));
        });
        window()
            .add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())
            .unwrap_throw();

        listeners.set_value(Some((el, on_wheel, on_resize)));
    });

    on_cleanup(move || {
        if let Some((el, on_wheel, on_resize)) = listeners.try_update_value(Option::take).flatten()
        {
            let _ =
                el.remove_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
            let _ = window()
                .remove_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
        }
    });

    let on_pointer_down = move |e: web_sys::PointerEvent| {
        // Pins, popups and the zoom buttons handle their own clicks.
        let on_control = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest(".map-pin, .map-popup, .map-zoom").ok().flatten())
            .is_some();
        if on_control || e.button() != 0 {
            return;
        }
        if let Some(el) = container.get_untracked() {
            let _ = el.set_pointer_capture(e.pointer_id());
        }
        let p = local_point(e.client_x(), e.client_y());
        gesture.update_value(|g| {
            if g.pointers.is_empty() {
                g.travel = 0.0;
            }
            g.pointers.insert(e.pointer_id(), p);
        });
    };

    let on_pointer_move = move |e: web_sys::PointerEvent| {
        let p = local_point(e.client_x(), e.client_y());
        gesture.update_value(|g| {
            let Some(&prev) = g.pointers.get(&e.pointer_id()) else {
                return;
            };
            let before = centroid_and_spread(&g.pointers);
            g.pointers.insert(e.pointer_id(), p);
            g.travel += (p.x - prev.x).hypot(p.y - prev.y);
            match (before, centroid_and_spread(&g.pointers)) {
                (Some((c0, s0)), Some((c1, s1))) if s0 > 0.0 => viewport.update(|v| {
                    *v = v
                        .pan_by(c1.x - c0.x, c1.y - c0.y)
                        .zoom_around(&c1, v.zoom + (s1 / s0).log2());
                }),
                _ if g.pointers.len() == 1 => {
                    viewport.update(|v| *v = v.pan_by(p.x - prev.x, p.y - prev.y))
                }
                _ => {}
            }
        });
    };

    let on_pointer_up = move |e: web_sys::PointerEvent| {
        gesture.update_value(|g| {
            if g.pointers.remove(&e.pointer_id()).is_some()
                && g.pointers.is_empty()
                && g.travel < CLICK_SLOP_PX
            {
                // A plain click on the map closes the open popup, as Leaflet did.
                open_popup.set(None);
            }
        });
    };

    let zoom_by = move |delta: f64| {
        let Some(rect) = container_rect() else { return };
        let centre = Point {
            x: rect.width() / 2.0,
            y: rect.height() / 2.0,
        };
        viewport.update(|v| *v = v.zoom_around(&centre, v.zoom + delta));
    };

    let transform = move || {
        viewport.with(|v| {
            format!(
                "translate({} {}) scale({})",
                v.offset.x,
                v.offset.y,
                v.scale()
            )
        })
    };
    let width = move || image.with(|i| i.width);
    let height = move || image.with(|i| i.height);
    let grid_d = Memo::new(move |_| {
        image.with(|i| grid.with(|g| grid_path(g.as_ref(), i.width as f64, i.height as f64)))
    });
    let grid_style = Memo::new(move |_| {
        grid.with(|g| {
            let style = match g.as_ref().and_then(|g| g.kind.as_ref()) {
                Some(grid::Kind::Hex(hex)) => hex.style.clone(),
                Some(grid::Kind::Square(square)) => square.style.clone(),
                None => None,
            };
            (
                style
                    .as_ref()
                    .map_or_else(|| DEFAULT_MARKER_COLOR.to_owned(), |s| s.color.clone()),
                style.as_ref().and_then(|s| s.weight).unwrap_or(1.0),
                style.as_ref().and_then(|s| s.opacity).unwrap_or(0.45),
            )
        })
    });
    let fog_d = Memo::new(move |_| {
        image.with(|i| {
            fog.with(|f| {
                f.as_ref()
                    .map(|f| fog_path(f, i.width as f64, i.height as f64))
            })
        })
    });
    let visible_links = Memo::new(move |_| {
        links.with(|links| {
            links
                .iter()
                .filter(|l| !l.hidden.unwrap_or(false))
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    let pin_view = move |link: Link| {
        let id = link.id.clone();
        let selected =
            Memo::new(move |_| selected_link_id.with(|s| s.as_deref() == Some(id.as_str())));
        let size = move || if selected.get() { 36.0 } else { 28.0 };
        let color = link
            .color
            .clone()
            .unwrap_or_else(|| DEFAULT_MARKER_COLOR.to_owned());
        let position = Point {
            x: link.x,
            y: link.y,
        };
        let style = move || {
            let s = viewport.with(|v| v.to_screen(&position));
            let size = size();
            format!(
                "left: {}px; top: {}px; width: {size}px; height: {size}px; border-color: {color};",
                s.x, s.y
            )
        };
        let id = link.id.clone();
        let glyph = move || marker_glyph(&link, (size() * 0.68_f64).round());
        view! {
            <div
                class="map-pin"
                style=style
                on:click=move |_| {
                    open_popup.set(Some(id.clone()));
                    on_select_link.run(id.clone());
                }
                inner_html=glyph
            ></div>
        }
    };

    let popup = move || {
        let id = open_popup.get()?;
        let link = visible_links.with(|links| links.iter().find(|l| l.id == id).cloned())?;
        let title = link_titles
            .with(|t| t.get(&link.target).cloned())
            .unwrap_or_else(|| link.target.clone());
        let position = Point {
            x: link.x,
            y: link.y,
        };
        let pin_radius = if selected_link_id.with(|s| s.as_deref() == Some(id.as_str())) {
            18.0
        } else {
            14.0
        };
        let style = move || {
            let s = viewport.with(|v| v.to_screen(&position));
            format!("left: {}px; top: {}px;", s.x, s.y - pin_radius - 8.0)
        };
        Some(view! {
            <div class="map-popup" style=style>
                <button type="button" class="map-popup-close" on:click=move |_| open_popup.set(None)>
                    "×"
                </button>
                <strong>{title}</strong>
                <br />
                <span class="map-popup-type">{link.r#type.clone()}</span>
            </div>
        })
    };

    view! {
        <div
            node_ref=container
            class="map-canvas"
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up
            on:pointercancel=on_pointer_up
        >
            <svg class="map-surface">
                <defs>
                    <pattern
                        id=FOG_PATTERN_ID
                        patternUnits="userSpaceOnUse"
                        width=FOG_TEXTURE_SIZE
                        height=FOG_TEXTURE_SIZE
                    >
                        <image
                            href=move || fog_d.with(Option::is_some).then(fog_texture_data_url)
                            width=FOG_TEXTURE_SIZE
                            height=FOG_TEXTURE_SIZE
                        />
                    </pattern>
                </defs>
                <g transform=transform>
                    <image href=image_url width=width height=height preserveAspectRatio="none" />
                    <path
                        d=move || grid_d.get()
                        fill="none"
                        stroke=move || grid_style.get().0
                        stroke-width=move || grid_style.get().1
                        stroke-opacity=move || grid_style.get().2
                        vector-effect="non-scaling-stroke"
                    />
                    <path
                        d=move || fog_d.get().unwrap_or_default()
                        fill=format!("url(#{FOG_PATTERN_ID})")
                        fill-opacity=FOG_OPACITY
                    />
                </g>
            </svg>
            // Keyed on the whole link (its Debug form — prost messages
            // aren't Hash), so an edited pin re-renders and an untouched
            // one doesn't.
            <For each=move || visible_links.get() key=|link| format!("{link:?}") children=pin_view />
            {popup}
            <div class="map-zoom">
                <button type="button" title="Zoom in" on:click=move |_| zoom_by(1.0)>
                    "+"
                </button>
                <button type="button" title="Zoom out" on:click=move |_| zoom_by(-1.0)>
                    "−"
                </button>
            </div>
        </div>
    }
}
