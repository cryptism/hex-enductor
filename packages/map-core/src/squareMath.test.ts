import { describe, expect, test } from "bun:test";
import type { SquareGrid } from "@hex-enductor/hexen-schema";
import { squareCorners, buildSquarePolygons } from "./squareMath.ts";

const GRID: SquareGrid = {
  type: "square",
  origin: { x: 0, y: 0 },
  cellSize: { x: 32, y: 32 },
  style: { color: "#fff", weight: 1, opacity: 0.45 },
};

describe("squareCorners", () => {
  test("cell (0, 0) is the unit square from origin", () => {
    expect(squareCorners({ x: 0, y: 0 }, { x: 32, y: 32 }, 0, 0)).toEqual([
      { x: 0, y: 0 },
      { x: 32, y: 0 },
      { x: 32, y: 32 },
      { x: 0, y: 32 },
    ]);
  });

  test("cell (2, 1) offsets by 2 and 1 cell widths", () => {
    expect(squareCorners({ x: 0, y: 0 }, { x: 32, y: 32 }, 2, 1)).toEqual([
      { x: 64, y: 32 },
      { x: 96, y: 32 },
      { x: 96, y: 64 },
      { x: 64, y: 64 },
    ]);
  });

  test("respects a non-zero origin", () => {
    expect(squareCorners({ x: 10, y: 20 }, { x: 32, y: 32 }, 0, 0)).toEqual([
      { x: 10, y: 20 },
      { x: 42, y: 20 },
      { x: 42, y: 52 },
      { x: 10, y: 52 },
    ]);
  });
});

describe("buildSquarePolygons", () => {
  test("tiles a small image with 4-cornered polygons", () => {
    const polygons = buildSquarePolygons(GRID, 64, 64);
    expect(polygons.length).toBeGreaterThan(0);
    for (const polygon of polygons) {
      expect(polygon).toHaveLength(4);
    }
  });

  test("covers the full image — every pixel corner falls inside some cell's bounds", () => {
    const polygons = buildSquarePolygons(GRID, 100, 100);
    const xs = polygons.flatMap((p) => p.map((c) => c.x));
    const ys = polygons.flatMap((p) => p.map((c) => c.y));
    expect(Math.min(...xs)).toBeLessThanOrEqual(0);
    expect(Math.max(...xs)).toBeGreaterThanOrEqual(100);
    expect(Math.min(...ys)).toBeLessThanOrEqual(0);
    expect(Math.max(...ys)).toBeGreaterThanOrEqual(100);
  });

  test("returns nothing for a degenerate (zero) cell size", () => {
    const degenerate: SquareGrid = { ...GRID, cellSize: { x: 0, y: 32 } };
    expect(buildSquarePolygons(degenerate, 100, 100)).toEqual([]);
  });

  test("still tiles correctly when the origin sits outside the image", () => {
    const offGrid: SquareGrid = { ...GRID, origin: { x: -500, y: -500 } };
    const polygons = buildSquarePolygons(offGrid, 64, 64);
    expect(polygons.length).toBeGreaterThan(0);
  });
});
