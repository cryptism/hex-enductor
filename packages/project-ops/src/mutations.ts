import type { Grid, HexenProject, ImageRef, Link, Location } from "@hex-enductor/hexen-schema";

/**
 * Pure, transport-agnostic operations on an in-memory HexenProject —
 * no file I/O, no path, no server. Both apps/hexend's tRPC procedures
 * and the editor's browser-native storage back end call these, so the
 * two never drift apart on what a mutation actually means.
 */

export function createMinimalProject(title: string, defaultLocationId: string): HexenProject {
  return {
    schemaVersion: 1,
    title,
    defaultLocation: defaultLocationId,
    content: { type: "inline" },
    locations: [{ id: defaultLocationId, grid: null, image: null, content: null, links: [] }],
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
  locationId: string,
  x: number,
  y: number,
  type: string,
): HexenProject {
  const parent = findLocation(project, parentLocationId);
  if (parent.links.some((l) => l.id === locationId)) {
    throw new Error(`"${parentLocationId}" already has a link to "${locationId}"`);
  }

  // The target might be a brand-new place, or an existing Location
  // that just didn't have a pin on this particular map yet — both are
  // the same operation, adding a Link, so only create the Location
  // itself when it doesn't already exist.
  if (!project.locations.some((l) => l.id === locationId)) {
    project.locations.push({ id: locationId, grid: null, image: null, content: null, links: [] });
  }

  parent.links.push({ id: locationId, x, y, type, color: null, hidden: false });
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
