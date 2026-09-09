import { readdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { homedir } from "node:os";

export interface DirEntry {
  name: string;
  isDirectory: boolean;
  isProject: boolean;
}

export interface DirectoryListing {
  path: string;
  parent: string | null;
  entries: DirEntry[];
}

// Same trust model as the rest of the server (see server.ts) — no auth,
// no jail. This just gives the editor something nicer than "paste an
// absolute path" to open a project with.
export async function listDirectory(path?: string): Promise<DirectoryListing> {
  const target = resolve(path && path.trim().length > 0 ? path : homedir());
  const dirents = await readdir(target, { withFileTypes: true });

  const entries: DirEntry[] = dirents
    .filter((d) => !d.name.startsWith("."))
    .filter((d) => d.isDirectory() || d.name.endsWith(".hexen.yml"))
    .map((d) => ({
      name: d.name,
      isDirectory: d.isDirectory(),
      isProject: d.name.endsWith(".hexen.yml"),
    }))
    .sort((a, b) => (a.isDirectory === b.isDirectory ? a.name.localeCompare(b.name) : a.isDirectory ? -1 : 1));

  const parent = dirname(target);

  return {
    path: target,
    parent: parent === target ? null : parent,
    entries,
  };
}
