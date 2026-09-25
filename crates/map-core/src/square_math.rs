use hexen_proto::hexen::v1::SquareGrid;

use crate::{pt, Point};

/// The four corners of cell (i, j), where origin is cell (0, 0)'s top-left corner.
pub fn square_corners(origin: &Point, cell_size: &Point, i: i64, j: i64) -> Vec<Point> {
    let x0 = origin.x + i as f64 * cell_size.x;
    let y0 = origin.y + j as f64 * cell_size.y;
    let x1 = x0 + cell_size.x;
    let y1 = y0 + cell_size.y;
    vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)]
}

/// Every square-cell polygon (as pixel-space corner arrays) that overlaps the image bounds, one cell of margin either side.
pub fn build_square_polygons(
    grid: &SquareGrid,
    image_width: f64,
    image_height: f64,
) -> Vec<Vec<Point>> {
    let origin = grid.origin.unwrap_or_default();
    let cell_size = grid.cell_size.unwrap_or_default();
    if cell_size.x == 0.0 || cell_size.y == 0.0 {
        return Vec::new();
    }

    let i_min = (-origin.x / cell_size.x).floor() as i64 - 1;
    let i_max = ((image_width - origin.x) / cell_size.x).ceil() as i64 + 1;
    let j_min = (-origin.y / cell_size.y).floor() as i64 - 1;
    let j_max = ((image_height - origin.y) / cell_size.y).ceil() as i64 + 1;

    let mut polygons = Vec::new();
    for i in i_min.min(i_max)..=i_min.max(i_max) {
        for j in j_min.min(j_max)..=j_min.max(j_max) {
            polygons.push(square_corners(&origin, &cell_size, i, j));
        }
    }
    polygons
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> SquareGrid {
        SquareGrid {
            origin: Some(pt(0.0, 0.0)),
            cell_size: Some(pt(32.0, 32.0)),
            ..Default::default()
        }
    }

    #[test]
    fn corners_of_cell_zero_are_the_unit_square_from_origin() {
        assert_eq!(
            square_corners(&pt(0.0, 0.0), &pt(32.0, 32.0), 0, 0),
            vec![pt(0.0, 0.0), pt(32.0, 0.0), pt(32.0, 32.0), pt(0.0, 32.0)]
        );
    }

    #[test]
    fn corners_offset_by_cell_index() {
        assert_eq!(
            square_corners(&pt(0.0, 0.0), &pt(32.0, 32.0), 2, 1),
            vec![
                pt(64.0, 32.0),
                pt(96.0, 32.0),
                pt(96.0, 64.0),
                pt(64.0, 64.0)
            ]
        );
    }

    #[test]
    fn corners_respect_a_non_zero_origin() {
        assert_eq!(
            square_corners(&pt(10.0, 20.0), &pt(32.0, 32.0), 0, 0),
            vec![
                pt(10.0, 20.0),
                pt(42.0, 20.0),
                pt(42.0, 52.0),
                pt(10.0, 52.0)
            ]
        );
    }

    #[test]
    fn tiles_a_small_image_with_quads() {
        let polygons = build_square_polygons(&grid(), 64.0, 64.0);
        assert!(!polygons.is_empty());
        assert!(polygons.iter().all(|p| p.len() == 4));
    }

    #[test]
    fn covers_the_full_image() {
        let polygons = build_square_polygons(&grid(), 100.0, 100.0);
        let all = polygons.iter().flatten();
        let (xs, ys): (Vec<f64>, Vec<f64>) = all.map(|p| (p.x, p.y)).unzip();
        assert!(xs.iter().cloned().fold(f64::INFINITY, f64::min) <= 0.0);
        assert!(xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) >= 100.0);
        assert!(ys.iter().cloned().fold(f64::INFINITY, f64::min) <= 0.0);
        assert!(ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max) >= 100.0);
    }

    #[test]
    fn nothing_for_a_zero_cell_size() {
        let degenerate = SquareGrid {
            cell_size: Some(pt(0.0, 32.0)),
            ..grid()
        };
        assert!(build_square_polygons(&degenerate, 100.0, 100.0).is_empty());
    }

    #[test]
    fn tiles_when_the_origin_is_outside_the_image() {
        let off = SquareGrid {
            origin: Some(pt(-500.0, -500.0)),
            ..grid()
        };
        assert!(!build_square_polygons(&off, 64.0, 64.0).is_empty());
    }
}
