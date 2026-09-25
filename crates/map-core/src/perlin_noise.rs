//! Classic 2D Perlin noise — Ken Perlin's reference permutation-table
//! algorithm — plus fBm (fractal Brownian motion, a few octaves summed)
//! for the mottled/cloudy look fog textures usually go for. No noise
//! library dependency; this is small enough to just own outright.

const PERMUTATION: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 215, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

fn perm(i: usize) -> usize {
    PERMUTATION[i & 255] as usize
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: usize, x: f64, y: f64) -> f64 {
    let h = hash & 3;
    let (u, v) = if h < 2 { (x, y) } else { (y, x) };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// Perlin noise at (x, y), roughly in [-1, 1]. Deterministic — same input, same output.
pub fn perlin2(x: f64, y: f64) -> f64 {
    let xi = (x.floor() as i64 & 255) as usize;
    let yi = (y.floor() as i64 & 255) as usize;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = fade(xf);
    let v = fade(yf);

    let a = perm(xi) + yi;
    let aa = perm(a);
    let ab = perm(a + 1);
    let b = perm(xi + 1) + yi;
    let ba = perm(b);
    let bb = perm(b + 1);

    lerp(
        v,
        lerp(u, grad(perm(aa), xf, yf), grad(perm(ba), xf - 1.0, yf)),
        lerp(
            u,
            grad(perm(ab), xf, yf - 1.0),
            grad(perm(bb), xf - 1.0, yf - 1.0),
        ),
    )
}

/// Several octaves of [`perlin2`] summed and normalized to roughly [-1, 1] — the "cloudy" look a single octave doesn't have.
pub fn fbm2(x: f64, y: f64, octaves: u32) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amplitude = 0.0;
    for _ in 0..octaves {
        total += perlin2(x * frequency, y * frequency) * amplitude;
        max_amplitude += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / max_amplitude
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perlin_is_deterministic() {
        assert_eq!(perlin2(1.234, 5.678), perlin2(1.234, 5.678));
    }

    #[test]
    fn perlin_is_zero_at_integer_lattice_points() {
        assert!(perlin2(3.0, 4.0).abs() < 1e-10);
    }

    #[test]
    fn perlin_stays_within_range() {
        let mut x = 0.0;
        while x < 5.0 {
            let mut y = 0.0;
            while y < 5.0 {
                let n = perlin2(x, y);
                assert!((-1.01..=1.01).contains(&n), "perlin2({x}, {y}) = {n}");
                y += 0.53;
            }
            x += 0.37;
        }
    }

    #[test]
    fn perlin_is_not_constant() {
        let mut samples = std::collections::HashSet::new();
        let mut x = 0.0;
        while x < 10.0 {
            samples.insert((perlin2(x, x * 1.3) * 1000.0).round() as i64);
            x += 0.7;
        }
        assert!(samples.len() > 1);
    }

    // Pinned against packages/map-core's TS implementation, so the Rust
    // and TS fog textures are the same image.
    #[test]
    fn matches_the_ts_implementation() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(perlin2(0.3, 0.7), 0.03723312096000009));
        assert!(close(perlin2(12.34, 56.78), 0.03039878445705918));
        assert!(close(fbm2(1.1, 2.2, 4), 0.06328422946133333));
        assert!(close(fbm2(0.25, 3.75, 4), -0.01617431640625));
    }

    #[test]
    fn fbm_is_deterministic_and_in_range() {
        assert_eq!(fbm2(2.5, 3.5, 4), fbm2(2.5, 3.5, 4));
        let mut x = 0.0;
        while x < 5.0 {
            let mut y = 0.0;
            while y < 5.0 {
                let n = fbm2(x, y, 4);
                assert!((-1.01..=1.01).contains(&n));
                y += 0.29;
            }
            x += 0.41;
        }
    }
}
