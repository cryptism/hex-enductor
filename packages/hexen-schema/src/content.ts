import { z } from "zod";

/**
 * Every content block carries an explicit `type`, matching the same
 * discriminated-union convention `Grid` uses — a second backend later
 * (see @hex-enductor/content-resolver) is a second variant here, not a
 * fork of the format. Obsidian is the only one that exists right now.
 */
export const ObsidianProjectContentSchema = z
  .object({
    type: z.literal("obsidian"),
    vaultRoot: z
      .string()
      .describe(
        "Path to the Obsidian vault, resolved relative to the .hexen.yml file's own directory.",
      ),
  })
  .describe("A project's content source: an Obsidian vault.");
export type ObsidianProjectContent = z.infer<typeof ObsidianProjectContentSchema>;

export const ProjectContentSchema = z
  .discriminatedUnion("type", [ObsidianProjectContentSchema])
  .describe("Where a project's location content (title/summary/body) is resolved from.");
export type ProjectContent = z.infer<typeof ProjectContentSchema>;

export const ObsidianLocationContentSchema = z
  .object({
    type: z.literal("obsidian"),
    ref: z.string().describe("Path to the markdown file, relative to the project's content.vaultRoot."),
  })
  .describe("A single location's content: one file in the project's Obsidian vault.");
export type ObsidianLocationContent = z.infer<typeof ObsidianLocationContentSchema>;

export const LocationContentSchema = z
  .discriminatedUnion("type", [ObsidianLocationContentSchema])
  .describe(
    "A Location's title/summary/body always come from here, resolved at load time — never duplicated into .hexen.yml.",
  );
export type LocationContent = z.infer<typeof LocationContentSchema>;
