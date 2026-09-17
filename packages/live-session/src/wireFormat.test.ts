import { describe, expect, test } from "bun:test";
import type { Grid, HexenProject, LocationContent, ProjectContent } from "@hex-enductor/hexen-schema";
import {
  commandFromWire,
  commandToWire,
  gridFromWire,
  gridToWire,
  locationContentFromWire,
  locationContentToWire,
  openedProjectDataFromWire,
  openedProjectDataToWire,
  projectContentFromWire,
  projectContentToWire,
} from "./wireFormat.ts";

const HEX_GRID: Grid = {
  type: "hex",
  origin: { x: 0, y: 0 },
  b1: { x: 10, y: 0 },
  b2: { x: 5, y: 8 },
  style: { color: "#fff", weight: 1, opacity: 0.5 },
};

describe("grid wire conversion", () => {
  test("round-trips a hex grid", () => {
    const wire = gridToWire(HEX_GRID);
    expect(wire).toEqual({ hex: { origin: HEX_GRID.origin, b1: HEX_GRID.b1, b2: HEX_GRID.b2, style: HEX_GRID.style } });
    expect(gridFromWire(wire)).toEqual(HEX_GRID);
  });

  test("round-trips null as null", () => {
    expect(gridToWire(null)).toBeNull();
    expect(gridFromWire(null)).toBeNull();
    expect(gridFromWire(undefined)).toBeNull();
  });
});

describe("project content wire conversion", () => {
  test("round-trips obsidian content", () => {
    const content: ProjectContent = { type: "obsidian", vaultRoot: "_vault" };
    const wire = projectContentToWire(content);
    expect(wire).toEqual({ obsidian: { vaultRoot: "_vault" } });
    expect(projectContentFromWire(wire)).toEqual(content);
  });

  test("round-trips inline content", () => {
    const content: ProjectContent = { type: "inline" };
    expect(projectContentFromWire(projectContentToWire(content))).toEqual(content);
  });
});

describe("location content wire conversion", () => {
  test("round-trips obsidian content", () => {
    const content: LocationContent = { type: "obsidian", ref: "town.md" };
    expect(locationContentFromWire(locationContentToWire(content))).toEqual(content);
  });

  test("round-trips inline content, defaulting a missing body to empty", () => {
    const content: LocationContent = { type: "inline", title: "The Town", body: "Hello." };
    expect(locationContentFromWire(locationContentToWire(content))).toEqual(content);
    expect(locationContentFromWire({ inline: { title: "The Town" } })).toEqual({
      type: "inline",
      title: "The Town",
      body: "",
    });
  });

  test("round-trips null as null", () => {
    expect(locationContentToWire(null)).toBeNull();
    expect(locationContentFromWire(null)).toBeNull();
  });
});

describe("commandToWire", () => {
  test("wraps a saveGrid command's grid", () => {
    expect(commandToWire({ type: "saveGrid", locationId: "town", grid: HEX_GRID })).toEqual({
      saveGrid: { locationId: "town", grid: { hex: { origin: HEX_GRID.origin, b1: HEX_GRID.b1, b2: HEX_GRID.b2, style: HEX_GRID.style } } },
    });
  });

  test("passes plain-field commands through untouched", () => {
    expect(commandToWire({ type: "setFogCells", locationId: "town", cells: ["2,3", "2,4"], revealed: true })).toEqual({
      setFogCells: { locationId: "town", cells: ["2,3", "2,4"], revealed: true },
    });
    expect(commandToWire({ type: "setFog", locationId: "town", fog: null })).toEqual({
      setFog: { locationId: "town", fog: null },
    });
  });
});

describe("commandFromWire", () => {
  test("is the inverse of commandToWire for every command variant", () => {
    const commands: Parameters<typeof commandToWire>[0][] = [
      { type: "saveLink", locationId: "town", linkId: "front-door", patch: { hidden: true } },
      { type: "saveLocationContent", locationId: "town", patch: { body: "Updated." } },
      { type: "addLocationLink", parentLocationId: "town", targetLocationId: "old-mill", x: 1, y: 2, linkType: "landmark" },
      { type: "saveGrid", locationId: "town", grid: HEX_GRID },
      { type: "saveGrid", locationId: "town", grid: null },
      { type: "saveImage", locationId: "town", image: { file: "_assets/town.png", width: 400, height: 400 } },
      { type: "setFog", locationId: "town", fog: { revealedCells: ["0,0"] } },
      { type: "setFogCells", locationId: "town", cells: ["2,3", "2,4"], revealed: false },
    ];
    for (const command of commands) {
      expect(commandFromWire(commandToWire(command) as Record<string, any>)).toEqual(command);
    }
  });

  test("returns null for an unrecognized shape", () => {
    expect(commandFromWire({})).toBeNull();
    expect(commandFromWire({ deleteEverything: {} })).toBeNull();
  });
});

describe("openedProjectDataToWire", () => {
  test("is the inverse of openedProjectDataFromWire for a full project", () => {
    const project: HexenProject = {
      schemaVersion: 1,
      title: "Demo",
      defaultLocation: "town",
      content: { type: "obsidian", vaultRoot: "_vault" },
      locations: [
        {
          id: "town",
          grid: HEX_GRID,
          image: { file: "_assets/town.png", width: 400, height: 400 },
          content: { type: "inline", title: "The Town", body: "Hello." },
          links: [{ id: "front-door", target: "inn", x: 1, y: 1, type: "settlement", color: null, hidden: false }],
          fog: { revealedCells: ["0,0"] },
        },
        { id: "inn", grid: null, image: null, content: null, links: [], fog: null },
      ],
    };
    const data = { project, warnings: ["a warning"], resolvedContent: {}, resolveErrors: {} };
    expect(openedProjectDataFromWire(openedProjectDataToWire(data) as any)).toEqual(data);
  });
});

describe("openedProjectDataFromWire", () => {
  test("converts a full project, defaulting absent collections", () => {
    const data = openedProjectDataFromWire({
      project: {
        schemaVersion: 1,
        title: "Demo",
        defaultLocation: "town",
        content: { inline: {} },
        locations: [
          {
            id: "town",
            grid: {
              hex: { origin: { x: 0, y: 0 }, b1: { x: 1, y: 0 }, b2: { x: 0, y: 1 }, style: { color: "#fff", weight: 1, opacity: 0.45 } },
            },
            content: { inline: { title: "The Town" } },
            fog: { revealedCells: ["0,0"] },
          },
        ],
      },
    });

    expect(data.warnings).toEqual([]);
    expect(data.resolvedContent).toEqual({});
    expect(data.project.locations[0]).toEqual({
      id: "town",
      grid: {
        type: "hex",
        origin: { x: 0, y: 0 },
        b1: { x: 1, y: 0 },
        b2: { x: 0, y: 1 },
        style: { color: "#fff", weight: 1, opacity: 0.45 },
      },
      image: null,
      content: { type: "inline", title: "The Town", body: "" },
      links: [],
      fog: { revealedCells: ["0,0"] },
    });
  });
});
