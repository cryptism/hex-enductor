# Hex Enductor
_Spare a little greed!_

[![Have *you* been to the English deer park?](https://img.youtube.com/vi/pcEajscsyQ0/0.jpg)](https://www.youtube.com/watch?v=pcEajscsyQ0)

Hex Enductor is a Leaflet-based TTRPG, annotation and presentation tool for GMs. It intends to support live, in-person play and supports on-the-fly editing of the mapped environment that syncs to pluggable content.

WIP, alpha, and very very overly suited to mine own ends. Feel free to swing by with features, bugs, all that whatnot, hopefully what's been done here is worthy to someone, even in our most decrepit futures to which we look forward in horror, more often than occasional, at the sight of our own ghosts gouged out of us and playing silly buggers among themselves.

## Getting started

Requires [Bun](https://bun.sh) — `flake.nix` provides it (and Node, for whenever a production entrypoint exists) via `nix develop`, or install it yourself.

```sh
bun install

# in one terminal
bun run dev:server

# in another
bun run dev:editor
```

Open `http://localhost:5173` and give it the absolute path to `examples/demo/demo.hexen.yml`.

## Structure

Hex Enductor projects are denoted by .hexen.yml and describe a DAG of `location` objects and one or more bits of `content` that the `<MapCanvas>` component resolves and renders via plugins. That's literally all it is at present!

I would like this project to be very plug-in-able, but currently the only plugin that will probably ever be supported is `content-obsidian`, which resolves Obsidian content from a vault and renders it.

Take a look at `examples/demo` to show how it works at present.

| | |
|---|---|
| `packages/hexen-schema` | The `.hexen.yml` format itself. |
| `packages/content-resolver` | Base interface for resolving location content |
| `packages/content-obsidian` | Reads and renders an Obsidian vault's YAML frontmatter + Markdown body directly. |
| `packages/map-core` | `<MapCanvas>`, a react-leaflet wrapper — the hex-grid math, the image overlay, the pins. Shared by the editor and (eventually) the wiki embed. |
| `apps/server` | Opens a .hexen.yml file. map images and other content from the project directory. |
| `apps/editor` | The GM-facing app — open a project, click a pin, edit it, save. |

## License

[MIT](LICENSE). One dependency, `react-leaflet`, ships under the
Hippocratic License 2.1 instead — not a conflict for MIT, but its own
notice needs to travel along if you redistribute a build.
