import { describe, expect, test } from "bun:test";
import type { Grid, LocationContent, ProjectContent } from "@hex-enductor/hexen-schema";
import {
  commandToWire,
  gridFromWire,
  gridToWire,
  locationContentFromWire,
  locationContentToWire,
  openedProjectDataFromWire,
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
    expect(commandToWire({ type: "toggleFogCell", locationId: "town", cell: "2,3" })).toEqual({
      toggleFogCell: { locationId: "town", cell: "2,3" },
    });
    expect(commandToWire({ type: "setFog", locationId: "town", fog: null })).toEqual({
      setFog: { locationId: "town", fog: null },
    });
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
