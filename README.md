# Hex Enductor
[___Spare a little greed!___](https://www.youtube.com/watch?v=pcEajscsyQ0)

![](hex-enductor.png)


__Hex Enductor__ is an annotation and GM presentation tool for TTRPGs. It intends to support live, in-person play and supports on-the-fly editing of the mapped environment that syncs to pluggable content.

WIP, alpha, and very very overly suited to mine own ends. Feel free to swing by with features, bugs, all that whatnot, hopefully what's been done here is worthy to someone, even in our most decrepit futures to which we look forward in horror, more often than occasional, at the sight of our own ghosts gouged out of us and playing silly buggers among themselves.

## Getting started

### Desktop app

`apps/desktop` bundles everything into one app: the editor in a native window, the server running inside it, and the presentation view for a second window or for other devices on your network (off by default; turn it on under **Server & presentation…**). Build it with [Tauri](https://tauri.app)'s CLI:

```sh
nix develop --command bash -c 'cd apps/desktop && cargo tauri build'
```

That produces installable bundles under `target/release/bundle/` (`.deb`, `.AppImage`, …) and the plain binary at `target/release/hex-enductor`.

### Quickstart: hex-enductor.pages.dev

The hosted editor at [hex-enductor.pages.dev](https://hex-enductor.pages.dev) can open a project folder directly in the browser — no install, no server, nothing to run. Click **Open from this browser…**, pick a folder with a `.hexen.yml` in it, and you're editing. This needs a Chromium-based browser (Chrome, Edge) — Firefox and Safari don't support the underlying File System Access API yet.

If your project uses an Obsidian vault for content, or your browser doesn't support that API, run the local server and connect to it from the hosted page (in a browser on the same machine — the page talks to `http://localhost:4000`):

```sh
nix develop --command bash -c 'cd apps/hexend && cargo run'
```

The deploy is hand-rolled in Cloudflare rather than something CI keeps in lockstep with `master` — if it's behaving oddly, the local setup below is the ground truth.

### Local setup

Everything is Rust: one Cargo workspace, with the two browser apps built to WebAssembly by [trunk](https://trunkrs.dev). `nix develop` provides the toolchain (see `flake.nix`); otherwise you need `rustc`/`cargo` with the `wasm32-unknown-unknown` target, `trunk`, and `wasm-bindgen-cli` 0.2.127.

```sh
# the server, on http://localhost:4000
cd apps/hexend && cargo run

# the editor, on http://localhost:5173
cd apps/editor && trunk serve

# the presentation view, on http://localhost:5174
cd apps/presentation && trunk serve
```

Open the editor and give it the absolute path to `examples/demo/demo.hexen.yml`. For the presentation view, append `?server=http://localhost:4000&path=<absolute path to the .hexen.yml>` to its URL.

The server only listens on localhost: it has no authentication, and it can open and write files anywhere you can. `HOST=0.0.0.0` exposes it to your network if you really want that; the desktop app's network sharing is the safer way to put a map on another screen, since it only exposes a read-only view.

### Migrating from Obsidian

If your maps currently live in an Obsidian vault's frontmatter (`map-root`/`map-x`/`map-y`), `hexen import-obsidian` turns the vault into a `.hexen.yml` project, and `hexen strip-obsidian-frontmatter` then removes the migrated fields from the notes:

```sh
cargo run -p hexen-cli -- import-obsidian --vault ~/my-vault --out ~/my-realm/realm.hexen.yml --title "My Realm"
cargo run -p hexen-cli -- strip-obsidian-frontmatter --vault ~/my-vault --dry-run
```

Projects saved by the older TypeScript tools use a slightly different format; `cargo run -p hexen-cli -- migrate <file.hexen.yml>` converts them.

## Structure

Hex Enductor projects are denoted by .hexen.yml and describe a DAG of `location` objects and one or more bits of `content` that the map resolves and renders. That's literally all it is at present! The format is defined in `schema/hexen/v1/*.proto`; `docs/hexen.schema.json` is generated from it for editor completion.

A location's content comes from one of two places: an Obsidian vault (a vault file's frontmatter + body), or written directly inline in the `.hexen.yml` itself — no vault required. The two mix freely within one project; inline is the escape hatch for a location that doesn't warrant its own vault file, or for a project that doesn't want a vault at all. I would like this to be more plug-in-able beyond that, but haven't needed to yet.

Take a look at `examples/demo` to show how it works at present — its `notice-board` location is inline, `town` and `inn` are Obsidian.

| | |
|---|---|
| `crates/hexen-proto` | The `.hexen.yml` format itself, generated from `schema/hexen/v1/*.proto`. |
| `crates/project-ops` | Everything about a project that doesn't touch a disk or network: parsing, the edit commands, undo history, Obsidian note parsing. Shared by the server and the editor. |
| `crates/map-core` | Map maths: hex/square grids, fog-of-war cells, the fog texture, pan/zoom, the link icon set. |
| `crates/hexen-web` | The map component and the live-session client, shared by the editor and presentation apps. |
| `apps/hexend` | The local server — opens a .hexen.yml file, serves map images, and pushes live state to every connected client over its `/ws` session. |
| `apps/editor` | The GM-facing app — open a project (from the server, or straight from a folder in the browser), click a pin, edit it. In GM mode it also controls fog of war. |
| `apps/presentation` | A read-only display for a second monitor/projector — follows the editor's live state, fog included. |
| `apps/desktop` | All of the above as one desktop app. |
| `apps/hexen-cli` | `hexen`: Obsidian import, format migration, and regenerating the link icons and JSON Schema. |

## License

[MIT](LICENSE).
