# Architecture plan

Status: pre-implementation. Nothing here is built yet — this is the plan to react to before any code gets written.

## 1. What this actually is

Today, the map tool lives inside `illuminated-world/site/maps/` as a generic Leaflet app (vanilla JS, no framework) that reads a `data.json` compiled at build time from an Obsidian vault, plus `map-server.py`, a dev-only Python server that adds a Ctrl+click location editor for local authoring. There's no persistent "editor" — authoring happens in Obsidian, and `map-server.py` (dev mode) is a thin local preview with one write-back path (editing marker position).

Pulling this out standalone is really pulling apart three things that currently overlap:

1. **Authoring** — a GM placing/editing locations, calibrating the hex grid, drawing on a map. Currently: Obsidian + dev mode.
2. **Publishing** — a read-only embed inside the wiki, for players/GM browsing lore between sessions. Currently: the built `data.json` + the same Leaflet app with editing stripped out.
3. **Live play** — a GM running a session, a Presentation view on a second monitor reacting to GM commands in real time (fog of war, pings, map switches). Doesn't exist yet — this is the new thing.

The plan below treats these as three *surfaces* sharing one *core* (rendering, data model, marker/hex logic), not three separate apps built from scratch three times.

## 2. Proposed shape

A pnpm workspace monorepo:

```
hex-enductor/
  apps/
    editor/              GM authoring app — Vite + React + TS
    presentation/         read-only player display — Vite + React + TS, much smaller
    server/               realtime + file I/O — TS, Hono
  packages/
    hexen-schema/         Zod schemas for the .hexen.yml format (§6) — the single source of truth types are derived from
    content-resolver/      the generic "resolve a location's content" interface — see §6a
    content-obsidian/       the first (only, for now) implementation of it — reads frontmatter+body from a vault path
    map-core/              <MapCanvas> component, grid math (hex + square), marker rendering — used by editor, presentation, AND the wiki embed
    ui/                     shared form controls / panel chrome, optional — only worth it once editor and presentation both need it
  docs/
    PLAN.md               this file
    ROADMAP.md
```

`map-core` is the piece that makes "three surfaces, one core" real: it's a dumb, mode-agnostic renderer (image + grid overlay + markers + popups) that takes a `readOnly` / `interactive` flag and an optional `liveState` (fog, pings) prop. The wiki embed becomes: import `map-core`, render it `readOnly`, no server, no websocket — same component the editor uses in edit mode, same component the presentation view uses with a websocket feed on top.

`content-resolver` / `content-obsidian` is the other half of "built as a plugin from day one," per your instinct about decoupling Obsidian rendering — see §6a for why this is its own package rather than a function inside `hexen-schema`.

## 3. Framework choice: not Next.js

You asked me to actually weigh this rather than default to it, so: I'd recommend **Vite + React + TypeScript** (a plain SPA) for both `editor` and `presentation`, not Next.js.

Next.js earns its complexity when you need SSR/SSG for content pages, file-based routing across many pages, or SEO. None of that applies here — this is an interactive canvas tool with effectively one screen per app, used by you, not indexed by anyone. Next.js's App Router also blurs client/server component boundaries in a way that adds real mental overhead for a canvas-heavy, client-state-heavy app, for no corresponding benefit. A Vite SPA gives you a faster dev loop, a simpler mental model, and a trivial build output (static files) you can serve however you like.

Where Next.js (or Remix) *would* earn its place: if "online play" (roadmap, lesser priority) becomes a real hosted multi-tenant product later — multiple GMs, accounts, a marketing/docs site, server-rendered campaign pages for SEO. That's a genuinely different product shape. Worth reconsidering then, not now.

Briefly, on staying in React vs. not: canvas/pan-zoom apps with lots of small position updates (drag a marker, live fog brush strokes) are exactly the case where React's re-render model costs you something and SolidJS's fine-grained reactivity doesn't. I'm not recommending you leave React for this — the ecosystem win (react-leaflet, React Hook Form, this being the stack you already know) outweighs it for v1 — but if fog-of-war painting ever feels laggy, that's the direction to look, not "add more memoization."

