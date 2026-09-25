//! Rust port of packages/map-core's framework-free half, plus the
//! pan/zoom math that Leaflet used to provide. The map component itself
//! lives with the front end that renders it (apps/presentation's
//! `map_canvas`).
//! Geometry is in the map image's own pixel space throughout, using
//! hexen-proto's `Point` directly.

pub mod fog;
pub mod fog_texture;
pub mod hex_math;
pub mod link_icons;
pub mod perlin_noise;
pub mod square_math;
pub mod viewport;

pub use hexen_proto::hexen::v1::Point;

pub(crate) fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}
