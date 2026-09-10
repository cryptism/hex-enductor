import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { appRouter } from "./router.ts";

let projectDir: string;
const caller = appRouter.createCaller({});

async function writeProject(yamlText: string): Promise<string> {
  const path = join(projectDir, "project.hexen.yml");
  await writeFile(path, yamlText, "utf-8");
  return path;
}

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
      locations: [{ id: "town", grid: null, image: null, content: null, links: [] }],
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

describe("addLocationLink", () => {
  test("creates a new Location and links to it when the id doesn't exist yet", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
`);
    const result = await caller.addLocationLink({
      path,
      parentLocationId: "town",
      locationId: "old-mill",
      x: 12,
      y: 34,
      type: "landmark",
    });

    const town = result.project.locations.find((l) => l.id === "town")!;
    expect(town.links).toEqual([{ id: "old-mill", x: 12, y: 34, type: "landmark", color: null, hidden: false }]);

    const mill = result.project.locations.find((l) => l.id === "old-mill");
    expect(mill).toEqual({ id: "old-mill", grid: null, image: null, content: null, links: [] });
  });

  test("links to an existing Location instead of duplicating it", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
  - id: inn
    content: { type: inline, title: The Inn, body: "" }
`);
    const result = await caller.addLocationLink({
      path,
      parentLocationId: "town",
      locationId: "inn",
      x: 5,
      y: 5,
      type: "settlement",
    });

    expect(result.project.locations.filter((l) => l.id === "inn")).toHaveLength(1);
    const town = result.project.locations.find((l) => l.id === "town")!;
    expect(town.links.map((l) => l.id)).toEqual(["inn"]);
  });

  test("refuses a second link to the same target from the same parent", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
    links:
      - id: inn
        x: 1
        y: 1
        type: settlement
  - id: inn
`);
    await expect(
      caller.addLocationLink({ path, parentLocationId: "town", locationId: "inn", x: 9, y: 9, type: "settlement" }),
    ).rejects.toThrow(/already has a link/);
  });

  test("rejects an unknown parent location", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
`);
    await expect(
      caller.addLocationLink({ path, parentLocationId: "nowhere", locationId: "inn", x: 0, y: 0, type: "settlement" }),
    ).rejects.toThrow(/No location "nowhere"/);
  });
});

describe("saveGrid", () => {
  test("sets a grid on a location that had none", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
`);
    const grid = {
      type: "hex" as const,
      origin: { x: 200, y: 200 },
      b1: { x: 60, y: 0 },
      b2: { x: 30, y: 52 },
      style: { color: "#c19a5f", weight: 1, opacity: 0.45 },
    };
    const result = await caller.saveGrid({ path, locationId: "town", grid });

    expect(result.project.locations[0]!.grid).toEqual(grid);
  });

  test("clears a grid back to null", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
    grid:
      type: square
      origin: { x: 0, y: 0 }
      cellSize: { x: 32, y: 32 }
      style: { color: "#fff" }
`);
    const result = await caller.saveGrid({ path, locationId: "town", grid: null });

    expect(result.project.locations[0]!.grid).toBeNull();
  });

  test("rejects an unknown location", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
`);
    await expect(caller.saveGrid({ path, locationId: "nowhere", grid: null })).rejects.toThrow(
      /No location "nowhere"/,
    );
  });
});

describe("saveImage", () => {
  test("sets an image on a location that had none", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
`);
    const image = { file: "_assets/town.png", width: 400, height: 300 };
    const result = await caller.saveImage({ path, locationId: "town", image });

    expect(result.project.locations[0]!.image).toEqual(image);
  });

  test("clears an image back to null", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    image: { file: _assets/town.png, width: 400, height: 300 }
`);
    const result = await caller.saveImage({ path, locationId: "town", image: null });

    expect(result.project.locations[0]!.image).toBeNull();
  });

  test("rejects an unknown location", async () => {
    const path = await writeProject(`
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
`);
    await expect(caller.saveImage({ path, locationId: "nowhere", image: null })).rejects.toThrow(
      /No location "nowhere"/,
    );
  });
});
