import { z } from "zod";

export const PointSchema = z.object({
  x: z.number(),
  y: z.number(),
});
export type Point = z.infer<typeof PointSchema>;

export const GridStyleSchema = z.object({
  color: z.string(),
  weight: z.number().default(1),
  opacity: z.number().min(0).max(1).default(0.45),
});
export type GridStyle = z.infer<typeof GridStyleSchema>;

// A hex grid is calibrated by an origin pixel plus two basis vectors
// (b1, b2) spanning the lattice — b1/b2 point from one hex's centre to
// two of its neighbours. See packages/map-core/src/hexMath.ts for the
// math this calibration feeds (ported from the current
// illuminated-world/site/maps/lib/app.js).
export const HexGridSchema = z.object({
  type: z.literal("hex"),
  origin: PointSchema,
  b1: PointSchema,
  b2: PointSchema,
  distancePerCell: z.number().positive().optional(),
  style: GridStyleSchema,
});
export type HexGrid = z.infer<typeof HexGridSchema>;

// Square grids are docs/ROADMAP.md's near-term priority, not built yet
// (packages/map-core has no renderer for this variant) — the shape
// exists now so adding one later is additive, not a schema rewrite.
export const SquareGridSchema = z.object({
  type: z.literal("square"),
  origin: PointSchema,
  cellSize: PointSchema,
  distancePerCell: z.number().positive().optional(),
  style: GridStyleSchema,
});
export type SquareGrid = z.infer<typeof SquareGridSchema>;

export const GridSchema = z.discriminatedUnion("type", [HexGridSchema, SquareGridSchema]);
export type Grid = z.infer<typeof GridSchema>;
