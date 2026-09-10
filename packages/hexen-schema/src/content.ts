import { z } from "zod";

/**
 * Every content block carries an explicit `type`, matching the same
 * discriminated-union convention `Grid` uses. Obsidian resolves against
 * an external vault; inline needs nothing external at all — a third
 * backend later is a third variant here, not a fork of the format.
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

export const InlineProjectContentSchema = z
  .object({ type: z.literal("inline") })
  .describe(
    "A project's content source: no external vault — each Location carries its own title/body directly in this file.",
  );
export type InlineProjectContent = z.infer<typeof InlineProjectContentSchema>;

export const ProjectContentSchema = z
  .discriminatedUnion("type", [ObsidianProjectContentSchema, InlineProjectContentSchema])
  .describe("Where a project's location content (title/summary/body) is resolved from.");
export type ProjectContent = z.infer<typeof ProjectContentSchema>;

export const ObsidianLocationContentSchema = z
  .object({
    type: z.literal("obsidian"),
    ref: z.string().describe("Path to the markdown file, relative to the project's content.vaultRoot."),
  })
  .describe("A single location's content: one file in the project's Obsidian vault.");
export type ObsidianLocationContent = z.infer<typeof ObsidianLocationContentSchema>;

export const InlineLocationContentSchema = z
  .object({
    type: z.literal("inline"),
    title: z.string(),
    body: z.string().default(""),
  })
  .describe("A single location's content, written directly into the .hexen.yml — no vault file.");
export type InlineLocationContent = z.infer<typeof InlineLocationContentSchema>;

export const LocationContentSchema = z
  .discriminatedUnion("type", [ObsidianLocationContentSchema, InlineLocationContentSchema])
  .describe(
    "A Location's title/summary/body always come from here, resolved at load time — never duplicated elsewhere. Inline content is the one exception to 'never duplicated into .hexen.yml': there is nowhere else for it to live.",
  );
export type LocationContent = z.infer<typeof LocationContentSchema>;
