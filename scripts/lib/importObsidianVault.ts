import type { HexenProject, Location, Link } from "@hex-enductor/hexen-schema";

export interface VaultEntry {
  /** Path to the note, relative to the vault root. */
  relativePath: string;
  frontmatter: Record<string, unknown>;
}

export interface MapImageRef {
  file: string;
  width: number;
  height: number;
}

export interface BuildOptions {
  title: string;
  /** content.vaultRoot to embed in the generated project. */
  vaultRoot: string;
  /** map-id -> where its base image ended up, relative to the output .hexen.yml. */
  mapImages: Record<string, MapImageRef>;
}

export interface BuildResult {
  project: HexenProject;
  warnings: string[];
}

/**
 * Slugifies a note's filename into a Location id. Not a general-purpose
 * transliterator — handles what illuminated-world's vault actually
 * contains (apostrophes, a couple of accented letters) rather than the
 * full space of Unicode.
 */
export function slugify(text: string): string {
  return text
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "") // combining diacritical marks left behind by NFKD
    .replace(/ø/g, "o")
    .replace(/Ø/g, "O")
    .toLowerCase()
    .replace(/'/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function isMapRoot(fm: Record<string, unknown>): boolean {
  return fm["map-root"] === true && typeof fm["map-id"] === "string";
}

function isPin(fm: Record<string, unknown>): boolean {
  return (
    typeof fm["map"] === "string" &&
    typeof fm["map-x"] === "number" &&
    typeof fm["map-y"] === "number"
  );
}

/**
 * Pure: takes already-parsed frontmatter (see readVaultEntries.ts for
 * the disk-reading half) and each map's already-resolved image, and
 * builds the Location/Link graph — today's map-root note becomes a
 * Location with a grid, each pinned note becomes both its own Location
 * (for its content) and a Link on its map's Location (for its
 * position). No disk access, so the actual graph-building logic here
 * is unit-testable without fixture files.
 */
export function buildHexenProject(entries: VaultEntry[], opts: BuildOptions): BuildResult {
  const warnings: string[] = [];
  const locations: Location[] = [];
  const linksByMapId = new Map<string, Link[]>();

  const mapRoots = entries.filter((e) => isMapRoot(e.frontmatter));
  for (const root of mapRoots) {
    const fm = root.frontmatter;
    const mapId = fm["map-id"] as string;
    const image = opts.mapImages[mapId] ?? null;
    if (!image) {
      warnings.push(`No resolved image for map "${mapId}" — image will be null.`);
    }

    const hasHexCalibration =
      typeof fm["map-hex-origin-x"] === "number" &&
      typeof fm["map-hex-b1-x"] === "number" &&
      typeof fm["map-hex-b2-x"] === "number";

    locations.push({
      id: mapId,
      grid: hasHexCalibration
        ? {
            type: "hex",
            origin: { x: fm["map-hex-origin-x"] as number, y: fm["map-hex-origin-y"] as number },
            b1: { x: fm["map-hex-b1-x"] as number, y: fm["map-hex-b1-y"] as number },
            b2: { x: fm["map-hex-b2-x"] as number, y: fm["map-hex-b2-y"] as number },
            distancePerCell:
              typeof fm["map-hex-km-per-hex"] === "number" ? fm["map-hex-km-per-hex"] : undefined,
            style: {
              color: typeof fm["map-hex-color"] === "string" ? fm["map-hex-color"] : "#c19a5f",
              weight: typeof fm["map-hex-weight"] === "number" ? fm["map-hex-weight"] : 1,
              opacity: typeof fm["map-hex-opacity"] === "number" ? fm["map-hex-opacity"] : 0.45,
            },
          }
        : null,
      image,
      content: { type: "obsidian", ref: root.relativePath },
      links: [],
    });
    linksByMapId.set(mapId, []);
  }

  const pins = entries.filter((e) => isPin(e.frontmatter));
  for (const pin of pins) {
    const fm = pin.frontmatter;
    const mapId = fm["map"] as string;
    const id = slugify(pin.relativePath.replace(/\.md$/i, "").split("/").pop()!);

    locations.push({
      id,
      grid: null,
      image: null,
      content: { type: "obsidian", ref: pin.relativePath },
      links: [],
    });

    const links = linksByMapId.get(mapId);
    if (!links) {
      warnings.push(
        `"${pin.relativePath}" has map: "${mapId}", but no map-root note declares that map-id — dropping its pin.`,
      );
      continue;
    }
    links.push({
      id,
      x: fm["map-x"] as number,
      y: fm["map-y"] as number,
      type: typeof fm["map-type"] === "string" ? fm["map-type"] : "waypoint",
      icon: typeof fm["map-icon"] === "string" ? fm["map-icon"] : undefined,
      color: typeof fm["map-color"] === "string" ? fm["map-color"] : null,
      hidden: false,
    });
  }

  for (const location of locations) {
    const links = linksByMapId.get(location.id);
    if (links) location.links = links;
  }

  if (mapRoots.length === 0) {
    warnings.push("No map-root note found — defaultLocation will be a guess.");
  }
  if (mapRoots.length > 1) {
    warnings.push(
      `${mapRoots.length} map-root notes found; using "${mapRoots[0]!.frontmatter["map-id"]}" as defaultLocation.`,
    );
  }

  const project: HexenProject = {
    schemaVersion: 1,
    title: opts.title,
    defaultLocation: (mapRoots[0]?.frontmatter["map-id"] as string) ?? locations[0]?.id ?? "",
    content: { type: "obsidian", vaultRoot: opts.vaultRoot },
    locations,
  };

  return { project, warnings };
}
