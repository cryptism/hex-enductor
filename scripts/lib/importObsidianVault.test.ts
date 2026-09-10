import { describe, expect, test } from "bun:test";
import { buildHexenProject, slugify, type VaultEntry } from "./importObsidianVault.ts";

describe("slugify", () => {
  test("lowercases and hyphenates spaces", () => {
    expect(slugify("An Unnamed Chapel")).toBe("an-unnamed-chapel");
  });

  test("drops apostrophes rather than hyphenating them", () => {
    expect(slugify("Willard's Hideout")).toBe("willards-hideout");
  });

  test("transliterates o-with-stroke", () => {
    expect(slugify("Frøsselanding")).toBe("frosselanding");
  });

  test("collapses runs of punctuation into one hyphen", () => {
    expect(slugify("The Temple of 1,000 Swords")).toBe("the-temple-of-1-000-swords");
  });
});

const MAP_IMAGE = { file: "_assets/vilheim-crace.png", width: 100, height: 100 };

function mapRootEntry(overrides: Partial<Record<string, unknown>> = {}): VaultEntry {
  return {
    relativePath: "Locations/Vilheim.md",
    frontmatter: {
      "map-root": true,
      "map-id": "vilheim-crace",
      title: "Vilheim & Crace",
      "map-hex-origin-x": 0,
      "map-hex-origin-y": 0,
      "map-hex-b1-x": 10,
      "map-hex-b1-y": 0,
      "map-hex-b2-x": 0,
      "map-hex-b2-y": 10,
      "map-hex-km-per-hex": 9,
      "map-hex-color": "#c19a5f",
      ...overrides,
    },
  };
}

function pinEntry(relativePath: string, overrides: Partial<Record<string, unknown>> = {}): VaultEntry {
  return {
    relativePath,
    frontmatter: {
      map: "vilheim-crace",
      "map-x": 10,
      "map-y": 20,
      "map-type": "settlement",
      "map-icon": "village",
      ...overrides,
    },
  };
}

describe("buildHexenProject", () => {
  test("builds a map Location plus a pin Location and Link", () => {
    const { project, warnings } = buildHexenProject(
      [mapRootEntry(), pinEntry("Locations/Pentegil Manor.md")],
      { title: "Test", vaultRoot: ".", mapImages: { "vilheim-crace": MAP_IMAGE } },
    );

    expect(warnings).toEqual([]);
    expect(project.defaultLocation).toBe("vilheim-crace");

    const map = project.locations.find((l) => l.id === "vilheim-crace")!;
    expect(map.grid?.type).toBe("hex");
    expect(map.image).toEqual(MAP_IMAGE);
    expect(map.content).toEqual({ type: "obsidian", ref: "Locations/Vilheim.md" });
    expect(map.links).toHaveLength(1);
    expect(map.links[0]).toMatchObject({
      target: "pentegil-manor",
      x: 10,
      y: 20,
      type: "settlement",
      icon: "village",
      color: null,
      hidden: false,
    });
    expect(typeof map.links[0]!.id).toBe("string");

    const pin = project.locations.find((l) => l.id === "pentegil-manor")!;
    expect(pin.grid).toBeNull();
    expect(pin.content).toEqual({ type: "obsidian", ref: "Locations/Pentegil Manor.md" });
  });

  test("still creates the pin's own Location, but drops the link and warns, when its map id is unknown", () => {
    const { project, warnings } = buildHexenProject(
      [mapRootEntry(), pinEntry("Locations/Orphan.md", { map: "nowhere" })],
      { title: "Test", vaultRoot: ".", mapImages: { "vilheim-crace": MAP_IMAGE } },
    );

    expect(warnings.some((w) => w.includes("nowhere"))).toBe(true);
    expect(project.locations.find((l) => l.id === "vilheim-crace")!.links).toEqual([]);
    expect(project.locations.find((l) => l.id === "orphan")).toBeDefined();
  });

  test("warns and leaves image null when a map has no resolved image", () => {
    const { project, warnings } = buildHexenProject([mapRootEntry()], {
      title: "Test",
      vaultRoot: ".",
      mapImages: {},
    });
    expect(warnings.some((w) => w.includes("vilheim-crace"))).toBe(true);
    expect(project.locations[0]!.image).toBeNull();
  });

  test("falls back to map-icon/map-color defaults when absent", () => {
    const { project } = buildHexenProject(
      [mapRootEntry(), pinEntry("Locations/Bare.md", { "map-icon": undefined, "map-type": undefined })],
      { title: "Test", vaultRoot: ".", mapImages: { "vilheim-crace": MAP_IMAGE } },
    );
    const link = project.locations.find((l) => l.id === "vilheim-crace")!.links[0]!;
    expect(link.type).toBe("waypoint");
    expect(link.icon).toBeUndefined();
    expect(link.color).toBeNull();
  });
});
