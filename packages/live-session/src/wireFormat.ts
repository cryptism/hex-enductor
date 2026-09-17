import type {
  FogOfWar,
  Grid,
  HexenProject,
  ImageRef,
  Link,
  Location,
  LocationContent,
  ProjectContent,
} from "@hex-enductor/hexen-schema";
import type { Command } from "@hex-enductor/project-ops";
import type { OpenedProjectData } from "./protocol.ts";

/**
 * hexend (Rust) speaks its schema's own protobuf JSON mapping on the wire:
 * a proto3 `oneof` serializes as `{ <selected field name>: value }`,
 * never the `{ type: "...", ...fields }` discriminated-union shape
 * hexen-schema (and everything built on it — map-core, project-ops,
 * the editor) actually uses. Every plain field name already matches
 * (protobuf JSON's default camelCase mapping), so the only seam this
 * module needs to cover is the handful of oneof-shaped types: Grid,
 * ProjectContent, and LocationContent — plus the ClientMessage /
 * ServerMessage envelope itself. A generated packages/hexen-proto-ts
 * would replace this by construction; until then it's hand-written
 * against schema/hexen/v1.
 */

type WireHexGrid = Omit<Extract<Grid, { type: "hex" }>, "type">;
type WireSquareGrid = Omit<Extract<Grid, { type: "square" }>, "type">;
export type WireGrid = { hex: WireHexGrid; square?: undefined } | { hex?: undefined; square: WireSquareGrid };

export function gridFromWire(wire: WireGrid | null | undefined): Grid | null {
  if (!wire) return null;
  if (wire.hex) return { type: "hex", ...wire.hex };
  return { type: "square", ...wire.square! };
}

export function gridToWire(grid: Grid | null): WireGrid | null {
  if (!grid) return null;
  const { type, ...rest } = grid;
  return type === "hex" ? { hex: rest as WireHexGrid } : { square: rest as WireSquareGrid };
}

type WireProjectContent = { obsidian: { vaultRoot: string }; inline?: undefined } | { obsidian?: undefined; inline: object };

export function projectContentFromWire(wire: WireProjectContent): ProjectContent {
  if (wire.obsidian) return { type: "obsidian", vaultRoot: wire.obsidian.vaultRoot };
  return { type: "inline" };
}

export function projectContentToWire(content: ProjectContent): WireProjectContent {
  return content.type === "obsidian" ? { obsidian: { vaultRoot: content.vaultRoot } } : { inline: {} };
}

type WireLocationContent =
  | { obsidian: { ref: string }; inline?: undefined }
  | { obsidian?: undefined; inline: { title: string; body?: string } };

export function locationContentFromWire(wire: WireLocationContent | null | undefined): LocationContent | null {
  if (!wire) return null;
  if (wire.obsidian) return { type: "obsidian", ref: wire.obsidian.ref };
  return { type: "inline", title: wire.inline!.title, body: wire.inline!.body ?? "" };
}

interface WireLocation {
  id: string;
  grid?: WireGrid | null;
  image?: ImageRef | null;
  content?: WireLocationContent | null;
  links?: Link[];
  fog?: FogOfWar | null;
}

function locationFromWire(wire: WireLocation): Location {
  return {
    id: wire.id,
    grid: gridFromWire(wire.grid),
    image: wire.image ?? null,
    content: locationContentFromWire(wire.content),
    links: wire.links ?? [],
    fog: wire.fog ?? null,
  };
}

interface WireHexenProject {
  schemaVersion: 1;
  title: string;
  defaultLocation: string;
  content: WireProjectContent;
  locations?: WireLocation[];
}

function projectFromWire(wire: WireHexenProject): HexenProject {
  return {
    schemaVersion: wire.schemaVersion,
    title: wire.title,
    defaultLocation: wire.defaultLocation,
    content: projectContentFromWire(wire.content),
    locations: (wire.locations ?? []).map(locationFromWire),
  };
}

interface WireOpenedProjectData {
  project: WireHexenProject;
  warnings?: string[];
  resolvedContent?: OpenedProjectData["resolvedContent"];
  resolveErrors?: OpenedProjectData["resolveErrors"];
}

export function openedProjectDataFromWire(wire: WireOpenedProjectData): OpenedProjectData {
  return {
    project: projectFromWire(wire.project),
    warnings: wire.warnings ?? [],
    resolvedContent: wire.resolvedContent ?? {},
    resolveErrors: wire.resolveErrors ?? {},
  };
}

export function commandToWire(command: Command): object {
  switch (command.type) {
    case "saveLink":
      return { saveLink: { locationId: command.locationId, linkId: command.linkId, patch: command.patch } };
    case "saveLocationContent":
      return { saveLocationContent: { locationId: command.locationId, patch: command.patch } };
    case "addLocationLink":
      return {
        addLocationLink: {
          parentLocationId: command.parentLocationId,
          targetLocationId: command.targetLocationId,
          x: command.x,
          y: command.y,
          linkType: command.linkType,
        },
      };
    case "saveGrid":
      return { saveGrid: { locationId: command.locationId, grid: gridToWire(command.grid) } };
    case "saveImage":
      return { saveImage: { locationId: command.locationId, image: command.image } };
    case "setFog":
      return { setFog: { locationId: command.locationId, fog: command.fog } };
    case "setFogCells":
      return { setFogCells: { locationId: command.locationId, cells: command.cells, revealed: command.revealed } };
    case "addLocation":
      return { addLocation: { locationId: command.locationId } };
  }
}
