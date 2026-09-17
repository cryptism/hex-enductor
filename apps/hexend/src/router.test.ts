import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { appRouter } from "./router.ts";

let projectDir: string;
const caller = appRouter.createCaller({});

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "router-test-"));
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

describe("createProject", () => {
  test("writes a minimal, valid inline-content project and opens it", async () => {
    const path = join(projectDir, "new-realm.hexen.yml");
    const result = await caller.createProject({ path, title: "New Realm", defaultLocationId: "town" });

    expect(result.project).toEqual({
      schemaVersion: 1,
      title: "New Realm",
      defaultLocation: "town",
      content: { type: "inline" },
      locations: [{ id: "town", grid: null, image: null, content: null, links: [], fog: null }],
    });
    expect(result.warnings).toEqual([]);
  });

  test("refuses to overwrite an existing file", async () => {
    const path = join(projectDir, "already-there.hexen.yml");
    await caller.createProject({ path, title: "First", defaultLocationId: "town" });

    await expect(caller.createProject({ path, title: "Second", defaultLocationId: "town" })).rejects.toThrow(
      /already exists/,
    );
  });

  test("rejects a save path whose directory doesn't exist", async () => {
    const path = join(projectDir, "nowhere", "project.hexen.yml");
    await expect(caller.createProject({ path, title: "New Realm", defaultLocationId: "town" })).rejects.toThrow(
      /doesn't exist/,
    );
  });
});
