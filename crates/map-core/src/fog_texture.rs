//! The fog overlay's tile: a Perlin-fbm-mottled version of the fog's
//! base color. This module only produces the RGBA pixels; turning them
//! into something a browser can draw (a canvas → data: URL) is the
//! front end's job.

use crate::perlin_noise::fbm2;

/// Tile size of the generated texture, in pixels — also what callers should use as the SVG pattern's tile size so it maps 1:1, no scaling artifacts.
pub const FOG_TEXTURE_SIZE: u32 = 128;

const FOG_BASE_COLOR: [f64; 3] = [10.0, 10.0, 8.0]; // #0a0a08, the flat color this replaced
const FOG_LIGHT_COLOR: [f64; 3] = [42.0, 44.0, 35.0]; // a mottled, slightly warmer highlight

/// Row-major RGBA, `FOG_TEXTURE_SIZE²` pixels, fully opaque — the
/// overlay's own fill opacity does any dimming, independent of this.
pub fn fog_noise_rgba() -> Vec<u8> {
    let size = FOG_TEXTURE_SIZE;
    let scale = 4.0; // how many noise "cells" the tile spans — higher = finer mottling
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let n = fbm2(
                x as f64 / size as f64 * scale,
                y as f64 / size as f64 * scale,
                4,
            );
            let t = (n + 1.0) / 2.0; // 0..1
            for c in 0..3 {
                data.push(
                    (FOG_BASE_COLOR[c] + t * (FOG_LIGHT_COLOR[c] - FOG_BASE_COLOR[c])).round()
                        as u8,
                );
            }
            data.push(255);
        }
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_a_full_opaque_tile_between_the_two_colors() {
        let data = fog_noise_rgba();
        assert_eq!(
            data.len(),
            (FOG_TEXTURE_SIZE * FOG_TEXTURE_SIZE * 4) as usize
        );
        for px in data.chunks_exact(4) {
            assert_eq!(px[3], 255);
            assert!((10..=42).contains(&px[0]));
        }
    }
}
