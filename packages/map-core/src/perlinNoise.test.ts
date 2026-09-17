import { describe, expect, test } from "bun:test";
import { fbm2, perlin2 } from "./perlinNoise.ts";

describe("perlin2", () => {
  test("is deterministic — same input, same output", () => {
    expect(perlin2(1.234, 5.678)).toBe(perlin2(1.234, 5.678));
  });

  test("is zero at integer lattice points (a known property of this algorithm)", () => {
    expect(perlin2(3, 4)).toBeCloseTo(0, 10);
  });

  test("stays within the expected range across a sample grid", () => {
    for (let x = 0; x < 5; x += 0.37) {
      for (let y = 0; y < 5; y += 0.53) {
        const n = perlin2(x, y);
        expect(n).toBeGreaterThanOrEqual(-1.01);
        expect(n).toBeLessThanOrEqual(1.01);
      }
    }
  });

  test("varies across the plane — not a constant function", () => {
    const samples = new Set<number>();
    for (let x = 0; x < 10; x += 0.7) samples.add(Math.round(perlin2(x, x * 1.3) * 1000));
    expect(samples.size).toBeGreaterThan(1);
  });
});

describe("fbm2", () => {
  test("is deterministic", () => {
    expect(fbm2(2.5, 3.5)).toBe(fbm2(2.5, 3.5));
  });

  test("stays within the expected range", () => {
    for (let x = 0; x < 5; x += 0.41) {
      for (let y = 0; y < 5; y += 0.29) {
        const n = fbm2(x, y);
        expect(n).toBeGreaterThanOrEqual(-1.01);
        expect(n).toBeLessThanOrEqual(1.01);
      }
    }
  });
});
