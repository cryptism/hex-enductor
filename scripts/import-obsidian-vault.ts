#!/usr/bin/env bun
// One-time (but kept, not thrown away) importer: turns an Obsidian
// vault using illuminated-world's map-root/map-x/map-y frontmatter
// convention into a .hexen.yml project. See README.md's "Migrating
// from Obsidian" section for what this does and doesn't touch.
import { mkdir, copyFile, readFile, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { serializeHexenProject } from "@hex-enductor/hexen-schema";
import { readVaultEntries } from "./lib/readVaultEntries.ts";
import { buildHexenProject, type MapImageRef } from "./lib/importObsidianVault.ts";
import { readPngSize } from "./lib/readPngSize.ts";

function parseArgs(argv: string[]): Record<string, string> {
  const args: Record<string, string> = {};
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i];
    if (!key?.startsWith("--")) throw new Error(`Expected a --flag, got "${key}"`);
    const value = argv[i + 1];
    if (value === undefined) throw new Error(`--${key.slice(2)} needs a value`);
    args[key.slice(2)] = value;
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const vaultDir = args["vault"];
  const outPath = args["out"];
  if (!vaultDir || !outPath) {
    console.error(
      "Usage: bun run scripts/import-obsidian-vault.ts --vault <dir> --out <file.hexen.yml> [--title <title>] [--assets-dir <name>]",
    );
    process.exitCode = 1;
    return;
  }
  const title = args["title"] ?? "Imported project";
  const assetsDirName = args["assets-dir"] ?? "_assets";
  const outDir = dirname(outPath);

  const entries = await readVaultEntries(vaultDir);

  const mapIds = new Set(
    entries
      .filter((e) => e.frontmatter["map-root"] === true)
      .map((e) => e.frontmatter["map-id"] as string),
  );

  const mapImages: Record<string, MapImageRef> = {};
  await mkdir(join(outDir, assetsDirName), { recursive: true });
  for (const mapId of mapIds) {
    // Matches illuminated-world's MapData emitter: the base image always
    // lives at _maps/<id>/map.png, regardless of a map-root note's own
    // map-img frontmatter (which nothing actually reads today).
    const sourcePath = join(vaultDir, "_maps", mapId, "map.png");
    let bytes: Buffer;
    try {
      bytes = await readFile(sourcePath);
    } catch {
      console.warn(`Warning: no image at ${sourcePath} for map "${mapId}" — skipping its image.`);
      continue;
    }
    const size = readPngSize(bytes);
    if (!size) {
      console.warn(`Warning: ${sourcePath} doesn't look like a PNG — skipping its image.`);
      continue;
    }
    const destPath = join(outDir, assetsDirName, `${mapId}.png`);
    await copyFile(sourcePath, destPath);
    mapImages[mapId] = { file: relative(outDir, destPath).replaceAll("\\", "/"), ...size };
  }

  const vaultRoot = relative(outDir, vaultDir).replaceAll("\\", "/") || ".";
  const { project, warnings } = buildHexenProject(entries, { title, vaultRoot, mapImages });

  for (const warning of warnings) console.warn(`Warning: ${warning}`);

  await writeFile(outPath, serializeHexenProject(project), "utf-8");

  const mapCount = project.locations.filter((l) => l.grid !== null).length;
  console.log(
    `Wrote ${outPath}: ${project.locations.length} locations (${mapCount} with their own map).`,
  );
}

await main();
