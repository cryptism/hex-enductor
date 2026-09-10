#!/usr/bin/env bun
// Regenerates packages/map-core/src/linkIcons.ts from the source SVGs at
// https://github.com/game-icons/icons (CC BY 3.0 — see CREDITS.md at the
// repo root). To add an icon: add an entry to ICON_SOURCES below, then run
// `bun run scripts/sync-link-icons.ts`.
import { writeFile } from "node:fs/promises";

type Author = "delapouite" | "lorc";
type Category = "settlement" | "landmark" | "ruin" | "hazard" | "waypoint";

interface IconSource {
  slug: string;
  label: string;
  category: Category;
  author: Author;
}

const ICON_SOURCES: IconSource[] = [
  // Settlement
  { slug: "village", label: "Village", category: "settlement", author: "delapouite" },
  { slug: "medieval-gate", label: "Gate", category: "settlement", author: "delapouite" },
  { slug: "hill-fort", label: "Hill fort", category: "settlement", author: "delapouite" },
  { slug: "watchtower", label: "Watchtower", category: "settlement", author: "delapouite" },
  { slug: "windmill", label: "Windmill", category: "settlement", author: "delapouite" },
  { slug: "well", label: "Well", category: "settlement", author: "delapouite" },
  { slug: "tavern-sign", label: "Tavern", category: "settlement", author: "delapouite" },

  // Landmark
  { slug: "church", label: "Church", category: "landmark", author: "delapouite" },
  { slug: "greek-temple", label: "Temple", category: "landmark", author: "delapouite" },
  { slug: "castle", label: "Castle", category: "landmark", author: "lorc" },
  { slug: "lighthouse", label: "Lighthouse", category: "landmark", author: "delapouite" },
  { slug: "obelisk", label: "Obelisk", category: "landmark", author: "delapouite" },
  { slug: "crystal-shrine", label: "Shrine", category: "landmark", author: "delapouite" },
  { slug: "oasis", label: "Oasis", category: "landmark", author: "delapouite" },
  { slug: "waterfall", label: "Waterfall", category: "landmark", author: "delapouite" },

  // Ruin
  { slug: "castle-ruins", label: "Castle ruins", category: "ruin", author: "delapouite" },
  { slug: "ancient-ruins", label: "Ancient ruins", category: "ruin", author: "delapouite" },
  { slug: "crypt-entrance", label: "Crypt entrance", category: "ruin", author: "delapouite" },
  { slug: "cave-entrance", label: "Cave entrance", category: "ruin", author: "delapouite" },
  { slug: "dungeon-gate", label: "Dungeon gate", category: "ruin", author: "delapouite" },
  { slug: "graveyard", label: "Graveyard", category: "ruin", author: "delapouite" },
  { slug: "tombstone", label: "Tombstone", category: "ruin", author: "lorc" },
  { slug: "rune-stone", label: "Rune stone", category: "ruin", author: "lorc" },

  // Hazard
  { slug: "swamp", label: "Swamp", category: "hazard", author: "delapouite" },
  { slug: "quicksand", label: "Quicksand", category: "hazard", author: "lorc" },
  { slug: "spider-web", label: "Spider web", category: "hazard", author: "lorc" },
  { slug: "thorny-vine", label: "Thorny vine", category: "hazard", author: "lorc" },
  { slug: "sandstorm", label: "Sandstorm", category: "hazard", author: "delapouite" },
  { slug: "volcano", label: "Volcano", category: "hazard", author: "lorc" },
  { slug: "lightning-storm", label: "Lightning storm", category: "hazard", author: "lorc" },
  { slug: "poison-gas", label: "Poison gas", category: "hazard", author: "lorc" },

  // Waypoint
  { slug: "campfire", label: "Campfire", category: "waypoint", author: "lorc" },
  { slug: "compass", label: "Compass", category: "waypoint", author: "lorc" },
  { slug: "footprint", label: "Footprint", category: "waypoint", author: "lorc" },
  { slug: "pin", label: "Pin", category: "waypoint", author: "delapouite" },
  { slug: "wooden-sign", label: "Signpost", category: "waypoint", author: "lorc" },
];

const CATEGORY_ORDER: Category[] = ["settlement", "landmark", "ruin", "hazard", "waypoint"];

async function fetchIconSvg(source: IconSource): Promise<string> {
  const url = `https://raw.githubusercontent.com/game-icons/icons/master/${source.author}/${source.slug}.svg`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch ${url}: ${res.status}`);
  const raw = await res.text();

  // Source files are a black background rect plus a white icon path —
  // drop the background and recolor the icon path so callers can tint
  // it via CSS `color`.
  const paths = raw.match(/<path[^>]*\/>/g);
  if (!paths || paths.length !== 2) {
    throw new Error(`Unexpected SVG shape for ${source.slug} (expected 2 <path> elements)`);
  }
  const iconPath = paths[1].replace('fill="#fff"', 'fill="currentColor"');
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">${iconPath}</svg>`;
}

async function main() {
  const sorted = [...ICON_SOURCES].sort(
    (a, b) => CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category) || a.slug.localeCompare(b.slug),
  );

  const entries = await Promise.all(
    sorted.map(async (source) => ({ ...source, svg: await fetchIconSvg(source) })),
  );

  const lines: string[] = [
    'export type IconAuthor = "delapouite" | "lorc";',
    'export type IconCategory = "settlement" | "landmark" | "ruin" | "hazard" | "waypoint";',
    "",
    "export interface LinkIconDef {",
    "  slug: string;",
    "  label: string;",
    "  category: IconCategory;",
    "  author: IconAuthor;",
    '  /** Single-color SVG markup; the icon path uses fill="currentColor". */',
    "  svg: string;",
    "}",
    "",
    "export const LINK_ICONS: LinkIconDef[] = [",
  ];
  for (const e of entries) {
    lines.push(
      "  {",
      `    slug: ${JSON.stringify(e.slug)},`,
      `    label: ${JSON.stringify(e.label)},`,
      `    category: ${JSON.stringify(e.category)},`,
      `    author: ${JSON.stringify(e.author)},`,
      `    svg: ${JSON.stringify(e.svg)},`,
      "  },",
    );
  }
  lines.push(
    "];",
    "",
    "export function findLinkIcon(slug: string | undefined): LinkIconDef | undefined {",
    "  return slug ? LINK_ICONS.find((icon) => icon.slug === slug) : undefined;",
    "}",
    "",
  );

  const outPath = new URL("../packages/map-core/src/linkIcons.ts", import.meta.url);
  await writeFile(outPath, lines.join("\n"));
  console.log(`Wrote ${entries.length} icons to ${outPath.pathname}`);
}

main();
