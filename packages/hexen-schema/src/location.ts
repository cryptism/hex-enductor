import { z } from "zod";
import { GridSchema } from "./grid.ts";
import { LocationContentSchema } from "./content.ts";

export const ImageRefSchema = z.object({
  // Resolved relative to the .hexen.yml file's own directory, not
  // content.vaultRoot — see docs/PLAN.md §6/§9 on why image storage is
  // deliberately decoupled from wherever the content plugin's root is.
  file: z.string(),
  width: z.number().int().positive(),
  height: z.number().int().positive(),
});
export type ImageRef = z.infer<typeof ImageRefSchema>;

// A pin on a parent location's map — position/display only. Never a
// title or summary: those live on the referenced Location's own
// content (docs/PLAN.md §6). `id` isn't required to resolve to a real
// Location at parse time (docs/PLAN.md §9: referential integrity is
// deliberately unspecified for now) — see validateProject in ./index.ts
// for the "warn, don't fail the whole load" pass that checks it.
export const LinkSchema = z.object({
  id: z.string(),
  x: z.number(),
  y: z.number(),
  type: z.string(),
  icon: z.string().optional(),
  color: z.string().nullable().default(null),
  hidden: z.boolean().default(false),
});
export type Link = z.infer<typeof LinkSchema>;

// The one node type in the format (docs/PLAN.md §6). A Location becomes
// a map you can click into purely by having a `grid`; it becomes a pin
// on someone else's map purely by being the target of a Link elsewhere.
// Both, either, or neither can be true of the same Location at once.
export const LocationSchema = z.object({
  id: z.string(),
  grid: GridSchema.nullable().default(null),
  image: ImageRefSchema.nullable().default(null),
  content: LocationContentSchema.nullable().default(null),
  links: z.array(LinkSchema).default([]),
});
export type Location = z.infer<typeof LocationSchema>;
