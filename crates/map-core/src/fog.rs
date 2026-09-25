//! Fog of war is addressed by its own uniform pixel grid over the base
//! image — independent of whatever terrain grid (hex/square/none) the
//! Location has, since a map without a terrain grid can still have fog.
//! Cell keys ("col,row") are opaque to the server; only this module and
//! the map canvas need to know what they mean. They're stored in
//! project files, so changing the scheme (cell size, key format) would
//! scramble every saved fog state.

use std::collections::HashSet;

use crate::{pt, Point};

pub const FOG_CELL_SIZE: f64 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FogGridDims {
    pub cols: u32,
    pub rows: u32,
}

pub fn fog_grid_dims(width: f64, height: f64) -> FogGridDims {
    FogGridDims {
        cols: ((width / FOG_CELL_SIZE).ceil() as u32).max(1),
        rows: ((height / FOG_CELL_SIZE).ceil() as u32).max(1),
    }
}

pub fn fog_cell_key(col: i64, row: i64) -> String {
    format!("{col},{row}")
}

/// The inverse of [`fog_cell_key`]; `None` for anything that isn't one.
pub fn parse_fog_cell_key(key: &str) -> Option<(i64, i64)> {
    let (col, row) = key.split_once(',')?;
    Some((col.parse().ok()?, row.parse().ok()?))
}

/// Which fog cell a pixel-space point falls in.
pub fn fog_cell_at(point: &Point) -> String {
    fog_cell_key(
        (point.x / FOG_CELL_SIZE).floor() as i64,
        (point.y / FOG_CELL_SIZE).floor() as i64,
    )
}

/// The four pixel-space corners of one fog cell, clamped to the image bounds.
pub fn fog_cell_corners(col: i64, row: i64, width: f64, height: f64) -> [Point; 4] {
    let x0 = col as f64 * FOG_CELL_SIZE;
    let y0 = row as f64 * FOG_CELL_SIZE;
    let x1 = (x0 + FOG_CELL_SIZE).min(width);
    let y1 = (y0 + FOG_CELL_SIZE).min(height);
    [pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]
}

/// Every (col, row) covering the image that isn't in `revealed_cells`,
/// row-major.
pub fn hidden_fog_cells<S: AsRef<str>>(
    width: f64,
    height: f64,
    revealed_cells: &[S],
) -> Vec<(i64, i64)> {
    let revealed: HashSet<&str> = revealed_cells.iter().map(AsRef::as_ref).collect();
    let FogGridDims { cols, rows } = fog_grid_dims(width, height);
    let mut hidden = Vec::new();
    for row in 0..rows as i64 {
        for col in 0..cols as i64 {
            if !revealed.contains(fog_cell_key(col, row).as_str()) {
                hidden.push((col, row));
            }
        }
    }
    hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: f64 = FOG_CELL_SIZE;
    const NONE: &[&str] = &[];

    #[test]
    fn grid_dims_ceil_to_cover_a_partial_trailing_cell() {
        assert_eq!(
            fog_grid_dims(S * 2.0 + 1.0, S),
            FogGridDims { cols: 3, rows: 1 }
        );
    }

    #[test]
    fn grid_dims_are_never_smaller_than_1x1() {
        assert_eq!(fog_grid_dims(1.0, 1.0), FogGridDims { cols: 1, rows: 1 });
    }

    #[test]
    fn cell_at_maps_a_pixel_to_its_containing_cell() {
        assert_eq!(fog_cell_at(&pt(0.0, 0.0)), fog_cell_key(0, 0));
        assert_eq!(fog_cell_at(&pt(S + 5.0, S * 2.0 + 5.0)), fog_cell_key(1, 2));
    }

    #[test]
    fn keys_round_trip() {
        assert_eq!(parse_fog_cell_key(&fog_cell_key(3, 7)), Some((3, 7)));
        assert_eq!(parse_fog_cell_key("nonsense"), None);
    }

    #[test]
    fn corners_of_a_full_cell_away_from_the_edge() {
        assert_eq!(
            fog_cell_corners(1, 1, S * 4.0, S * 4.0),
            [
                pt(S, S),
                pt(S * 2.0, S),
                pt(S * 2.0, S * 2.0),
                pt(S, S * 2.0)
            ]
        );
    }

    #[test]
    fn corners_clamp_a_trailing_cell_to_the_image() {
        assert_eq!(
            fog_cell_corners(1, 1, S + 10.0, S + 10.0),
            [
                pt(S, S),
                pt(S + 10.0, S),
                pt(S + 10.0, S + 10.0),
                pt(S, S + 10.0)
            ]
        );
    }

    #[test]
    fn hidden_is_every_cell_when_nothing_is_revealed() {
        assert_eq!(hidden_fog_cells(S * 2.0, S, NONE), vec![(0, 0), (1, 0)]);
    }

    #[test]
    fn hidden_excludes_revealed_cells() {
        assert_eq!(hidden_fog_cells(S * 2.0, S, &["0,0"]), vec![(1, 0)]);
    }

    #[test]
    fn hidden_is_empty_once_everything_is_revealed() {
        assert!(hidden_fog_cells(S, S, &["0,0"]).is_empty());
    }
}
