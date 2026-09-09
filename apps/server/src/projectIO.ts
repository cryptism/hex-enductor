import { readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { parseHexenProject, serializeHexenProject, type HexenProject } from "@hex-enductor/hexen-schema";
import { createObsidianResolver } from "@hex-enductor/content-obsidian";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

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

  if (project.content.type === "obsidian") {
    const resolver = createObsidianResolver({
      projectDir: dirname(path),
      vaultRoot: project.content.vaultRoot,
    });

    await Promise.all(
      project.locations
        .filter((location) => location.content !== null)
        .map(async (location) => {
          try {
            // location.content is narrowed non-null by the filter above
            resolvedContent[location.id] = await resolver.resolve(location.content!);
          } catch (err) {
            resolveErrors[location.id] = err instanceof Error ? err.message : String(err);
          }
        }),
    );
  }

  return { project, warnings, resolvedContent, resolveErrors };
}

export async function saveProject(path: string, project: HexenProject): Promise<void> {
  const yamlText = serializeHexenProject(project);
  await writeFile(path, yamlText, "utf-8");
}
