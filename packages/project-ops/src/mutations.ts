import type { FogOfWar, Grid, HexenProject, ImageRef, Link, Location, ProjectContent } from "@hex-enductor/hexen-schema";

/**
 * Pure, transport-agnostic operations on an in-memory HexenProject —
 * no file I/O, no path, no server. The editor's browser-native storage
 * backend calls these directly; apps/hexend (Rust) hand-ports the same
 * logic (see its own mutations.rs) since it isn't part of this bun
 * workspace, so the two never drift apart on what a mutation means.
 */

export function createMinimalProject(
  title: string,
  defaultLocationId: string,
  content: ProjectContent = { type: "inline" },
): HexenProject {
  return {
    schemaVersion: 1,
    title,
    defaultLocation: defaultLocationId,
    content,
    locations: [{ id: defaultLocationId, grid: null, image: null, content: null, links: [], fog: null }],
  };
}

function findLocation(project: HexenProject, locationId: string): Location {
  const location = project.locations.find((l) => l.id === locationId);
  if (!location) {
    throw new Error(`No location "${locationId}" in this project`);
  }
  return location;
}

export function saveLink(
  project: HexenProject,
  locationId: string,
  linkId: string,
  patch: Partial<Link>,
): HexenProject {
  const location = findLocation(project, locationId);
  const linkIndex = location.links.findIndex((l) => l.id === linkId);
  if (linkIndex === -1) {
    throw new Error(`Location "${locationId}" has no link "${linkId}"`);
  }
  location.links[linkIndex] = { ...location.links[linkIndex]!, ...patch };
  return project;
}

export function saveLocationContent(
  project: HexenProject,
  locationId: string,
  patch: { title?: string; body?: string },
): HexenProject {
  const location = findLocation(project, locationId);
  if (location.content !== null && location.content.type !== "inline") {
    throw new Error(`Location "${locationId}" has ${location.content.type} content, not inline`);
  }

  location.content = {
    type: "inline",
    title: location.content?.title ?? "",
    body: location.content?.body ?? "",
    ...patch,
  };
  return project;
}

export function addLocationLink(
  project: HexenProject,
  parentLocationId: string,
  targetLocationId: string,
  x: number,
  y: number,
  type: string,
): HexenProject {
  const parent = findLocation(project, parentLocationId);

  // The target might be a brand-new place, or an existing Location
  // that just didn't have a pin on this particular map yet — both are
  // the same operation, adding a Link, so only create the Location
  // itself when it doesn't already exist.
  if (!project.locations.some((l) => l.id === targetLocationId)) {
    project.locations.push({ id: targetLocationId, grid: null, image: null, content: null, links: [], fog: null });
  }

  // A Link's own id is independent of its target on purpose — a
  // Location can be the target of more than one Link from the same
  // (or a different) parent, e.g. a front and back door into the same
  // building, each its own pin at its own position.
  parent.links.push({ id: crypto.randomUUID(), target: targetLocationId, x, y, type, color: null, hidden: false });
  return project;
}

export function saveGrid(project: HexenProject, locationId: string, grid: Grid | null): HexenProject {
  const location = findLocation(project, locationId);
  location.grid = grid;
  return project;
}

export function saveImage(project: HexenProject, locationId: string, image: ImageRef | null): HexenProject {
  const location = findLocation(project, locationId);
  location.image = image;
  return project;
}

export function setFog(project: HexenProject, locationId: string, fog: FogOfWar | null): HexenProject {
  const location = findLocation(project, locationId);
  location.fog = fog;
  return project;
}

/**
 * Sets a batch of cells' revealed state explicitly, rather than
 * toggling, so repeatedly painting the same cell while dragging is
 * idempotent. Plural rather than one call per cell so a whole
 * click-drag stroke lands as a single command/broadcast.
 */
export function setFogCells(project: HexenProject, locationId: string, cellsToSet: string[], revealed: boolean): HexenProject {
  const location = findLocation(project, locationId);
  if (!location.fog) {
    throw new Error(`Location "${locationId}" has no fog of war — start it before setting cells`);
  }
  const cells = new Set(location.fog.revealedCells);
  for (const cell of cellsToSet) {
    if (revealed) {
      cells.add(cell);
    } else {
      cells.delete(cell);
    }
  }
  location.fog = { revealedCells: [...cells] };
  return project;
}
