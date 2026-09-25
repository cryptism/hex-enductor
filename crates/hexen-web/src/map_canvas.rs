//! The map: base image, terrain grid, link pins, fog of war, pings, plus
//! the editor's map tools (click-to-place, fog painting, the Ping tool)
//! and both halves of Follow mode (reporting this view; following
//! someone else's) — the port of packages/map-core's `MapCanvas.tsx`,
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
use map_core::fog::{fog_cell_at, fog_cell_corners, hidden_fog_cells, FOG_CELL_SIZE};
use map_core::fog_texture::{fog_noise_rgba, FOG_TEXTURE_SIZE};
use map_core::hex_math::{build_hex_polygons, load_hex_basis};
use map_core::link_icons::find_link_icon;
use map_core::square_math::build_square_polygons;
use map_core::viewport::{wheel_zoom_delta, Viewport};
use map_core::Point;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;

const DEFAULT_MARKER_COLOR: &str = "#c19a5f";
const FOG_PATTERN_ID: &str = "hexenductor-fog-noise";
const FOG_OPACITY: f64 = 0.92;
/// Held down while painting, the fog layer dims so the GM can see what
/// they're about to re-cover instead of painting blind.
const FOG_OPACITY_ERASING: f64 = 0.35;
/// How often a held-down paint stroke flushes its touched cells as one
/// setFogCells command — batched so a fast drag across many cells lands
/// (and broadcasts to every other viewer) as a handful of updates, not
/// one per cell.
const FOG_PAINT_FLUSH_MS: i32 = 80;

/// How long a ping's radiating rings take, start to finish (three rings
/// 0.35s apart, 1.3s each — see `.map-ping` in map.css). Callers clear
/// `ping_at` no sooner than this, or the last ring cuts off.
pub const PING_EFFECT_DURATION_MS: u64 = 2000;

/// A ping to show. `key` must change even for a repeat ping at the same
/// spot, so the animation restarts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PingMark {
    pub x: f64,
    pub y: f64,
    pub key: u64,
}

/// A map view for Follow mode: the image point at the centre, and the
/// zoom level (see `Viewport`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapView {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

/// How long following a view takes to glide there.
const FOLLOW_ANIMATION_MS: f64 = 300.0;
/// A wheel/zoom-button gesture reports its view this long after it stops.
const VIEW_REPORT_DEBOUNCE_MS: u64 = 150;
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

/// A fog paint stroke in progress: its direction is decided once, at
/// pointerdown (shift = restore fog), and the cells it has crossed
/// since the last flush wait in `pending`.
struct Stroke {
    pointer_id: i32,
    revealed: bool,
    pending: Vec<String>,
    last: Point,
    interval: i32,
    _tick: Closure<dyn FnMut()>,
}