**Leaflet itself**: keep it. It already does exactly what you need (`L.CRS.Simple` pixel-space maps, a mature plugin ecosystem, and you've already built calibration/hex/ruler logic against it). Wrap it in `react-leaflet` rather than replacing it — that gets you React component ergonomics (markers as JSX, popups as React portals) without discarding any of the existing map logic.

## 4. Forms and state

- **Forms**: React Hook Form + Zod (via `@hookform/resolvers`). The win here is specifically that the *same* Zod schema in `packages/hexen-schema` validates a location form on submit **and** defines the `.hexen.yml` format (§6) — one schema, not two things to keep in sync.
- **App/editor state** (selected tool, selected marker, calibration-in-progress, fog brush state): **Zustand**. Context alone gets expensive once you have frequent canvas-driven updates re-rendering large trees; Redux is more ceremony than this needs. Zustand (optionally with its `immer` middleware for ergonomic nested updates) is the common choice for exactly this shape of app — it's what Excalidraw-style editors tend to reach for.
- **Server state** (fetching/saving `.hexen.yml` projects, the realtime command channel): see §7 — paired with tRPC this mostly stops being a separate concern.

## 5. GM vs. Player roles

Worth being explicit about this now since it shapes the server's auth model, not just the UI:

| | GM (editor + presentation controller) | Player (presentation view) |
|---|---|---|
| Reads | Full location data, GM notes, hidden locations, fog state | Only non-hidden locations, only revealed fog areas, no GM notes |
| Writes | Everything — locations, calibration, fog, map switches | Nothing |
| Connection | Authenticated (even if that's just "the one person running the app locally") | Unauthenticated or session-token-only; joins a GM's session |
| Realtime | Sends commands (switch map, reveal fog, ping location, open dialog) | Receives commands, renders resulting state |

Concretely: the **Presentation view is a pure function of GM-broadcast state** — it holds no independent authority, doesn't fetch GM notes even if it could reach the server, and every "hidden" or "fogged" filter happens server-side before it reaches that client, not just visually hidden in the UI. That last point matters more than it sounds: if filtering happens client-side, a player opening devtools sees the GM notes anyway. Filter on the server.

## 6. The `.hexen.yml` format

Revised per your read on this — single file per project, flat locations array, content stays in Obsidian markdown and gets resolved at load time rather than duplicated. This replaces the folder-per-map proposal entirely.

A **location** is the one node type in the format. It's deliberately the same concept whether it's "a continent-scale map with a hex grid and forty things pinned to it" or "a torch bracket with a two-line description and nothing pinned to it" — the only thing that changes is which fields are present. A location becomes a *map you can click into* purely by having a `grid`; it becomes *a point on someone else's map* purely by being the target of a `links[]` entry elsewhere. Both can be true of the same location at once (Pentegil Manor is a pin on the Vilheim & Crace map **and** has its own floor-plan grid with its own pins). This is what makes the whole project one flat, searchable DAG (roadmap: browse/search all nodes) instead of a map-shaped tree.

```yaml
# illuminated-world.hexen.yml
schemaVersion: 1
title: The Illuminated World
defaultLocation: vilheim-crace

content:
  type: obsidian
  vaultRoot: ../illuminated-world/site/vault   # content.ref values resolve relative to this

locations:
  - id: vilheim-crace
    grid:
      type: hex
      origin: { x: 1706.96, y: 1758.02 }
      b1: { x: 84.99, y: 0.08 }
      b2: { x: 42.41, y: 73.49 }
      distancePerCell: 9
      style: { color: "#c19a5f", weight: 1, opacity: 0.45 }
    image: { file: assets/vilheim-crace.png, width: 3840, height: 2160 }   # resolves relative to this .hexen.yml file, not vaultRoot
    content:
      type: obsidian
      ref: "Locations/The Most Glorious Exarchates of Vilheim & Crace.md"
    links:
      - id: pentegil-manor
        x: 412
        y: 233
        type: settlement
        icon: castle
        color: null
        hidden: false

  - id: pentegil-manor
    grid: null            # no map of its own (yet) — just a pin on vilheim-crace
    image: null
    content:
      type: obsidian
      ref: "Locations/Pentegil Manor.md"
    links: []
```

Walking through the deliberate choices in there:

- **`grid`** generalizes today's `map-hex-*` frontmatter, and is a discriminated union on `type` so square grids (your stated near-term priority) are a second case, not a rewrite: `type: square` would carry `origin` + `cellSize: { x, y }` instead of `b1`/`b2`, plus the same `style`. I renamed `kmPerHex` to `distancePerCell` so one field name works for both — nothing depends on the old name yet, so now's the moment to fix it. `grid: null` means "this location isn't itself a map."
- **`links`** is today's `map-x`/`map-y`/`map-type`/`map-icon`/`map-color` frontmatter, moved off the *referenced* location and onto the *referencing* one. That's the crucial shift: position is a property of the relationship between two locations (*where does Pentegil Manor sit on Vilheim & Crace's map*), not a property of Pentegil Manor itself — which is what makes a location nameable and positioned differently on two different parent maps if that ever comes up, and what makes map-stitching (roadmap) and click-through-to-submap (roadmap) both just "another entry in `links[]` pointing at a location with a `grid`," no separate mechanism needed.
- **`content.ref`** is the pointer into the Obsidian vault, resolved at load time — title, summary, and body all come from the referenced markdown file's frontmatter/content, not from `.hexen.yml`. A location's `links[]` entries never carry a title or summary for that reason; they're pure positioning + display (icon/color/type/hidden), and the name gets looked up through `id → locations[].content.ref → frontmatter.title` when something needs to render it.
- **`image.file`**, unlike `content.ref`, resolves relative to the `.hexen.yml` file's own directory, not `vaultRoot` — map images live in a hex-enductor-owned `assets/` folder rather than the vault's existing `_maps/<id>/` folders. That's one copy of each image to carry over during migration rather than zero, but it keeps image storage decoupled from "wherever the Obsidian vault happens to be," consistent with treating Obsidian as a swappable content plugin rather than a filesystem dependency baked into the format.
- **Fog of war and other session state stay out of this file entirely** — a sibling file (`illuminated-world.hexen.state.json`, say), not a field on a location. `.hexen.yml` is authored content you hand-edit and want clean git diffs on; fog changes every session and would turn that history into noise. Same reasoning will apply to player/monster counters and anything else session-scoped later.

### 6a. `content` as a plugin point, not a special case

You flagged wanting to decouple Obsidian rendering into a plugin later, and pointed at exactly the right two spots — the top-level `content` block and each location's `content` block are the whole seam. Concretely:

- `packages/content-resolver` defines the interface: something like `resolveLocationContent(ref: string, projectContentConfig) => { title, summary, body }`, plus whatever a project's top-level `content` block needs to look like to configure it.
- `packages/content-obsidian` is the only implementation right now: it takes `{ vaultRoot }`, and for a given location's `content.ref`, reads that file relative to `vaultRoot` and parses YAML frontmatter (`title`, `summary`) + Markdown body — exactly what Quartz's own frontmatter parsing already does, independently reimplemented here rather than shared, since taking a runtime dependency on the Quartz toolchain for this would be the wrong direction of coupling.
- The server picks which resolver to use from the project's `content.type` (today, always `"obsidian"`) — a second backend later (inline content stored directly in `.hexen.yml`, say, or a different note-taking tool) is a second package implementing the same interface, not a fork of the format.

**Decided: every `content` block, top-level and per-location, carries its own `type`.** This matches the convention `grid` already uses (`type: hex | square`), so there's one pattern in the format for "this object is one of several kinds," not two. It costs a repeated `type: obsidian` on every location, but it means a location's `content` block is self-describing in isolation — decodable by a Zod `z.discriminatedUnion("type", [...])` without consulting the rest of the file, which is also just a better error message when it's wrong (`content.type` at this location isn't a recognized value, vs. a confusing failure two levels up when the resolver can't figure out what shape to expect).

## 7. Server

**Hono** — TypeScript-native, minimal, runs on Node or Bun without changes, and pairs cleanly with **tRPC** for the editor↔server API (save/load a location, list a project's locations) so you get end-to-end type inference from `hexen-schema` with no separate REST layer to keep in sync by hand. Fastify is the mature alternative if tRPC's magic ever feels like too much; I'd start with Hono+tRPC and only reach for Fastify if you hit something Hono genuinely can't do.

Realtime (GM → Presentation commands): plain `ws`, or Hono's own WebSocket helper. Socket.IO is the other option if you want rooms/reconnection handling out of the box — reasonable trade of a heavier dependency for less code, worth it once "online play" (multiple GM sessions, roadmap) is real; overkill for one GM and one presentation window on the same LAN.

Runtime: **Bun for local dev, Node in production** — Bun's fast startup and built-in TS/WebSocket support are worth it for the day-to-day authoring loop, while Node stays the boring, reliable choice for whatever's actually running during a live session. Hono runs unchanged on either, which is most of why it's the pick here.

## 8. Migrating off the current Obsidian vault

The single-file-plus-external-content design makes this lighter than the folder-per-location proposal it replaces: the vault itself barely moves. What actually needs to happen:

1. Write a one-time **importer script** (Node/TS, lives in this repo, not thrown away after use): for each `map-root: true` note under `site/vault/Locations/`, emit a `locations[]` entry with its `grid` (from `map-hex-*` frontmatter) and `image`. For every other note with `map`/`map-x`/`map-y` frontmatter, emit a `locations[]` entry (`content.ref` pointing at that note, unchanged) **and** a `links[]` entry on its parent map carrying the position/icon/color/type that used to live in that note's own frontmatter.
2. The importer then **rewrites those notes' frontmatter** to drop the now-migrated `map`/`map-x`/`map-y`/`map-type`/`map-icon`/`map-color` fields (they live in `.hexen.yml` now) — but leaves `title`, `summary`, `tags`, and the body completely alone. This is the one genuinely destructive step; review it by hand before committing.
3. Drop `.base` files entirely — replaced by hex-enductor's own browse/search (roadmap).
4. Copy each map's base image (today's `vault/_maps/<id>/map.png`) into this project's own `assets/` folder, and point each location's `image.file` at the copy — images don't stay referenced in the vault (§9).
5. Point `content.vaultRoot` at wherever `illuminated-world/site/vault` ends up living relative to the `.hexen.yml` file, run it once, review the diff (small vault, very doable), commit.

Because content never duplicates — it's a reference, not a copy — there's no on-going sync problem to design around here at all. The vault stays the single source of truth for prose; `.hexen.yml` is the single source of truth for space. The only one-way transformation is stripping the now-redundant `map-*` frontmatter, which only needs to happen once.

## 9. Decisions and open questions

Resolved:

- **No auth for v1** (local/LAN trust) — deliberate scope cut, revisit if "online play" ever becomes real.
- **Bun for dev, Node for production.**
- **`.hexen.yml` files are opened ad hoc**, like a document — `File > Open`, arbitrary path, a recent-files list — not managed in a dedicated directory with a project picker. `content.vaultRoot` and `image.file` both resolve relative to wherever that file happens to be.
- **Map images live in a hex-enductor-owned `assets/` folder**, not the vault's `_maps/<id>/` folders — see the `image.file` note in §6. Migration (§8) will need to copy each map image once rather than reference it in place.
- **Every `content` block carries an explicit `type`** — see the end of §6a.
- **`links[].id` referential integrity is deliberately unspecified for now** — do whatever's least work at implementation time (almost certainly: skip/warn on a dangling link rather than fail the whole load, since Zod validates each location's own shape independently anyway and a cross-referencing check is a separate, second pass). Revisit if broken links become an actual authoring annoyance.
- **A location can be linked from more than one parent map** — confirmed intentional, not just a side effect of the schema. No uniqueness constraint on `links[].id` across the file.

Still open:

- Where does the published wiki's map embed get its data from once this exists — does Quartz's build read `.hexen.yml` directly (point `MapData`'s emitter at it), or does hex-enductor export a `data.json` compatible with what `MapData`/`app.js` already expect, as a transitional step?
- Single-map-per-session assumption: does the Presentation view need to hold state for multiple concurrently-open maps (for map-stitching / sub-map click-through), or is "one active map, replaced on GM command" sufficient for v1?
