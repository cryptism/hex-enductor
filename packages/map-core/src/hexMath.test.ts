import { describe, expect, test } from "bun:test";
import type { HexGrid } from "@hex-enductor/hexen-schema";
import { loadHexBasis, pxToAxial, hexDistance, hexCorners, buildHexPolygons } from "./hexMath.ts";

// An orthogonal basis (b1 along +x, b2 along +y) isn't a "real" hex
// lattice geometrically, but it makes the axial-coordinate arithmetic
// easy to hand-verify, which is all these tests need from it.
const GRID: HexGrid = {
  type: "hex",
  origin: { x: 0, y: 0 },
  b1: { x: 10, y: 0 },
  b2: { x: 0, y: 10 },
  style: { color: "#fff", weight: 1, opacity: 0.45 },
};

describe("pxToAxial", () => {
  test("maps the origin to (0,0)", () => {
    const basis = loadHexBasis(GRID)!;
    expect(pxToAxial(basis, { x: 0, y: 0 })).toEqual({ q: 0, r: 0 });
  });

  test("maps a point along b1 to (1,0), and along b2 to (0,1)", () => {
    const basis = loadHexBasis(GRID)!;
    expect(pxToAxial(basis, { x: 10, y: 0 })).toEqual({ q: 1, r: 0 });
    expect(pxToAxial(basis, { x: 0, y: 10 })).toEqual({ q: 0, r: 1 });
  });

  test("rounds to the nearest hex", () => {
    const basis = loadHexBasis(GRID)!;
    expect(pxToAxial(basis, { x: 24, y: 4 })).toEqual({ q: 2, r: 0 });
  });
});

describe("loadHexBasis", () => {
  test("returns null for degenerate (parallel) basis vectors", () => {
    const degenerate: HexGrid = { ...GRID, b1: { x: 10, y: 0 }, b2: { x: 20, y: 0 } };
    expect(loadHexBasis(degenerate)).toBeNull();
  });
});

describe("hexDistance", () => {
  test("is 0 for the same point", () => {
    const basis = loadHexBasis(GRID)!;
    expect(hexDistance(basis, { x: 5, y: 5 }, { x: 5, y: 5 })).toBe(0);
  });

  test("is 1 between adjacent hexes", () => {
    const basis = loadHexBasis(GRID)!;
    expect(hexDistance(basis, { x: 0, y: 0 }, { x: 10, y: 0 })).toBe(1);
  });

  test("matches the cube-distance formula for a non-adjacent pair", () => {
    const basis = loadHexBasis(GRID)!;
    // (0,0) -> (2,2): (|dq| + |dq+dr| + |dr|) / 2 = (2 + 4 + 2) / 2 = 4
    expect(hexDistance(basis, { x: 0, y: 0 }, { x: 20, y: 20 })).toBe(4);
  });
});

describe("hexCorners", () => {
  test("produces 6 points, each at radius |b1|/sqrt(3) from the center", () => {
    const center = { x: 0, y: 0 };
    const corners = hexCorners(center, GRID.b1);
    expect(corners).toHaveLength(6);
    const expectedRadius = 10 / Math.sqrt(3);
    for (const p of corners) {
      expect(Math.hypot(p.x - center.x, p.y - center.y)).toBeCloseTo(expectedRadius, 10);
    }
  });

  test("starts 30 degrees off b1's angle", () => {
    const corners = hexCorners({ x: 0, y: 0 }, GRID.b1);
    // b1 points along +x (angle 0), so the first corner sits at +30deg —
    // r*cos(30deg) resolves to exactly half of |b1| here.
    expect(corners[0]!.x).toBeCloseTo(5, 10);
    expect(corners[0]!.y).toBeCloseTo(10 / Math.sqrt(3) / 2, 10);
  });
});

describe("buildHexPolygons", () => {
  test("covers a small image with 6-cornered polygons", () => {
    const basis = loadHexBasis(GRID)!;
    const polygons = buildHexPolygons(basis, 30, 30);
    expect(polygons.length).toBeGreaterThan(0);
    for (const polygon of polygons) {
      expect(polygon).toHaveLength(6);
    }
  });

  test("returns nothing for a grid with no valid basis", () => {
    const degenerate: HexGrid = { ...GRID, b1: { x: 10, y: 0 }, b2: { x: 20, y: 0 } };
    const basis = loadHexBasis(degenerate);
    expect(basis).toBeNull();
  });
});
