#!/usr/bin/env bun
// hexend (the Rust server) parses/writes .hexen.yml in protobuf's own
// JSON/YAML convention — a oneof serializes as `{ <field name>: value
// }`, not the `{ type: "...", ...fields }` shape packages/hexen-schema
// (and this project's browser-native local-fs storage) still use. See
// CLAUDE.md's "A second, un-migrated dialect" note. This script
// converts one project file from the old shape to the new one, in
// place, so it can be opened by hexend.
//
// Only Grid, ProjectContent, and LocationContent are oneof-backed and
// actually change shape — everything else (ids, links, images, fog)
// is already field-for-field identical between the two conventions.
// After this runs, the file can no longer be opened via the browser's
// "Open from this browser…" (File System Access) path — only hexend
// itself parses the new shape — until hexen-schema is migrated too.
import { readFile, writeFile } from "node:fs/promises";
import { stringify as stringifyYaml } from "yaml";
import { parseHexenProject, type Location, type LocationContent } from "@hex-enductor/hexen-schema";
import { gridToWire, projectContentToWire } from "@hex-enductor/live-session";

function locationContentToWire(content: LocationContent | null): object | null {
  if (!content) return null;
  return content.type === "obsidian"
    ? { obsidian: { ref: content.ref } }
    : { inline: { title: content.title, body: content.body } };
}

function locationToWire(location: Location): object {
  return {
    id: location.id,
    grid: gridToWire(location.grid),
    image: location.image,
    content: locationContentToWire(location.content),
    links: location.links,
    fog: location.fog,
  };
}

function parseArgs(argv: string[]) {
  const args: Record<string, string> = {};
  let dryRun = false;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!;
    if (arg === "--dry-run") {
      dryRun = true;
      continue;
    }
    if (!arg.startsWith("--")) throw new Error(`Expected a --flag, got "${arg}"`);
    const value = argv[++i];
    if (value === undefined) throw new Error(`${arg} needs a value`);
    args[arg.slice(2)] = value;
  }
  return { args, dryRun };
}

async function main() {
  const { args, dryRun } = parseArgs(process.argv.slice(2));
  const path = args["project"];
  if (!path) {
    console.error("Usage: bun run scripts/migrate-project-to-wire-format.ts --project <file.hexen.yml> [--dry-run]");
    process.exitCode = 1;
    return;
  }

  const yamlText = await readFile(path, "utf-8");
  const { project, warnings } = parseHexenProject(yamlText);
  if (warnings.length > 0) {
    console.warn(`Warnings while parsing ${path}:`);
    for (const w of warnings) console.warn(`  ${w}`);
  }

  const wireProject = {
    schemaVersion: project.schemaVersion,
    title: project.title,
    defaultLocation: project.defaultLocation,
    content: projectContentToWire(project.content),
    locations: project.locations.map(locationToWire),
  };
  const out = stringifyYaml(wireProject);

  if (dryRun) {
    console.log(out);
    return;
  }
  await writeFile(path, out, "utf-8");
  console.log(`Migrated ${path} (${project.locations.length} location(s)) to the current wire-shaped schema.`);
}

await main();
