import { z } from "zod";
import { LocationSchema } from "./location.ts";
import { ProjectContentSchema } from "./content.ts";

export const HexenProjectSchema = z.object({
  schemaVersion: z.literal(1),
  title: z.string(),
  defaultLocation: z.string(),
  content: ProjectContentSchema,
  locations: z.array(LocationSchema),
});
export type HexenProject = z.infer<typeof HexenProjectSchema>;
