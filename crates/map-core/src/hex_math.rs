//! Ported from illuminated-world/site/maps/lib/app.js (via
//! packages/map-core/src/hexMath.ts) — see that file's own comment
//! block for the derivation. A calibration is an origin pixel plus two
//! basis vectors (b1, b2) spanning the hex lattice; any pixel converts
//! to axial (q, r) via the inverse of the 2x2 matrix [b1 b2]. A regular
//! hexagon's corners sit at radius |b1|/sqrt(3), starting 30deg off
//! b1's angle and stepping every 60deg — true regardless of grid
//! orientation or which two neighbour directions were used to calibrate.

use std::f64::consts::PI;

use hexen_proto::hexen::v1::HexGrid;

use crate::{pt, Point};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix2x2Inverse {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexBasis {
    pub origin: Point,
    pub b1: Point,
    pub b2: Point,
    inv: Matrix2x2Inverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Axial {
    pub q: i64,
    pub r: i64,
}

fn invert_2x2(b1: &Point, b2: &Point) -> Option<Matrix2x2Inverse> {
    let det = b1.x * b2.y - b2.x * b1.y;
    if det.abs() < 1e-9 {
        return None;
    }
    Some(Matrix2x2Inverse {
        a: b2.y / det,
        b: -b2.x / det,
        c: -b1.y / det,
        d: b1.x / det,
    })
}

pub fn load_hex_basis(grid: &HexGrid) -> Option<HexBasis> {
    let b1 = grid.b1.unwrap_or_default();
    let b2 = grid.b2.unwrap_or_default();
    let inv = invert_2x2(&b1, &b2)?;
    Some(HexBasis {
        origin: grid.origin.unwrap_or_default(),
        b1,
        b2,
        inv,
    })
}

impl HexBasis {
    /// Fractional (unrounded) axial coordinates of a pixel.
    fn axial_f(&self, p: &Point) -> (f64, f64) {
        let dx = p.x - self.origin.x;
        let dy = p.y - self.origin.y;
        (
            self.inv.a * dx + self.inv.b * dy,
            self.inv.c * dx + self.inv.d * dy,
        )
    }
}

/// JS `Math.round` semantics (halves round toward +∞), not Rust's
/// `f64::round` (halves round away from zero) — keeps results identical
/// to the TS port on exact half-way points.
fn js_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

pub fn px_to_axial(basis: &HexBasis, p: &Point) -> Axial {
    let (q, r) = basis.axial_f(p);
    Axial {
        q: js_round(q),
        r: js_round(r),
    }
}

pub fn hex_distance(basis: &HexBasis, a: &Point, b: &Point) -> i64 {
    let a = px_to_axial(basis, a);
    let b = px_to_axial(basis, b);
    let dq = a.q - b.q;
    let dr = a.r - b.r;
    (dq.abs() + (dq + dr).abs() + dr.abs()) / 2
}

pub fn hex_corners(center: &Point, b1: &Point) -> Vec<Point> {
    let s = b1.x.hypot(b1.y);
    let r = s / 3f64.sqrt();
    let angle0 = b1.y.atan2(b1.x) + PI / 6.0; // b1's angle + 30deg
    (0..6)
        .map(|k| {
            let a = angle0 + k as f64 * (PI / 3.0);
            pt(center.x + r * a.cos(), center.y + r * a.sin())
        })
        .collect()
}

/// Every hex polygon (as pixel-space corner arrays) whose centre falls within one hex of the image bounds.
pub fn build_hex_polygons(
    basis: &HexBasis,
    image_width: f64,
    image_height: f64,
) -> Vec<Vec<Point>> {
    let corners = [
        pt(0.0, 0.0),
        pt(image_width, 0.0),
        pt(0.0, image_height),
        pt(image_width, image_height),
    ];
    let (qs, rs): (Vec<f64>, Vec<f64>) = corners.iter().map(|c| basis.axial_f(c)).unzip();
    let min = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let q_min = min(&qs).floor() as i64 - 1;
    let q_max = max(&qs).ceil() as i64 + 1;
    let r_min = min(&rs).floor() as i64 - 1;
    let r_max = max(&rs).ceil() as i64 + 1;

    let mut polygons = Vec::new();
    for q in q_min..=q_max {
        for r in r_min..=r_max {
            let (q, r) = (q as f64, r as f64);
            let center = pt(
                basis.origin.x + q * basis.b1.x + r * basis.b2.x,
                basis.origin.y + q * basis.b1.y + r * basis.b2.y,
            );
            if center.x < -100.0
                || center.x > image_width + 100.0
                || center.y < -100.0
                || center.y > image_height + 100.0
            {
                continue;
            }
            polygons.push(hex_corners(&center, &basis.b1));
        }
    }
    polygons
}

#[cfg(test)]
mod tests {
    use super::*;

    // An orthogonal basis (b1 along +x, b2 along +y) isn't a "real" hex
    // lattice geometrically, but it makes the axial-coordinate arithmetic
    // easy to hand-verify, which is all these tests need from it.
    fn grid() -> HexGrid {
        HexGrid {
            origin: Some(pt(0.0, 0.0)),
            b1: Some(pt(10.0, 0.0)),
            b2: Some(pt(0.0, 10.0)),
            ..Default::default()
        }
    }

    fn basis() -> HexBasis {
        load_hex_basis(&grid()).unwrap()
    }

    #[test]
    fn origin_maps_to_zero() {
        assert_eq!(px_to_axial(&basis(), &pt(0.0, 0.0)), Axial { q: 0, r: 0 });
    }

    #[test]
    fn basis_vectors_map_to_unit_axials() {
        assert_eq!(px_to_axial(&basis(), &pt(10.0, 0.0)), Axial { q: 1, r: 0 });
        assert_eq!(px_to_axial(&basis(), &pt(0.0, 10.0)), Axial { q: 0, r: 1 });
    }

    #[test]
    fn rounds_to_the_nearest_hex() {
        assert_eq!(px_to_axial(&basis(), &pt(24.0, 4.0)), Axial { q: 2, r: 0 });
    }

    #[test]
    fn degenerate_basis_is_none() {
        let degenerate = HexGrid {
            b2: Some(pt(20.0, 0.0)),
            ..grid()
        };
        assert!(load_hex_basis(&degenerate).is_none());
    }

    #[test]
    fn distance_cases() {
        let b = basis();
        assert_eq!(hex_distance(&b, &pt(5.0, 5.0), &pt(5.0, 5.0)), 0);
        assert_eq!(hex_distance(&b, &pt(0.0, 0.0), &pt(10.0, 0.0)), 1);
        // (0,0) -> (2,2): (|dq| + |dq+dr| + |dr|) / 2 = (2 + 4 + 2) / 2 = 4
        assert_eq!(hex_distance(&b, &pt(0.0, 0.0), &pt(20.0, 20.0)), 4);
    }

    #[test]
    fn corners_are_six_points_at_the_expected_radius() {
        let corners = hex_corners(&pt(0.0, 0.0), &pt(10.0, 0.0));
        assert_eq!(corners.len(), 6);
        let expected = 10.0 / 3f64.sqrt();
        for p in &corners {
            assert!((p.x.hypot(p.y) - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn corners_start_30_degrees_off_b1() {
        let corners = hex_corners(&pt(0.0, 0.0), &pt(10.0, 0.0));
        assert!((corners[0].x - 5.0).abs() < 1e-10);
        assert!((corners[0].y - 10.0 / 3f64.sqrt() / 2.0).abs() < 1e-10);
    }

    #[test]
    fn polygons_cover_a_small_image_with_hexagons() {
        let polygons = build_hex_polygons(&basis(), 30.0, 30.0);
        assert!(!polygons.is_empty());
        assert!(polygons.iter().all(|p| p.len() == 6));
    }
}
