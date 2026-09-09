import { z } from "zod";

export const PointSchema = z
  .object({
    x: z.number(),
    y: z.number(),
  })
  .describe("A pixel coordinate in the map image's own coordinate space.");
export type Point = z.infer<typeof PointSchema>;

export const GridStyleSchema = z
  .object({
    color: z.string().describe("CSS color for the grid lines."),
    weight: z.number().default(1).describe("Grid line stroke width, in pixels."),
    opacity: z.number().min(0).max(1).default(0.45),
  })
  .describe("Visual styling for a grid overlay's lines.");
export type GridStyle = z.infer<typeof GridStyleSchema>;

export const HexGridSchema = z
  .object({
    type: z.literal("hex"),
    origin: PointSchema.describe("Pixel position of one hex's centre."),
    b1: PointSchema.describe("Vector from origin to one neighbouring hex's centre."),
    b2: PointSchema.describe(
      "Vector from origin to a second neighbouring hex's centre, not parallel to b1.",
    ),
    distancePerCell: z
      .number()
      .positive()
      .optional()
      .describe("Real-world distance one hex represents (e.g. km), for ruler readouts."),
    style: GridStyleSchema,
  })
  .describe(
    "A hex lattice calibrated by an origin plus two basis vectors — see packages/map-core/src/hexMath.ts for the math this feeds.",
  );
export type HexGrid = z.infer<typeof HexGridSchema>;

export const SquareGridSchema = z
  .object({
    type: z.literal("square"),
    origin: PointSchema.describe("Pixel position of one cell's top-left corner."),
    cellSize: PointSchema.describe("Width and height of one grid cell, in pixels."),
    distancePerCell: z
      .number()
      .positive()
      .optional()
      .describe("Real-world distance one cell represents (e.g. km), for ruler readouts."),
    style: GridStyleSchema,
  })
  .describe("A square lattice.");
export type SquareGrid = z.infer<typeof SquareGridSchema>;

export const GridSchema = z
  .discriminatedUnion("type", [HexGridSchema, SquareGridSchema])
  .describe("How a Location's own map surface is calibrated, if it has one.");
export type Grid = z.infer<typeof GridSchema>;
