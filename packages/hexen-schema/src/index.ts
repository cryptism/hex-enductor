import { parse as parseYaml, stringify as stringifyYaml } from "yaml";
import { HexenProjectSchema, type HexenProject } from "./project.ts";

export * from "./grid.ts";
export * from "./content.ts";
export * from "./location.ts";
export * from "./project.ts";

export interface ParsedHexenProject {
  project: HexenProject;
  /**
   * Non-fatal problems found after shape validation passed — dangling
   * links, an unresolvable defaultLocation, duplicate ids. Deliberately
   * warn-don't-fail (docs/PLAN.md §9): each Location's own shape is
   * still valid Zod on its own, so one bad reference shouldn't sink the
   * whole project.
   */
  warnings: string[];
}

export function parseHexenProject(yamlText: string): ParsedHexenProject {
  const raw: unknown = parseYaml(yamlText);
  const project = HexenProjectSchema.parse(raw);
  const warnings = validateReferences(project);
  return { project, warnings };
}

export function serializeHexenProject(project: HexenProject): string {
  // Re-validate on the way out too — a hand-built object (e.g. from a
  // form) shouldn't be able to write invalid YAML to disk.
  const validated = HexenProjectSchema.parse(project);
  return stringifyYaml(validated);
}

function validateReferences(project: HexenProject): string[] {
  const warnings: string[] = [];
  const seenIds = new Set<string>();

  for (const location of project.locations) {
    if (seenIds.has(location.id)) {
      warnings.push(`Duplicate location id "${location.id}"`);
    }
    seenIds.add(location.id);
  }

  if (!seenIds.has(project.defaultLocation)) {
    warnings.push(`defaultLocation "${project.defaultLocation}" isn't a known location id`);
  }

  for (const location of project.locations) {
    for (const link of location.links) {
      if (!seenIds.has(link.id)) {
        warnings.push(
          `Location "${location.id}" links to unknown location id "${link.id}"`,
        );
      }
    }
  }

  return warnings;
}
