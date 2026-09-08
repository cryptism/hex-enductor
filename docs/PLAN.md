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
    editor/           GM authoring app — Vite + React + TS
    presentation/      read-only player display — Vite + React + TS, much smaller
    server/            realtime + file I/O — TS, Hono
  packages/
    map-core/          <MapCanvas> component, hex-grid math, marker rendering — used by editor, presentation, AND the wiki embed
    map-schema/         Zod schemas for the package format (§6) — the single source of truth types are derived from
    ui/                 shared form controls / panel chrome, optional — only worth it once editor and presentation both need it
  docs/
    PLAN.md             this file
    ROADMAP.md
```

`map-core` is the piece that makes "three surfaces, one core" real: it's a dumb, mode-agnostic renderer (image + hex overlay + markers + popups) that takes a `readOnly` / `interactive` flag and an optional `liveState` (fog, pings) prop. The wiki embed becomes: import `map-core`, render it `readOnly`, no server, no websocket — same component the editor uses in edit mode, same component the presentation view uses with a websocket feed on top.

## 3. Framework choice: not Next.js

You asked me to actually weigh this rather than default to it, so: I'd recommend **Vite + React + TypeScript** (a plain SPA) for both `editor` and `presentation`, not Next.js.

Next.js earns its complexity when you need SSR/SSG for content pages, file-based routing across many pages, or SEO. None of that applies here — this is an interactive canvas tool with effectively one screen per app, used by you, not indexed by anyone. Next.js's App Router also blurs client/server component boundaries in a way that adds real mental overhead for a canvas-heavy, client-state-heavy app, for no corresponding benefit. A Vite SPA gives you a faster dev loop, a simpler mental model, and a trivial build output (static files) you can serve however you like.

Where Next.js (or Remix) *would* earn its place: if "online play" (roadmap, lesser priority) becomes a real hosted multi-tenant product later — multiple GMs, accounts, a marketing/docs site, server-rendered campaign pages for SEO. That's a genuinely different product shape. Worth reconsidering then, not now.

Briefly, on staying in React vs. not: canvas/pan-zoom apps with lots of small position updates (drag a marker, live fog brush strokes) are exactly the case where React's re-render model costs you something and SolidJS's fine-grained reactivity doesn't. I'm not recommending you leave React for this — the ecosystem win (react-leaflet, React Hook Form, this being the stack you already know) outweighs it for v1 — but if fog-of-war painting ever feels laggy, that's the direction to look, not "add more memoization."

**Leaflet itself**: keep it. It already does exactly what you need (`L.CRS.Simple` pixel-space maps, a mature plugin ecosystem, and you've already built calibration/hex/ruler logic against it). Wrap it in `react-leaflet` rather than replacing it — that gets you React component ergonomics (markers as JSX, popups as React portals) without discarding any of the existing map logic.

## 4. Forms and state

- **Forms**: React Hook Form + Zod (via `@hookform/resolvers`). The win here is specifically that the *same* Zod schema in `packages/map-schema` validates a location form on submit **and** defines the on-disk file format (§6) — one schema, not two things to keep in sync.
- **App/editor state** (selected tool, selected marker, calibration-in-progress, fog brush state): **Zustand**. Context alone gets expensive once you have frequent canvas-driven updates re-rendering large trees; Redux is more ceremony than this needs. Zustand (optionally with its `immer` middleware for ergonomic nested updates) is the common choice for exactly this shape of app — it's what Excalidraw-style editors tend to reach for.
- **Server state** (fetching/saving map packages, the realtime command channel): see §7 — paired with tRPC this mostly stops being a separate concern.

## 5. GM vs. Player roles

Worth being explicit about this now since it shapes the server's auth model, not just the UI:

| | GM (editor + presentation controller) | Player (presentation view) |
|---|---|---|
| Reads | Full location data, GM notes, hidden locations, fog state | Only non-hidden locations, only revealed fog areas, no GM notes |
| Writes | Everything — locations, calibration, fog, map switches | Nothing |
| Connection | Authenticated (even if that's just "the one person running the app locally") | Unauthenticated or session-token-only; joins a GM's session |
| Realtime | Sends commands (switch map, reveal fog, ping location, open dialog) | Receives commands, renders resulting state |

Concretely: the **Presentation view is a pure function of GM-broadcast state** — it holds no independent authority, doesn't fetch GM notes even if it could reach the server, and every "hidden" or "fogged" filter happens server-side before it reaches that client, not just visually hidden in the UI. That last point matters more than it sounds: if filtering happens client-side, a player opening devtools sees the GM notes anyway. Filter on the server.

## 6. Map package format

The current format is Obsidian-shaped in two specific ways worth separating: (a) *notes with frontmatter* — this part is good and not actually Obsidian-specific (Jekyll, Hugo, Astro content collections, and Obsidian all use plain YAML-frontmatter Markdown; it's a portable convention, not a proprietary one), and (b) *wikilinks and `.base` files* — this part genuinely is Obsidian-only and is what needs to go.

Proposal: **keep Markdown + YAML frontmatter per location**, **drop wikilinks and `.base` files**.

```
maps/
  vilheim-crace/
    map.yaml                  # manifest — see below
    assets/
      base.png
      thumb.png
    locations/
      pentegil-manor.md
      the-lord-commander.md
