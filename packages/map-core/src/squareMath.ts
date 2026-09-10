import type { Point, SquareGrid } from "@hex-enductor/hexen-schema";

/** The four corners of cell (i, j), where origin is cell (0, 0)'s top-left corner. */
export function squareCorners(origin: Point, cellSize: Point, i: number, j: number): Point[] {
  const x0 = origin.x + i * cellSize.x;
  const y0 = origin.y + j * cellSize.y;
  const x1 = x0 + cellSize.x;
  const y1 = y0 + cellSize.y;
  return [
    { x: x0, y: y0 },
    { x: x1, y: y0 },
    { x: x1, y: y1 },
    { x: x0, y: y1 },
  ];
}

/** Every square-cell polygon (as pixel-space corner arrays) that overlaps the image bounds, one cell of margin either side. */
export function buildSquarePolygons(grid: SquareGrid, imageWidth: number, imageHeight: number): Point[][] {
  const { origin, cellSize } = grid;
  if (cellSize.x === 0 || cellSize.y === 0) return [];

  const iMin = Math.floor(-origin.x / cellSize.x) - 1;
  const iMax = Math.ceil((imageWidth - origin.x) / cellSize.x) + 1;
  const jMin = Math.floor(-origin.y / cellSize.y) - 1;
  const jMax = Math.ceil((imageHeight - origin.y) / cellSize.y) + 1;

  const polygons: Point[][] = [];
  for (let i = Math.min(iMin, iMax); i <= Math.max(iMin, iMax); i++) {
    for (let j = Math.min(jMin, jMax); j <= Math.max(jMin, jMax); j++) {
      polygons.push(squareCorners(origin, cellSize, i, j));
    }
  }
  return polygons;
}
