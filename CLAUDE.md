# hex-enductor — working notes

All Rust, one Cargo workspace (`Cargo.toml`, one `Cargo.lock`, one
`target/`). The TypeScript apps and packages were removed in 2026-09;
don't go looking for `packages/*` or `*.tsx` — git history has them if
you need to know what something used to do.

Run everything through `nix develop --command bash -c '...'` — that's
where trunk, the pinned wasm-bindgen, and the desktop app's GTK/WebKit
libraries live. `protoc` is **not** needed (see hexen-proto below).

## Layout

- `schema/hexen/v1/*.proto` — the project format and the /ws protocol.
  The `.hexen.yml` on disk is protobuf's canonical JSON mapping, as
  YAML (pbjson): lowerCamelCase fields, a `oneof` written as
  `{ <member>: value }` (e.g. `grid: { hex: {...} }`, `content:
  { obsidian: { ref: … } }`), zero-valued scalars omitted, unknown
  fields rejected.
- `crates/hexen-proto` — prost + pbjson codegen of the protos. Compiles
  them with protox (pure Rust). Also embeds the descriptor set
  (`FILE_DESCRIPTOR_SET`) for tools that work from the schema.
- `crates/project-ops` — everything about a project that touches no
  disk or socket: YAML parse/serialize/validate (`document`), the
  Command reducer (`commands`, `mutations`), undo `History`, Obsidian
  note parsing (`content`), image sizing. hexend wraps it with tokio::fs
  and the WS session; the editor's browser-folder storage runs the same
  code in wasm. One implementation, one format.
  - `LinkPatch.icon`/`color` set to `""` **clear** the field (proto3
    JSON can't send null).
- `crates/map-core` — framework-free map maths: hex/square grids, fog
  cells, Perlin fog texture, link icons (`link_icons.rs`, generated —
  see hexen-cli), `viewport` (pan/zoom).
- `crates/hexen-web` — Leptos pieces the two browser apps share:
  `MapCanvas` and the /ws `live_session` client, plus `map.css`.
  **No Leaflet**: the map is one SVG in image-pixel space (image, grid
  path, fog path) under a `Viewport` transform, with HTML pins/popups
  over it; pan/wheel/pinch and the editor tools (click-to-place, fog
  painting) are done by hand with pointer events. Keep it that way.
- `apps/hexend` — the server (axum). `PORT` (default 4000), `HOST`
  (default **127.0.0.1**; it has no auth and can open/write/list files
  anywhere, so it's loopback unless you ask). `server::app` is the full
  API; `server::viewer_app` is a read-only surface for other devices
  (a /ws that only attaches to already-open projects and drops client
  messages, no listing/creation/uploads). On both, `/image` reads and
  uploads only work inside an open project's directory.
- `apps/editor` — the editor (Leptos CSR, trunk, :5173).
  `ui_state.rs` holds every "which switch resets which" rule as plain
  methods (tested natively). `storage/` has two backends:
  - server: `hexen_web::live_session` + plain HTTP for images;
  - browser folder (File System Access API, Chromium only): runs
    project-ops in wasm, notifies from memory immediately, and writes
    the file in the background, **coalescing** writes so a fast fog
    stroke can't land an older state last. Old-format files get an
    error pointing at `hexen migrate`.
  - `showDirectoryPicker`/`requestPermission` are bound by hand in
    `storage/local_fs.rs` (web-sys gates them behind
    `web_sys_unstable_apis`; don't turn that cfg on for the build).
  - hexend's URL: the desktop app's (see below) when running inside
    it, else baked in at build time, `HEXEND_URL=… trunk build`
    (default `http://localhost:4000`).
- `apps/presentation` — read-only live view for a second screen (trunk,
  :5174), driven by `?server=…&path=…`.
- `apps/desktop` — Tauri 2; see below.
- `apps/hexen-cli` — the `hexen` binary: `import-obsidian`,
  `strip-obsidian-frontmatter`, `migrate` (old "type:" format →
  current), `sync-link-icons` (regenerates `link_icons.rs`), `schema`
  (regenerates `docs/hexen.schema.json` from the proto descriptors —
  a test fails if the checked-in file is stale, so run it after any
  `.proto` change). `cargo run -p hexen-cli -- <command> --help`.

`wasm-bindgen` is pinned `=0.2.127` (hexen-web, editor) to match
nixpkgs' `wasm-bindgen-cli_0_2_127` in the flake — trunk needs the two
identical, and trunk's own downloaded binary won't run on NixOS. Bump
them together.

## Desktop app: `apps/desktop` (Tauri 2)

One binary: the editor in a native window, hexend **in-process** (it's
a library — no sidecars), and the presentation app served to extra
windows and, opt-in, the LAN.

- `src/servers.rs` is the substance (natively tested):
  - local server: `hexend::server::app` on 127.0.0.1, random port, for
    the editor and local presentation windows;
  - LAN server (off by default, toggled from the editor):
    `viewer_app` on 0.0.0.0, port 4747 if free;
  - both serve the presentation app (built with `--public-url
    /presentation/` into `apps/desktop/presentation-dist/`, embedded
    via rust-embed) under `/presentation/`.
- `src/lib.rs`: Tauri wiring. The editor window gets
  `window.__HEXEN_DESKTOP__ = { serverUrl, lanUrl, version }` via an
  initialization script — **that global is the feature flag**
  (`apps/editor/src/desktop.rs`): with it, the editor talks to the
  embedded server, the picker offers the native file dialog instead of
  the browser-folder backend, and the sidebar gets "Server &
  presentation…". The same editor build works in a browser without it.
  Commands: `server_info`, `set_lan_sharing`, `open_presentation_window`,
  `pick_project_file`. Only the `main` window has IPC capabilities;
  presentation windows are plain http:// pages.
- Not in the workspace's `default-members` (needs WebKitGTK and the
  front-end builds), so root `cargo test` skips it.
- Build: `cd apps/desktop && cargo tauri build` (its beforeBuildCommand
  runs both trunk builds; `--no-bundle` for just the binary, `--bundles
  deb` etc. for packages). The binary is `target/{debug,release}/hex-enductor`.

## Fog of war (issue #6)

- `Location.fog` — presence (even empty) means fog is on;
  `FogOfWar.revealedCells` — opaque cell keys, meaningless to the server.
- Commands: `setFog` (start with `{revealedCells: []}` / clear with
  absent) and `setFogCells` (batch-sets a list of cells' revealed state
  — plural and idempotent so a click-drag stroke lands as one command
  per ~80ms tick, not one per cell).
- Cell addressing is a **fixed 64px grid over the image**, independent
  of the terrain grid — `crates/map-core/src/fog.rs`. The keys are in
  saved projects, so the scheme can't change.
- **Control lives in the editor**, via two independent switches
  (`ui_state.rs`):
  - GM mode alone: a read-only, Krita-style "layers panel" — a fog-layer
    visibility checkbox that only affects the GM's own view, plus a
    status line that always states what players currently see
    (`fog_controls.rs`).
  - GM mode + edit mode: the "Paint fog" tool (click-drag reveals,
    shift at stroke start restores) and "Fog/Reveal entire map" behind
    a confirm. Holding shift while painting dims the fog overlay
    (cosmetic only). Strokes fill in every cell between pointer
    samples.
- The presentation app is purely read-only: it renders `location.fog`
  like any other state. Don't add GM controls there; they'd duplicate
  `fog_controls.rs`.
- **Known rough edge:** pins are HTML over the map's SVG, so they stay
  visible through fog for GM and players alike. Fixing it means hiding
  pins whose cell is hidden, in `MapCanvas`.

## Running things for manual/browser testing

```
nix develop --command bash -c 'cd apps/hexend && cargo run'         # :4000
nix develop --command bash -c 'cd apps/editor && trunk serve'       # :5173
nix develop --command bash -c 'cd apps/presentation && trunk serve' # :5174
nix develop --command bash -c 'cd apps/desktop && cargo tauri build --no-bundle && ../../target/release/hex-enductor'
```

Editor: paste the absolute path to a `.hexen.yml` (e.g.
`examples/demo/demo.hexen.yml`) into the picker. Presentation: append
`?server=http://localhost:4000&path=<ABSOLUTE path>` to its URL. hexend
keeps sessions in memory for its lifetime — restart it after editing a
project file by hand.

- Browser-testing the folder backend: Playwright can't drive the native
  picker, so seed OPFS (`navigator.storage.getDirectory()`) and stub
  `window.showDirectoryPicker` to return that directory via
  `addInitScript` — the app code runs unchanged.
- Headless desktop checks in a container: `Xvfb :99`, run the binary
  under `dbus-launch` with `WEBKIT_DISABLE_COMPOSITING_MODE=1`, drive it
  with `xdotool`, screenshot with ImageMagick's `import -window root`.

## Testing

```
nix develop --command bash -c 'cargo test'                           # workspace, minus desktop
nix develop --command bash -c 'cargo test -p hex-enductor-desktop'   # needs presentation-dist built
nix develop --command bash -c 'cargo clippy --workspace --all-targets'
nix develop --command bash -c 'cargo clippy -p editor -p presentation --target wasm32-unknown-unknown'
nix develop --command bash -c 'cd schema && buf lint'
```

`apps/hexend` isn't rustfmt-clean and has never been: don't run
`cargo fmt` over it (or the whole workspace) as part of an unrelated
change — format the crates you touched, `-p <crate>`.

## Issue tracking

Feature gaps get filed as GitHub issues (`gh issue create`), not just
mentioned in chat — run `gh issue list --state all` before starting a
review so you don't re-flag something already tracked.
