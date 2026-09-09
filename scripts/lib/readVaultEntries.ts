import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import matter from "gray-matter";
import type { VaultEntry } from "./importObsidianVault.ts";

const SKIP_DIRS = new Set([".obsidian", ".trash", "node_modules"]);

/** Same convention Quartz's own Explorer uses: an underscore-prefixed folder holds assets, not notes. */
function isHiddenDir(name: string): boolean {
  return name.startsWith("_") || name.startsWith(".") || SKIP_DIRS.has(name);
}

/** Walks a vault directory and parses frontmatter out of every markdown file — Obsidian-asset folders (_maps, _assets, .trash, .obsidian) are skipped entirely, not just ignored for map purposes. */
export async function readVaultEntries(vaultDir: string): Promise<VaultEntry[]> {
  const entries: VaultEntry[] = [];

  async function walk(dir: string) {
    const items = await readdir(dir, { withFileTypes: true });
    for (const item of items) {
      if (item.isDirectory()) {
        if (isHiddenDir(item.name)) continue;
        await walk(join(dir, item.name));
        continue;
      }
      if (!item.name.toLowerCase().endsWith(".md")) continue;

      const fullPath = join(dir, item.name);
      const raw = await readFile(fullPath, "utf-8");
      if (raw.trim().length === 0) continue; // stub notes, e.g. an unwritten wikilink target

      const { data } = matter(raw);
      entries.push({
        // node:path's `relative` uses the platform separator; .hexen.yml paths are always "/".
        relativePath: relative(vaultDir, fullPath).replaceAll("\\", "/"),
        frontmatter: data,
      });
    }
  }

  await walk(vaultDir);
  return entries;
}
