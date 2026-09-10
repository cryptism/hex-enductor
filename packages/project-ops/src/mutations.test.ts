import { describe, expect, test } from "bun:test";
import type { HexenProject } from "@hex-enductor/hexen-schema";
import {
  createMinimalProject,
  saveLink,
  saveLocationContent,
  addLocationLink,
  saveGrid,
  saveImage,
} from "./mutations.ts";

function fixture(): HexenProject {
  return {
    schemaVersion: 1,
    title: "Test Realm",
    defaultLocation: "town",
    content: { type: "inline" },
    locations: [
      {
        id: "town",
        grid: null,
        image: null,
        content: { type: "inline", title: "The Town", body: "" },
        links: [{ id: "front-door", target: "inn", x: 1, y: 1, type: "settlement", color: null, hidden: false }],
      },
      { id: "inn", grid: null, image: null, content: null, links: [] },
      { id: "well", grid: null, image: null, content: null, links: [] },
    ],
  };
}

describe("createMinimalProject", () => {
  test("builds a minimal, inline-content, single-location project", () => {
    expect(createMinimalProject("New Realm", "town")).toEqual({
      schemaVersion: 1,
      title: "New Realm",
      defaultLocation: "town",
      content: { type: "inline" },
      locations: [{ id: "town", grid: null, image: null, content: null, links: [] }],
    });
  });
});

describe("saveLink", () => {
  test("merges a patch into the matching link", () => {
    const project = saveLink(fixture(), "town", "front-door", { x: 9, hidden: true });
    expect(project.locations[0]!.links[0]).toEqual({
      id: "front-door",
      target: "inn",
      x: 9,
      y: 1,
      type: "settlement",
      color: null,
      hidden: true,
    });
  });

  test("throws on an unknown location", () => {
    expect(() => saveLink(fixture(), "nowhere", "front-door", {})).toThrow(/No location "nowhere"/);
  });

  test("throws on an unknown link", () => {
    expect(() => saveLink(fixture(), "town", "ghost", {})).toThrow(/has no link "ghost"/);
  });
});

describe("saveLocationContent", () => {
  test("merges a patch, defaulting missing fields to empty", () => {
    const project = saveLocationContent(fixture(), "inn", { title: "The Inn" });
    expect(project.locations[1]!.content).toEqual({ type: "inline", title: "The Inn", body: "" });
  });

  test("preserves existing fields not present in the patch", () => {
    const project = saveLocationContent(fixture(), "town", { body: "Updated." });
    expect(project.locations[0]!.content).toEqual({ type: "inline", title: "The Town", body: "Updated." });
  });

  test("refuses to overwrite non-inline content", () => {
    const project = fixture();
    project.locations[0]!.content = { type: "obsidian", ref: "town.md" };
    expect(() => saveLocationContent(project, "town", { title: "x" })).toThrow(/not inline/);
  });
});

describe("addLocationLink", () => {
  test("creates a new Location and links to it when the id doesn't exist yet", () => {
    const project = addLocationLink(fixture(), "town", "old-mill", 5, 6, "landmark");
    const link = project.locations[0]!.links.find((l) => l.target === "old-mill");
    expect(link).toMatchObject({ target: "old-mill", x: 5, y: 6, type: "landmark", color: null, hidden: false });
    expect(typeof link?.id).toBe("string");
    expect(project.locations.find((l) => l.id === "old-mill")).toEqual({
      id: "old-mill",
      grid: null,
      image: null,
      content: null,
      links: [],
    });
  });

  test("links to an existing Location instead of duplicating it", () => {
    const project = addLocationLink(fixture(), "town", "well", 2, 2, "landmark");
    expect(project.locations.filter((l) => l.id === "well")).toHaveLength(1);
    expect(project.locations.find((l) => l.id === "well")).toEqual({
      id: "well",
      grid: null,
      image: null,
      content: null,
      links: [],
    });
  });

  test("allows a second link from the same parent to the same target — separate entrances", () => {
    // e.g. a front and back door into the same toll house: two pins on
    // the parent's map, both targeting "inn", each with its own id and
    // position.
    const project = addLocationLink(fixture(), "town", "inn", 9, 9, "settlement");
    const linksToInn = project.locations[0]!.links.filter((l) => l.target === "inn");
    expect(linksToInn).toHaveLength(2);
    expect(new Set(linksToInn.map((l) => l.id)).size).toBe(2);
    expect(project.locations.filter((l) => l.id === "inn")).toHaveLength(1);
  });
});

describe("saveGrid", () => {
  test("sets the grid", () => {
    const grid = {
      type: "hex" as const,
      origin: { x: 0, y: 0 },
      b1: { x: 10, y: 0 },
      b2: { x: 5, y: 8 },
      style: { color: "#fff", weight: 1, opacity: 0.5 },
    };
    expect(saveGrid(fixture(), "town", grid).locations[0]!.grid).toEqual(grid);
  });

  test("clears the grid", () => {
    expect(saveGrid(fixture(), "town", null).locations[0]!.grid).toBeNull();
  });
});

describe("saveImage", () => {
  test("sets the image", () => {
    const image = { file: "_assets/town.png", width: 400, height: 300 };
    expect(saveImage(fixture(), "town", image).locations[0]!.image).toEqual(image);
  });

  test("clears the image", () => {
    expect(saveImage(fixture(), "town", null).locations[0]!.image).toBeNull();
  });

  test("throws on an unknown location", () => {
    expect(() => saveImage(fixture(), "nowhere", null)).toThrow(/No location "nowhere"/);
  });
});
