import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openProject } from "./projectIO.ts";

let projectDir: string;

async function writeProject(yamlText: string): Promise<string> {
  const path = join(projectDir, "project.hexen.yml");
  await writeFile(path, yamlText, "utf-8");
  return path;
}

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "project-io-test-"));
  await mkdir(join(projectDir, "_vault"), { recursive: true });
  await writeFile(join(projectDir, "_vault", "town.md"), "---\ntitle: The Town\n---\nA quiet crossroads.\n");
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

describe("openProject", () => {
  test("resolves inline content directly, no vault required", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: A quiet crossroads. }
`);
    const { resolvedContent, resolveErrors } = await openProject(path);
    expect(resolvedContent.town).toEqual({ title: "The Town", body: "A quiet crossroads." });
    expect(resolveErrors).toEqual({});
  });

  test("resolves obsidian content when a vault is configured", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    content: { type: obsidian, ref: town.md }
`);
    const { resolvedContent, resolveErrors } = await openProject(path);
    expect(resolvedContent.town?.title).toBe("The Town");
    expect(resolveErrors).toEqual({});
  });

  test("resolves inline and obsidian locations side by side in one project", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    content: { type: obsidian, ref: town.md }
  - id: side-note
    content: { type: inline, title: Side Note, body: Not worth its own vault file. }
`);
    const { resolvedContent, resolveErrors } = await openProject(path);
    expect(resolvedContent.town?.title).toBe("The Town");
    expect(resolvedContent["side-note"]).toEqual({ title: "Side Note", body: "Not worth its own vault file." });
    expect(resolveErrors).toEqual({});
  });

  test("reports a resolveError for obsidian content when the project has no vault", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: obsidian, ref: town.md }
`);
    const { resolvedContent, resolveErrors } = await openProject(path);
    expect(resolvedContent.town).toBeUndefined();
    expect(resolveErrors.town).toMatch(/no vault configured/);
  });
});
