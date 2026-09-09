#!/usr/bin/env bun
// Step 2 of the Obsidian migration (see import-obsidian-vault.ts for
// step 1): removes the map-*/map-hex-* frontmatter that's now owned by
// a generated .hexen.yml, from the vault notes it came from. Leaves
// title, summary, tags, and the body of every note completely alone —
// only touches the specific keys buildHexenProject actually reads
// (MAP_ROOT_FIELDS / PIN_FIELDS), by filtering frontmatter lines
// rather than re-serializing the YAML, so nothing else about a note's
// formatting changes.
//
// Run this only after confirming the generated .hexen.yml is correct
// — until whatever reads .hexen.yml is wired up, this will break
// anything (a Quartz build, say) that still depends on this
// frontmatter, on purpose.
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { readVaultEntries } from "./lib/readVaultEntries.ts";
import { MAP_ROOT_FIELDS, PIN_FIELDS } from "./lib/importObsidianVault.ts";
import { stripFrontmatterFields } from "./lib/stripFrontmatterFields.ts";

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
  const vaultDir = args["vault"];
  if (!vaultDir) {
    console.error("Usage: bun run scripts/strip-obsidian-frontmatter.ts --vault <dir> [--dry-run]");
    process.exitCode = 1;
    return;
  }

  const entries = await readVaultEntries(vaultDir);
  let filesChanged = 0;

  for (const entry of entries) {
    const isMapRoot = entry.frontmatter["map-root"] === true;
    const isPin = typeof entry.frontmatter["map"] === "string";
    if (!isMapRoot && !isPin) continue;

    const fields = isMapRoot ? MAP_ROOT_FIELDS : PIN_FIELDS;
    const fullPath = join(vaultDir, entry.relativePath);
    const raw = await readFile(fullPath, "utf-8");
    const { content, removed } = stripFrontmatterFields(raw, fields);

    if (removed.length === 0) continue;
    filesChanged++;
    console.log(`${entry.relativePath}: removed ${removed.join(", ")}`);
    if (!dryRun) await writeFile(fullPath, content, "utf-8");
  }

  console.log(
    dryRun
      ? `Dry run: would change ${filesChanged} file(s).`
      : `Changed ${filesChanged} file(s).`,
  );
}

await main();
