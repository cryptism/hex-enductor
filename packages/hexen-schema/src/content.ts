import { z } from "zod";

// Every content block carries an explicit `type` (docs/PLAN.md §6a/§9),
// matching the same discriminated-union convention Grid uses — a
// second backend later is a second variant here, not a fork of the
// format. Obsidian is the only one that exists right now.

export const ObsidianProjectContentSchema = z.object({
  type: z.literal("obsidian"),
  // Resolved relative to the .hexen.yml file's own directory (not a
  // fixed project root) — see docs/PLAN.md §9.
  vaultRoot: z.string(),
});
export type ObsidianProjectContent = z.infer<typeof ObsidianProjectContentSchema>;

export const ProjectContentSchema = z.discriminatedUnion("type", [ObsidianProjectContentSchema]);
export type ProjectContent = z.infer<typeof ProjectContentSchema>;

export const ObsidianLocationContentSchema = z.object({
  type: z.literal("obsidian"),
  // Path to the markdown file, relative to the project's content.vaultRoot.
  ref: z.string(),
});
export type ObsidianLocationContent = z.infer<typeof ObsidianLocationContentSchema>;

export const LocationContentSchema = z.discriminatedUnion("type", [ObsidianLocationContentSchema]);
export type LocationContent = z.infer<typeof LocationContentSchema>;
