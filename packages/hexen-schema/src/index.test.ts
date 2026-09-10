import { describe, expect, test } from "bun:test";
import { parseHexenProject, serializeHexenProject, type HexenProject } from "./index.ts";

const MINIMAL_PROJECT_YAML = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content:
  type: obsidian
  vaultRoot: _vault
locations:
  - id: town
    content:
      type: obsidian
      ref: town.md
`;

describe("parseHexenProject", () => {
  test("parses a minimal project and applies defaults", () => {
    const { project, warnings } = parseHexenProject(MINIMAL_PROJECT_YAML);
    expect(warnings).toEqual([]);
    const town = project.locations[0]!;
    expect(town.grid).toBeNull();
    expect(town.image).toBeNull();
    expect(town.links).toEqual([]);
  });

  test("applies Link defaults: hidden false, color null", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    links:
      - id: inn
        x: 1
        y: 2
        type: settlement
`;
    const { project } = parseHexenProject(yaml);
    const link = project.locations[0]!.links[0]!;
    expect(link.hidden).toBe(false);
    expect(link.color).toBeNull();
  });

  test("warns on a duplicate location id rather than throwing", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
  - id: town
`;
    const { warnings } = parseHexenProject(yaml);
    expect(warnings).toContain('Duplicate location id "town"');
  });

  test("warns when defaultLocation isn't a known location id", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: nowhere
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
`;
    const { warnings } = parseHexenProject(yaml);
    expect(warnings).toContain('defaultLocation "nowhere" isn\'t a known location id');
  });

  test("warns on a dangling links[].id instead of failing the whole parse", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    links:
      - id: ghost-town
        x: 0
        y: 0
        type: settlement
`;
    const { project, warnings } = parseHexenProject(yaml);
    // The project itself still parsed — this is the load-bearing part of
    // "warn, don't fail": a dangling reference doesn't sink the file.
    expect(project.locations[0]!.links[0]!.id).toBe("ghost-town");
    expect(warnings).toContain('Location "town" links to unknown location id "ghost-town"');
  });

  test("accepts both hex and square grid variants", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    grid:
      type: hex
      origin: { x: 0, y: 0 }
      b1: { x: 10, y: 0 }
      b2: { x: 5, y: 8 }
      style: { color: "#fff" }
  - id: keep
    grid:
      type: square
      origin: { x: 0, y: 0 }
      cellSize: { x: 32, y: 32 }
      style: { color: "#fff" }
`;
    const { project, warnings } = parseHexenProject(yaml);
    expect(project.locations[0]!.grid?.type).toBe("hex");
    expect(project.locations[1]!.grid?.type).toBe("square");
    expect(warnings).toEqual([]);
  });

  test("accepts a project with no vault, using inline content throughout", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content:
  type: inline
locations:
  - id: town
    content:
      type: inline
      title: The Town
      body: A quiet crossroads.
`;
    const { project, warnings } = parseHexenProject(yaml);
    expect(project.content).toEqual({ type: "inline" });
    expect(project.locations[0]!.content).toEqual({
      type: "inline",
      title: "The Town",
      body: "A quiet crossroads.",
    });
    expect(warnings).toEqual([]);
  });

  test("defaults an inline location's body to an empty string", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: Untitled }
`;
    const { project } = parseHexenProject(yaml);
    expect(project.locations[0]!.content).toEqual({ type: "inline", title: "Untitled", body: "" });
  });

  test("allows inline and obsidian content to mix within one obsidian-backed project", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    content: { type: obsidian, ref: town.md }
  - id: side-note
    content: { type: inline, title: Side Note, body: Not worth its own vault file. }
`;
    const { project, warnings } = parseHexenProject(yaml);
    expect(project.locations[0]!.content).toEqual({ type: "obsidian", ref: "town.md" });
    expect(project.locations[1]!.content?.type).toBe("inline");
    expect(warnings).toEqual([]);
  });

  test("rejects an unrecognized grid type", () => {
    const yaml = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    grid:
      type: triangular
`;
    expect(() => parseHexenProject(yaml)).toThrow();
  });
});

describe("serializeHexenProject", () => {
  test("round-trips through parse", () => {
    const { project } = parseHexenProject(MINIMAL_PROJECT_YAML);
    const reparsed = parseHexenProject(serializeHexenProject(project));
    expect(reparsed.project).toEqual(project);
  });

  test("rejects a hand-built object that doesn't match the schema", () => {
    const invalid = { schemaVersion: 2 } as unknown as HexenProject;
    expect(() => serializeHexenProject(invalid)).toThrow();
  });
});
