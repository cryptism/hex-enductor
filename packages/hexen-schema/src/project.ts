import { z } from "zod";
import { LocationSchema } from "./location.ts";
import { ProjectContentSchema } from "./content.ts";

export const HexenProjectSchema = z
  .object({
    schemaVersion: z.literal(1),
    title: z.string(),
    defaultLocation: z.string().describe("Which Location id opens by default."),
    content: ProjectContentSchema,
    locations: z.array(LocationSchema).describe("Every Location in the project, as a flat list."),
  })
  .describe("A .hexen.yml file: one project, one file.");
export type HexenProject = z.infer<typeof HexenProjectSchema>;