```

`map.yaml`:
```yaml
schemaVersion: 1
id: vilheim-crace
title: The Most Glorious Exarchates of Vilheim & Crace
image: { file: assets/base.png, width: 3840, height: 2160 }
hexGrid:
  origin: { x: 1706.96, y: 1758.02 }
  b1: { x: 84.99, y: 0.08 }
  b2: { x: 42.41, y: 73.49 }
  kmPerHex: 9
  style: { color: "#c19a5f", weight: 1, opacity: 0.45 }
links: []   # room for map-stitching (roadmap) — e.g. { toMapId, edge: "north", offset: 12 }
```

`locations/pentegil-manor.md`:
```markdown
---
id: pentegil-manor
type: settlement
icon: castle
x: 412
y: 233
color: null
hidden: false
subMap: null   # set to a map id if this location has its own map (roadmap: click-through)
---
Speak with the head of House Pentegil in Crace.

GM notes and read-aloud text live here, same as today.
```

Two deliberate omissions from that file: no `link:` frontmatter (that was already dead per your own commit history — the note *is* the page now), and no wikilinks anywhere — `subMap` and future cross-references are plain ids, resolved by `map-schema`, not by a wikilink resolver.

**Fog of war and other session state get their own file** (`fog.json`, sibling to `map.yaml`), not mixed into `map.yaml` or a location's frontmatter. This is a real distinction worth keeping: `map.yaml` and `locations/*.md` are *authored content* (hand-edited, meaningfully diffed in git, rarely conflicts), while fog state is *runtime state* that changes every session and would otherwise turn your content history into session-log noise. Same reasoning extends to anything else session-scoped later (player/monster counters, revealed-location log).

Format choice — Markdown+frontmatter over plain JSON or YAML-only: JSON has no comments and is unpleasant to hand-edit; a bare YAML file per location works but throws away the one thing Markdown gives you for free, which is a body for GM prose that doesn't need escaping. You already like this shape (it's why locations read so well as wiki pages) — the change is narrower than it might sound: stop being Obsidian's frontmatter dialect, start being *a* frontmatter dialect your own tool owns and parses (the same way Quartz already parses it today without needing Obsidian installed).

## 7. Server

**Hono** — TypeScript-native, minimal, runs on Node or Bun without changes, and pairs cleanly with **tRPC** for the editor↔server API (save/load a location, list a map's locations) so you get end-to-end type inference from `map-schema` with no separate REST layer to keep in sync by hand. Fastify is the mature alternative if tRPC's magic ever feels like too much; I'd start with Hono+tRPC and only reach for Fastify if you hit something Hono genuinely can't do.

Realtime (GM → Presentation commands): plain `ws`, or Hono's own WebSocket helper. Socket.IO is the other option if you want rooms/reconnection handling out of the box — reasonable trade of a heavier dependency for less code, worth it once "online play" (multiple GM sessions, roadmap) is real; overkill for one GM and one presentation window on the same LAN.

Runtime: Node is the safe default. Bun is worth a look for local dev (fast startup, built-in TS, built-in WebSocket) but I wouldn't commit to it for anything that needs to reliably run *during* a session — it's the newer, less-tested choice, and this is the one part of the stack where "boring and reliable" beats "fast."

## 8. Migrating off the current Obsidian vault

Because the only real breaking change is wikilinks/`.base` files, the migration is a single mechanical pass, not a rewrite:

1. Write a one-time **importer script** (Node/TS, lives in this repo, not thrown away after use): reads `site/vault/Locations/*.md` and the `_maps/<id>/` asset folders from `illuminated-world`, resolves each `[[Wikilink]]` to the target note's slug, drops the `.base` file (its one job — table browsing — becomes a first-class feature, roadmap item 3), and writes out the new `maps/<id>/` package structure.
2. Run it once, review the diff by hand (small vault, this is very doable), commit the result as the seed content for `hex-enductor`.
3. Decide the cutover model rather than drifting into it: either (a) **one-way cutover** — the new package format becomes canonical immediately, the Obsidian vault becomes a read-only historical artifact, or (b) **keep authoring in Obsidian for a while**, re-running the importer on demand as a one-way sync (Obsidian → package, never the reverse). I'd steer away from anything bidirectional — two live sources of truth for the same content is the actual sync-loss risk, not the format change itself. Given you're already deep into the illuminated-world Quartz build for the *published* wiki, (b) with periodic re-import is probably the pragmatic middle path until the new editor covers everything Obsidian currently does for you.

## 9. Open questions to settle before writing code

- Where does the published wiki's map embed get its data from once this exists — does Quartz's build read directly from `hex-enductor`'s package format (probably: point `MapData`'s emitter at a `maps/` folder in either repo), or does `hex-enductor` export a `data.json` compatible with what `MapData`/`app.js` already expect, as a transitional step?
- Single-map-per-session assumption: does the Presentation view need to hold state for multiple concurrently-open maps (for map-stitching / sub-map click-through), or is "one active map, replaced on GM command" sufficient for v1?
- Auth for the GM connection: local-only (no auth, trust the LAN) is fine for v1 and should be stated as a deliberate scope cut, not an oversight, if that's the plan.
