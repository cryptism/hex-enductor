# Hex Enductor
[___Spare a little greed!___](https://www.youtube.com/watch?v=pcEajscsyQ0)

__Hex Enductor__ is a Leaflet-based annotation and GM presentation tool for TTRPGs. It intends to support live, in-person play and supports on-the-fly editing of the mapped environment that syncs to pluggable content.

WIP, alpha, and very very overly suited to mine own ends. Feel free to swing by with features, bugs, all that whatnot, hopefully what's been done here is worthy to someone, even in our most decrepit futures to which we look forward in horror, more often than occasional, at the sight of our own ghosts gouged out of us and playing silly buggers among themselves.

## Getting started

### Quickstart: hex-enductor.pages.dev

The editor itself is just a static UI, hosted at [hex-enductor.pages.dev](https://hex-enductor.pages.dev) — everything that touches your files (opening a project, browsing directories, saving edits) happens in a small server you run yourself. So the fastest path in only needs that server, not the whole frontend toolchain:

```sh
bun install
bun run dev:server
```

Then open [hex-enductor.pages.dev](https://hex-enductor.pages.dev) **in a browser on that same machine** and give it the absolute path to `examples/demo/demo.hexen.yml`, or one of your own projects.

That "same machine" part matters: the hosted page talks to `http://localhost:4000` by default, which only resolves to your server if the browser and the server are on the same box. It's also a hand-rolled Cloudflare Pages deploy rather than something CI keeps in lockstep with `master` — if it's behaving oddly, the full local setup below is the ground truth.

### Full local setup

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

A location's content comes from one of two places: an Obsidian vault (`content-obsidian`, resolving a vault file's frontmatter + body), or written directly inline in the `.hexen.yml` itself — no vault required. The two mix freely within one project; inline is the escape hatch for a location that doesn't warrant its own vault file, or for a project that doesn't want a vault at all. I would like this to be more plug-in-able beyond that, but haven't needed to yet.

Take a look at `examples/demo` to show how it works at present — its `notice-board` location is inline, `town` and `inn` are Obsidian.

| | |
|---|---|
| `packages/hexen-schema` | The `.hexen.yml` format itself. |
| `packages/content-resolver` | Base interface for resolving location content |
| `packages/content-obsidian` | Reads and renders an Obsidian vault's YAML frontmatter + Markdown body directly. |
| `packages/map-core` | `<MapCanvas>`, a react-leaflet wrapper — the hex-grid math, the image overlay, the pins. Shared by the editor and (eventually) the wiki embed. |
| `apps/server` | Opens a .hexen.yml file. map images and other content from the project directory. |
| `apps/editor` | The GM-facing app — open a project, click a pin, edit it, save. |

## License

[MIT](LICENSE).

We recognise and support `react-leaflet`'s use of Hippocratic License 2.1
