import type { Point } from "@hex-enductor/hexen-schema";

/**
 * Fog of war is addressed by its own uniform pixel grid over the base
 * image — independent of whatever terrain grid (hex/square/none) the
 * Location has, since a map without a terrain grid can still have fog.
 * Cell keys ("col,row") are opaque to the server; only this module and
 * MapCanvas need to know what they mean.
 */
export const FOG_CELL_SIZE = 64;

export interface FogGridDims {
  cols: number;
  rows: number;
}

export function fogGridDims(image: { width: number; height: number }): FogGridDims {
  return {
    cols: Math.max(1, Math.ceil(image.width / FOG_CELL_SIZE)),
    rows: Math.max(1, Math.ceil(image.height / FOG_CELL_SIZE)),
  };
}

export function fogCellKey(col: number, row: number): string {
  return `${col},${row}`;
}

/** Which fog cell a pixel-space point falls in. */
export function fogCellAt(point: Point): string {
  return fogCellKey(Math.floor(point.x / FOG_CELL_SIZE), Math.floor(point.y / FOG_CELL_SIZE));
}

/** The four pixel-space corners of one fog cell, clamped to the image bounds. */
export function fogCellCorners(col: number, row: number, image: { width: number; height: number }): Point[] {
  const x0 = col * FOG_CELL_SIZE;
  const y0 = row * FOG_CELL_SIZE;
  const x1 = Math.min(x0 + FOG_CELL_SIZE, image.width);
  const y1 = Math.min(y0 + FOG_CELL_SIZE, image.height);
  return [
    { x: x0, y: y0 },
    { x: x1, y: y0 },
    { x: x1, y: y1 },
    { x: x0, y: y1 },
  ];
}

/** Every cell key covering the image that isn't in revealedCells. */
export function hiddenFogCells(image: { width: number; height: number }, revealedCells: readonly string[]): string[] {
  const revealed = new Set(revealedCells);
  const { cols, rows } = fogGridDims(image);
  const hidden: string[] = [];
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const key = fogCellKey(col, row);
      if (!revealed.has(key)) hidden.push(key);
    }
  }
  return hidden;
}
