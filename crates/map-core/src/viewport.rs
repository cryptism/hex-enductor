//! The map's pan/zoom state and the math on it — framework-free, so the
//! front end only has to turn pointer events into calls on this.
//!
//! A [`Viewport`] maps image pixels to screen pixels (relative to the
//! map's container) as `screen = image * scale + offset`, with
//! `scale = 2^zoom` — the same zoom levels Leaflet's CRS.Simple used
//! (zoom 0 is 1:1), so the limits and snapping carry over.

use crate::{pt, Point};

pub const MIN_ZOOM: f64 = -4.0;
pub const MAX_ZOOM: f64 = 3.0;
/// Fitting to bounds rounds the zoom down to a multiple of this.
pub const ZOOM_SNAP: f64 = 0.25;
/// Leaflet's default `wheelPxPerZoomLevel`.
const WHEEL_PX_PER_ZOOM_LEVEL: f64 = 60.0;

/// How many zoom levels a wheel scroll of `delta_px` (positive = scroll
/// down = zoom out) is worth — Leaflet's ScrollWheelZoom curve: roughly
/// linear for the small deltas a trackpad sends, saturating at 4 levels,
/// and about one level for a typical ~100px mouse-wheel notch.
pub fn wheel_zoom_delta(delta_px: f64) -> f64 {
    let d2 = delta_px / (WHEEL_PX_PER_ZOOM_LEVEL * 4.0);
    let d3 = 4.0 * (2.0 / (1.0 + (-d2.abs()).exp())).log2();
    -d3.copysign(d2)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub zoom: f64,
    pub offset: Point,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            zoom: 0.0,
            offset: pt(0.0, 0.0),
        }
    }
}

impl Viewport {
    pub fn scale(&self) -> f64 {
        self.zoom.exp2()
    }

    pub fn to_screen(&self, p: &Point) -> Point {
        let s = self.scale();
        pt(p.x * s + self.offset.x, p.y * s + self.offset.y)
    }

    pub fn to_image(&self, p: &Point) -> Point {
        let s = self.scale();
        pt((p.x - self.offset.x) / s, (p.y - self.offset.y) / s)
    }

    /// The largest snapped zoom at which a `width`×`height` image fits
    /// inside a `container_w`×`container_h` container, centred.
    pub fn fit(width: f64, height: f64, container_w: f64, container_h: f64) -> Viewport {
        let zoom = if width > 0.0 && height > 0.0 && container_w > 0.0 && container_h > 0.0 {
            let exact = (container_w / width).min(container_h / height).log2();
            ((exact / ZOOM_SNAP).floor() * ZOOM_SNAP).clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            0.0
        };
        let s = zoom.exp2();
        Viewport {
            zoom,
            offset: pt(
                (container_w - width * s) / 2.0,
                (container_h - height * s) / 2.0,
            ),
        }
    }

    /// The image point at the centre of a `container_w`×`container_h`
    /// container — what Follow mode reports.
    pub fn center(&self, container_w: f64, container_h: f64) -> Point {
        self.to_image(&pt(container_w / 2.0, container_h / 2.0))
    }

    /// The view with image point `center` in the middle of the container,
    /// at `zoom` (clamped) — what Follow mode applies.
    pub fn centered_on(center: &Point, zoom: f64, container_w: f64, container_h: f64) -> Viewport {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let s = zoom.exp2();
        Viewport {
            zoom,
            offset: pt(
                container_w / 2.0 - center.x * s,
                container_h / 2.0 - center.y * s,
            ),
        }
    }

    pub fn pan_by(&self, dx: f64, dy: f64) -> Viewport {
        Viewport {
            zoom: self.zoom,
            offset: pt(self.offset.x + dx, self.offset.y + dy),
        }
    }

    /// Changes zoom (clamped) while keeping whatever image point is
    /// under the screen point `anchor` fixed there.
    pub fn zoom_around(&self, anchor: &Point, zoom: f64) -> Viewport {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let under = self.to_image(anchor);
        let s = zoom.exp2();
        Viewport {
            zoom,
            offset: pt(anchor.x - under.x * s, anchor.y - under.y * s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Point, b: &Point) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn screen_and_image_round_trip() {
        let v = Viewport {
            zoom: 1.5,
            offset: pt(12.0, -40.0),
        };
        let p = pt(123.0, 456.0);
        assert!(close(&v.to_image(&v.to_screen(&p)), &p));
    }

    #[test]
    fn fit_snaps_down_and_centres() {
        // 1000px image in a 700px square container: exact zoom log2(0.7) ≈ -0.51,
        // snapped down to -0.75.
        let v = Viewport::fit(1000.0, 1000.0, 700.0, 700.0);
        assert_eq!(v.zoom, -0.75);
        let centre = v.to_screen(&pt(500.0, 500.0));
        assert!(close(&centre, &pt(350.0, 350.0)));
    }

    #[test]
    fn fit_clamps_to_the_zoom_limits() {
        assert_eq!(Viewport::fit(10.0, 10.0, 10_000.0, 10_000.0).zoom, MAX_ZOOM);
        assert_eq!(Viewport::fit(1e6, 1e6, 100.0, 100.0).zoom, MIN_ZOOM);
    }

    #[test]
    fn fit_is_harmless_before_layout() {
        assert_eq!(Viewport::fit(100.0, 100.0, 0.0, 0.0).zoom, 0.0);
    }

    #[test]
    fn zoom_around_keeps_the_anchor_fixed() {
        let v = Viewport {
            zoom: 0.0,
            offset: pt(30.0, 20.0),
        };
        let anchor = pt(200.0, 150.0);
        let under = v.to_image(&anchor);
        let zoomed = v.zoom_around(&anchor, 2.0);
        assert!(close(&zoomed.to_screen(&under), &anchor));
    }

    #[test]
    fn wheel_notch_is_about_one_level_and_saturates() {
        assert!((wheel_zoom_delta(-100.0) - 1.08).abs() < 0.01);
        assert!((wheel_zoom_delta(100.0) + 1.08).abs() < 0.01);
        assert!(wheel_zoom_delta(-1e6) <= 4.0);
        assert_eq!(wheel_zoom_delta(0.0), 0.0);
    }

    #[test]
    fn centered_on_and_center_round_trip() {
        let v = Viewport::centered_on(&pt(120.0, 80.0), 1.25, 800.0, 600.0);
        assert!(close(&v.center(800.0, 600.0), &pt(120.0, 80.0)));
        assert_eq!(v.zoom, 1.25);
        assert_eq!(
            Viewport::centered_on(&pt(0.0, 0.0), 50.0, 1.0, 1.0).zoom,
            MAX_ZOOM
        );
    }

    #[test]
    fn zoom_around_clamps() {
        let v = Viewport::default();
        assert_eq!(v.zoom_around(&pt(0.0, 0.0), 99.0).zoom, MAX_ZOOM);
        assert_eq!(v.zoom_around(&pt(0.0, 0.0), -99.0).zoom, MIN_ZOOM);
    }
}
