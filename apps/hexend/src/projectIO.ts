import { readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import {
  parseHexenProject,
  serializeHexenProject,
  type HexenProject,
  type InlineLocationContent,
} from "@hex-enductor/hexen-schema";
import { createObsidianResolver } from "@hex-enductor/content-obsidian";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

// Inline content needs no I/O and no resolver package of its own — it's
// already exactly a ResolvedContent, just with the `type` discriminant
// stripped off.
function resolveInlineContent(content: InlineLocationContent): ResolvedContent {
  return { title: content.title, body: content.body };
}

export interface OpenedProject {
  project: HexenProject;
  warnings: string[];
  /** location.id -> resolved content, for every location whose content resolved cleanly. */
  resolvedContent: Record<string, ResolvedContent>;
  /** location.id -> the error message, for every location whose content failed to resolve (e.g. a moved/renamed vault file). */
  resolveErrors: Record<string, string>;
}

export async function openProject(path: string): Promise<OpenedProject> {
  const yamlText = await readFile(path, "utf-8");
  const { project, warnings } = parseHexenProject(yamlText);

  const resolvedContent: Record<string, ResolvedContent> = {};
  const resolveErrors: Record<string, string> = {};

  // A Location's own content.type picks how it resolves, independent of
  // the project's — an obsidian-backed project can still hold inline
  // locations that don't warrant a vault file of their own.
  const obsidianResolver =
    project.content.type === "obsidian"
      ? createObsidianResolver({ projectDir: dirname(path), vaultRoot: project.content.vaultRoot })
      : null;

  await Promise.all(
    project.locations
      .filter((location) => location.content !== null)
      .map(async (location) => {
        // location.content is narrowed non-null by the filter above
        const content = location.content!;
        try {
          if (content.type === "inline") {
            resolvedContent[location.id] = resolveInlineContent(content);
          } else if (obsidianResolver) {
            resolvedContent[location.id] = await obsidianResolver.resolve(content);
          } else {
            throw new Error(`Location "${location.id}" has obsidian content, but this project has no vault configured`);
          }
        } catch (err) {
          resolveErrors[location.id] = err instanceof Error ? err.message : String(err);
        }
      }),
  );

  return { project, warnings, resolvedContent, resolveErrors };
}

export async function saveProject(path: string, project: HexenProject): Promise<void> {
  const yamlText = serializeHexenProject(project);
  await writeFile(path, yamlText, "utf-8");
}
