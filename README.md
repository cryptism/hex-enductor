# Hex Enductor
_Spare a little greed!_

[![Have *you* been to the English deer park?](https://img.youtube.com/vi/pcEajscsyQ0/0.jpg)](https://www.youtube.com/watch?v=pcEajscsyQ0)

Hex Enductor is a Leaflet-based GM map annotation and presentation tool.

WIP, alpha, and very very overly suited to mine own ends.

## What's in here

A Bun workspace monorepo. `.hexen.yml` (`docs/hexen.schema.json`) is the one file format everything below reads and writes.

| | |
|---|---|
| `packages/hexen-schema` | The `.hexen.yml` format itself — Zod schemas, a YAML parser with a warn-don't-fail referential-integrity pass, and a serializer. Also generates `docs/hexen.schema.json`. |
| `packages/content-resolver` | The interface a location's `content` block resolves through — title/summary/body, independent of where they actually come from. |
| `packages/content-obsidian` | The one implementation of that interface today: reads an Obsidian vault's YAML frontmatter + Markdown body directly. |
| `packages/map-core` | `<MapCanvas>`, a react-leaflet wrapper — the hex-grid math, the image overlay, the pins. Shared by the editor and (eventually) the wiki embed. |
| `apps/server` | Hono + tRPC. Opens a project (parses + resolves content), saves a link's patch back to disk, serves map images from the project directory. |
| `apps/editor` | The GM-facing app — open a project, click a pin, edit it, save. Vite + React + Zustand (client state) + tRPC/react-query (server state) + React Hook Form. |
| `examples/demo` | A tiny two-location `.hexen.yml` project exercising the whole loop — the thing to open first. |

No Presentation view yet, no Node production entrypoint (the Bun dev entrypoint and the eventual Node one both just wrap the same Hono `app` export), no square grids. See the repo's issues for what's actually planned — square grid support is the one thing marked high priority.

## Getting started

Requires [Bun](https://bun.sh) — `flake.nix` provides it (and Node, for whenever a production entrypoint exists) via `nix develop`, or install it yourself.

```sh
bun install

# in one terminal
bun run dev:server

# in another
bun run dev:editor
```

Open the editor (Vite will print the URL, typically `http://localhost:5173`) and give it the absolute path to `examples/demo/demo.hexen.yml`.

Other useful commands, run from the repo root:

```sh
bun run typecheck   # every package
bun run test        # every package's bun:test suite
bun run schema:docs # regenerate docs/hexen.schema.json after changing packages/hexen-schema
```

## The `.hexen.yml` format

`docs/hexen.schema.json` is generated from `packages/hexen-schema`'s Zod schemas (`bun run schema:docs`) — every field's description there comes straight from that source. A `.hexen.yml` file can reference it directly for editor validation/autocomplete:

```yaml
# yaml-language-server: $schema=./docs/hexen.schema.json
```

`examples/demo/demo.hexen.yml` is a small, valid, worked example if you'd rather read one than the schema.
