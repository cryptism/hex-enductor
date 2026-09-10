import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir, homedir } from "node:os";
import { dirname, join } from "node:path";
import { listDirectory } from "./browse.ts";

let projectDir: string;

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "browse-test-"));
  await mkdir(join(projectDir, "subdir"), { recursive: true });
  await writeFile(join(projectDir, "campaign.hexen.yml"), "title: Test\n");
  await writeFile(join(projectDir, "notes.txt"), "not a project\n");
  await writeFile(join(projectDir, ".hidden.hexen.yml"), "title: Hidden\n");
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

describe("listDirectory", () => {
  test("lists directories and .hexen.yml files, dirs first", async () => {
    const result = await listDirectory(projectDir);
    expect(result.path).toBe(projectDir);
    expect(result.entries).toEqual([
      { name: "subdir", isDirectory: true, isProject: false },
      { name: "campaign.hexen.yml", isDirectory: false, isProject: true },
    ]);
  });

  test("omits non-project files and dotfiles", () => {
    return listDirectory(projectDir).then((result) => {
      const names = result.entries.map((e) => e.name);
      expect(names).not.toContain("notes.txt");
      expect(names).not.toContain(".hidden.hexen.yml");
    });
  });

  test("reports the parent directory", async () => {
    const result = await listDirectory(projectDir);
    expect(result.parent).toBe(dirname(projectDir));
  });

  test("defaults to the home directory when no path is given", async () => {
    const result = await listDirectory();
    expect(result.path).toBe(homedir());
  });
});
