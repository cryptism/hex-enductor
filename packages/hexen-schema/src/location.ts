import { z } from "zod";
import { GridSchema } from "./grid.ts";
import { LocationContentSchema } from "./content.ts";

export const ImageRefSchema = z
  .object({
    file: z
      .string()
      .describe(
        "Path to the base map image, resolved relative to the .hexen.yml file's own directory (not content.vaultRoot) — image storage is decoupled from wherever the content plugin's root is.",
      ),
    width: z.number().int().positive(),
    height: z.number().int().positive(),
  })
  .describe("A Location's own base map image, if it has one.");
export type ImageRef = z.infer<typeof ImageRefSchema>;

export const LinkSchema = z
  .object({
    id: z
      .string()
      .describe(
        "This pin's own identity, stable across edits and independent of what it points at — so a Location can be the target of more than one Link (e.g. a front and back door into the same building).",
      ),
    target: z.string().describe("The Location this pin points at."),
    x: z.number(),
    y: z.number(),
    type: z.string().describe("Marker type, e.g. settlement/ruin/landmark/hazard/waypoint."),
    icon: z.string().optional(),
    color: z.string().nullable().default(null),
    hidden: z.boolean().default(false),
  })
  .describe(
    "A pin on a parent Location's map: position and display only, never a title or summary — those live on the target Location's own content. See parseHexenProject in ./index.ts for how a dangling target is handled.",
  );
export type Link = z.infer<typeof LinkSchema>;

export const LocationSchema = z
  .object({
    id: z.string(),
    grid: GridSchema.nullable().default(null),
    image: ImageRefSchema.nullable().default(null),
    content: LocationContentSchema.nullable().default(null),
    links: z.array(LinkSchema).default([]),
  })
  .describe(
    "The one node type in the format. A Location becomes a map you can click into purely by having a grid, and a pin on someone else's map purely by being the target of a Link elsewhere — both, either, or neither can be true of the same Location at once.",
  );
export type Location = z.infer<typeof LocationSchema>;
