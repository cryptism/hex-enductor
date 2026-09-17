import { describe, expect, test } from "bun:test";
import type { HexenProject } from "@hex-enductor/hexen-schema";
import { applyCommand, CommandSchema, type Command } from "./commands.ts";

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
        fog: null,
      },
      { id: "inn", grid: null, image: null, content: null, links: [], fog: null },
    ],
  };
}

describe("CommandSchema", () => {
  test("parses one of each command shape", () => {
    const commands: Command[] = [
      { type: "saveLink", locationId: "town", linkId: "front-door", patch: { hidden: true } },
      { type: "saveLocationContent", locationId: "town", patch: { body: "Updated." } },
      { type: "addLocationLink", parentLocationId: "town", targetLocationId: "old-mill", x: 1, y: 2, linkType: "landmark" },
      { type: "saveGrid", locationId: "town", grid: null },
      { type: "saveImage", locationId: "town", image: null },
      { type: "setFog", locationId: "town", fog: { revealedCells: ["0,0"] } },
      { type: "setFogCells", locationId: "town", cells: ["0,0"], revealed: true },
      { type: "addLocation", locationId: "staged-map" },
    ];
    for (const command of commands) {
      expect(CommandSchema.safeParse(command).success).toBe(true);
    }
  });

  test("rejects an unknown command type", () => {
    expect(CommandSchema.safeParse({ type: "deleteEverything" }).success).toBe(false);
  });
});

describe("applyCommand", () => {
  test("dispatches saveLink", () => {
    const project = applyCommand(fixture(), {
      type: "saveLink",
      locationId: "town",
      linkId: "front-door",
      patch: { hidden: true },
    });
    expect(project.locations[0]!.links[0]!.hidden).toBe(true);
  });

  test("dispatches addLocationLink, mapping linkType to the pin's marker type", () => {
    const project = applyCommand(fixture(), {
      type: "addLocationLink",
      parentLocationId: "town",
      targetLocationId: "old-mill",
      x: 5,
      y: 6,
      linkType: "landmark",
    });
    const link = project.locations[0]!.links.find((l) => l.target === "old-mill");
    expect(link).toMatchObject({ type: "landmark", x: 5, y: 6 });
  });

  test("dispatches saveGrid and saveImage", () => {
    const withImage = applyCommand(fixture(), {
      type: "saveImage",
      locationId: "town",
      image: { file: "_assets/town.png", width: 400, height: 300 },
    });
    expect(withImage.locations[0]!.image).toEqual({ file: "_assets/town.png", width: 400, height: 300 });

    const withoutGrid = applyCommand(fixture(), { type: "saveGrid", locationId: "town", grid: null });
    expect(withoutGrid.locations[0]!.grid).toBeNull();
  });

  test("dispatches setFog and setFogCells", () => {
    const started = applyCommand(fixture(), { type: "setFog", locationId: "town", fog: { revealedCells: [] } });
    expect(started.locations[0]!.fog).toEqual({ revealedCells: [] });

    const revealed = applyCommand(started, { type: "setFogCells", locationId: "town", cells: ["0,0"], revealed: true });
    expect(revealed.locations[0]!.fog).toEqual({ revealedCells: ["0,0"] });
  });

  test("dispatches addLocation", () => {
    const project = applyCommand(fixture(), { type: "addLocation", locationId: "staged-map" });
    expect(project.locations.find((l) => l.id === "staged-map")).toBeTruthy();
  });
});