/// Every fog cell a straight drag from `a` to `b` (image pixels) passes
/// over — pointer events arrive far apart on a fast drag, so sampling
/// only their positions would leave gaps in the stroke.
fn cells_along(a: &Point, b: &Point) -> Vec<String> {
    let steps = ((b.x - a.x).hypot(b.y - a.y) / (FOG_CELL_SIZE / 4.0))
        .ceil()
        .max(1.0) as usize;
    let mut cells: Vec<String> = Vec::new();
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let cell = fog_cell_at(&Point {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
        });
        if cells.last() != Some(&cell) {
            cells.push(cell);
        }
    }
    cells
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
    #[prop(optional, into)] on_select_link: Option<Callback<String>>,
    /// `None` means fog is off for this Location — nothing is drawn.
    #[prop(into)]
    fog: Signal<Option<FogOfWar>>,
    /// Show the grid overlay at all — a view toggle, independent of whether `grid` itself is configured.
    #[prop(into, default = true.into())]
    grid_visible: Signal<bool>,
    /// The Add Location tool: while true, a click (not a drag) on the map calls `on_place` with the image point.
    #[prop(into, default = false.into())]
    placing: Signal<bool>,
    #[prop(optional, into)] on_place: Option<Callback<Point>>,
    /// While true (and fog is present), click-drag paints fog cells instead of panning the map.
    #[prop(into, default = false.into())]
    fog_editable: Signal<bool>,
    /// `(cells, revealed)` — one call per flushed batch of a stroke.
    #[prop(optional, into)]
    on_paint_fog_cells: Option<Callback<(Vec<String>, bool)>>,
    /// The Ping tool: while true, a click on the map calls `on_ping` with the image point.
    #[prop(into, default = false.into())]
    pinging: Signal<bool>,
    #[prop(optional, into)] on_ping: Option<Callback<Point>>,
    /// A ping to show; the caller clears it after `PING_EFFECT_DURATION_MS`.
    #[prop(into, default = Signal::stored(None))]
    ping_at: Signal<Option<PingMark>>,
    /// Follow mode, leading side: called with this map's view after every pan/zoom.
    #[prop(optional, into)]
    on_view_change: Option<Callback<MapView>>,
    /// Follow mode, following side: glides to this view whenever it changes.
    #[prop(into, default = Signal::stored(None))]
    follow_view: Signal<Option<MapView>>,
) -> impl IntoView {
    let container = NodeRef::<Div>::new();
    let viewport = RwSignal::new(Viewport::default());
    let open_popup = RwSignal::new(None::<String>);
    let gesture = StoredValue::new_local(Gesture::default());
    let listeners = StoredValue::new_local(None::<Listeners>);
    let stroke = StoredValue::new_local(None::<Stroke>);
    let painting = Memo::new(move |_| {
        fog_editable.get() && fog.with(Option::is_some) && on_paint_fog_cells.is_some()
    });

    let flush_stroke = move || {
        let batch = stroke.try_update_value(|s| {
            s.as_mut()
                .map(|s| (std::mem::take(&mut s.pending), s.revealed))
        });
        if let (Some(Some((cells, revealed))), Some(on_paint)) = (batch, on_paint_fog_cells) {
            if !cells.is_empty() {
                on_paint.run((cells, revealed));
            }
        }
    };
    let end_stroke = move || {
        flush_stroke();
        if let Some(Some(s)) = stroke.try_update_value(Option::take) {
            window().clear_interval_with_handle(s.interval);
        }
    };
    // The tool being switched off mid-stroke still lands what was painted.
    Effect::new(move |_| {
        if !painting.get() {
            end_stroke();
        }
    });

    // Only tracked while the paint tool is armed — holding shift previews
    // erase mode by dimming the fog layer, the same key a stroke reads to
    // decide its direction.
    let shift_held = RwSignal::new(false);
    let key_listeners = StoredValue::new_local(None::<Closure<dyn FnMut(web_sys::KeyboardEvent)>>);
    let detach_keys = move || {
        if let Some(Some(on_key)) = key_listeners.try_update_value(Option::take) {
            for event in ["keydown", "keyup"] {
                let _ = window()
                    .remove_event_listener_with_callback(event, on_key.as_ref().unchecked_ref());
            }
        }
        shift_held.set(false);
    };
    Effect::new(move |_| {
        detach_keys();
        if !painting.get() {
            return;
        }
        let on_key =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Shift" {
                    shift_held.set(e.type_() == "keydown");
                }
            });
        for event in ["keydown", "keyup"] {
            window()
                .add_event_listener_with_callback(event, on_key.as_ref().unchecked_ref())
                .unwrap_throw();
        }
        key_listeners.set_value(Some(on_key));
    });

    let container_rect = move || {
        container
            .get_untracked()
            .map(|el| el.get_bounding_client_rect())
    };
    let report_view = move || {
        let (Some(on_view_change), Some(rect)) = (on_view_change, container_rect()) else {
            return;
        };
        let v = viewport.get_untracked();
        let c = v.center(rect.width(), rect.height());
        on_view_change.run(MapView {
            x: c.x,
            y: c.y,
            zoom: v.zoom,
        });
    };
    let report_timer = StoredValue::new_local(None::<TimeoutHandle>);
    let report_view_soon = move || {
        if on_view_change.is_none() {
            return;
        }
        if let Some(Some(timer)) = report_timer.try_update_value(Option::take) {
            timer.clear();
        }
        let timer =
            set_timeout_with_handle(report_view, Duration::from_millis(VIEW_REPORT_DEBOUNCE_MS))
                .ok();
        report_timer.set_value(timer);
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
                report_view_soon();
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

    // Follow mode, following side: glide from wherever this map is to the
    // followed view. A newer target supersedes an animation in flight.
    let animation = StoredValue::new(0u64);
    Effect::new(move |_| {
        let Some(target) = follow_view.get() else {
            return;
        };
        let Some(rect) = container.get().map(|el| el.get_bounding_client_rect()) else {
            return;
        };
        let (w, h) = (rect.width(), rect.height());
        let from = viewport.get_untracked();
        animation.update_value(|n| *n += 1);
        animate_to(
            viewport,
            animation,
            animation.get_value(),
            js_sys::Date::now(),
            from.center(w, h),
            from.zoom,
            target,
            (w, h),
        );
    });

    on_cleanup(move || {
        end_stroke();
        if let Some(Some(timer)) = report_timer.try_update_value(Option::take) {
            timer.clear();
        }
        detach_keys();
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

        if painting.get_untracked() {
            if stroke.with_value(Option::is_some) {
                return; // a second finger doesn't start a second stroke
            }
            let at = viewport.with_untracked(|v| v.to_image(&p));
            let tick = Closure::<dyn FnMut()>::new(flush_stroke);
            let interval = window()
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    tick.as_ref().unchecked_ref(),
                    FOG_PAINT_FLUSH_MS,
                )
                .unwrap_throw();
            stroke.set_value(Some(Stroke {
                pointer_id: e.pointer_id(),
                revealed: !e.shift_key(),
                pending: vec![fog_cell_at(&at)],
                last: at,
                interval,
                _tick: tick,
            }));
            return;
        }

        gesture.update_value(|g| {
            if g.pointers.is_empty() {
                g.travel = 0.0;
            }
            g.pointers.insert(e.pointer_id(), p);
        });
    };

    let on_pointer_move = move |e: web_sys::PointerEvent| {
        let p = local_point(e.client_x(), e.client_y());
        let painted = stroke.try_update_value(|s| {
            let s = s.as_mut().filter(|s| s.pointer_id == e.pointer_id())?;
            let at = viewport.with_untracked(|v| v.to_image(&p));
            for cell in cells_along(&s.last, &at) {
                if !s.pending.contains(&cell) {
                    s.pending.push(cell);
                }
            }
            s.last = at;
            Some(())
        });
        if matches!(painted, Some(Some(()))) {
            return;
        }
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
        if stroke.with_value(|s| s.as_ref().is_some_and(|s| s.pointer_id == e.pointer_id())) {
            end_stroke();
            return;
        }
        let p = local_point(e.client_x(), e.client_y());
        let clicked = gesture
            .try_update_value(|g| {
                g.pointers.remove(&e.pointer_id()).is_some()
                    && g.pointers.is_empty()
                    && g.travel < CLICK_SLOP_PX
            })
            .unwrap_or(false);
        if clicked {
            // A plain click on the map closes the open popup, as Leaflet did.
            open_popup.set(None);
            let at = viewport.with_untracked(|v| v.to_image(&p));
            match (
                placing.get_untracked(),
                on_place,
                pinging.get_untracked(),
                on_ping,
            ) {
                (true, Some(on_place), _, _) => on_place.run(at),
                (_, _, true, Some(on_ping)) => on_ping.run(at),
                _ => {}
            }
        } else if gesture.with_value(|g| g.pointers.is_empty()) {
            // A pan or pinch just ended.
            report_view();
        }
    };

    let zoom_by = move |delta: f64| {
        let Some(rect) = container_rect() else { return };
        let centre = Point {
            x: rect.width() / 2.0,
            y: rect.height() / 2.0,
        };
        viewport.update(|v| *v = v.zoom_around(&centre, v.zoom + delta));
        report_view_soon();
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
    // Pins under fog are covered by it, as the fog layer's own opacity
    // would cover them if they were drawn beneath it — and can't be
    // clicked, so a player can't find a hidden pin by feel.
    let hidden_cells = Memo::new(move |_| {
        image.with(|i| {
            fog.with(|f| {
                f.as_ref().map(|f| {
                    hidden_fog_cells(i.width as f64, i.height as f64, &f.revealed_cells)
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                })
            })
        })
    });
    let fog_opacity = move || {
        if shift_held.get() {
            FOG_OPACITY_ERASING
        } else {
            FOG_OPACITY
        }
    };

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
        let cell = (
            (position.x / FOG_CELL_SIZE).floor() as i64,
            (position.y / FOG_CELL_SIZE).floor() as i64,
        );
        let fogged = Memo::new(move |_| {
            hidden_cells.with(|h| h.as_ref().is_some_and(|h| h.contains(&cell)))
        });
        let style = move || {
            let s = viewport.with(|v| v.to_screen(&position));
            let size = size();
            let under_fog = if fogged.get() {
                format!(" opacity: {}; pointer-events: none;", 1.0 - fog_opacity())
            } else {
                String::new()
            };
            format!(
                "left: {}px; top: {}px; width: {size}px; height: {size}px; border-color: {color};{under_fog}",
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
                    if let Some(on_select_link) = on_select_link {
                        on_select_link.run(id.clone());
                    }
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
            class=move || {
                if painting.get() {
                    "map-canvas painting-fog"
                } else if placing.get() || pinging.get() {
                    "map-canvas placing"
                } else {
                    "map-canvas"
                }
            }
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
                        d=move || if grid_visible.get() { grid_d.get() } else { String::new() }
                        fill="none"
                        stroke=move || grid_style.get().0
                        stroke-width=move || grid_style.get().1
                        stroke-opacity=move || grid_style.get().2
                        vector-effect="non-scaling-stroke"
                    />
                    <path
                        d=move || fog_d.get().unwrap_or_default()
                        fill=format!("url(#{FOG_PATTERN_ID})")
                        fill-opacity=move || if shift_held.get() { FOG_OPACITY_ERASING } else { FOG_OPACITY }
                    />
                </g>
            </svg>
            // Keyed on the whole link (its Debug form — prost messages
            // aren't Hash), so an edited pin re-renders and an untouched
            // one doesn't.
            <For each=move || visible_links.get() key=|link| format!("{link:?}") children=pin_view />
            {popup}
            {move || {
                ping_at
                    .get()
                    .map(|mark| {
                        let at = Point { x: mark.x, y: mark.y };
                        let style = move || {
                            let s = viewport.with(|v| v.to_screen(&at));
                            format!("left: {}px; top: {}px;", s.x, s.y)
                        };
                        view! {
                            <div class="map-ping" style=style>
                                <span class="map-ping-dot"></span>
                                <span class="map-ping-ring"></span>
                                <span class="map-ping-ring"></span>
                                <span class="map-ping-ring"></span>
                            </div>
                        }
                    })
            }}
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

/// One animation frame of Follow mode's glide: eases the centre point
/// and zoom from where the map was to `target`, then schedules the next
/// frame — unless a newer target has started its own glide.
#[allow(clippy::too_many_arguments)]
fn animate_to(
    viewport: RwSignal<Viewport>,
    animation: StoredValue<u64>,
    generation: u64,
    start: f64,
    from_center: Point,
    from_zoom: f64,
    target: MapView,
    (w, h): (f64, f64),
) {
    if animation.try_get_value() != Some(generation) {
        return;
    }
    let t = ((js_sys::Date::now() - start) / FOLLOW_ANIMATION_MS).clamp(0.0, 1.0);
    let eased = if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    };
    let lerp = |a: f64, b: f64| a + (b - a) * eased;
    let center = Point {
        x: lerp(from_center.x, target.x),
        y: lerp(from_center.y, target.y),
    };
    viewport.set(Viewport::centered_on(
        &center,
        lerp(from_zoom, target.zoom),
        w,
        h,
    ));
    if t < 1.0 {
        request_animation_frame(move || {
            animate_to(
                viewport,
                animation,
                generation,
                start,
                from_center,
                from_zoom,
                target,
                (w, h),
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fast_drag_covers_every_cell_it_crosses() {
        let cells = cells_along(&Point { x: 10.0, y: 10.0 }, &Point { x: 250.0, y: 10.0 });
        assert_eq!(cells, ["0,0", "1,0", "2,0", "3,0"]);
    }

    #[test]
    fn a_diagonal_drag_steps_through_neighbouring_cells() {
        let cells = cells_along(&Point { x: 10.0, y: 10.0 }, &Point { x: 140.0, y: 140.0 });
        assert_eq!(cells.first().map(String::as_str), Some("0,0"));
        assert_eq!(cells.last().map(String::as_str), Some("2,2"));
        assert!(cells.contains(&"1,1".to_string()));
    }
}
