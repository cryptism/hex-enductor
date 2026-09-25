# hex-enductor — working notes

## One server: `apps/hexend` (Rust)

`apps/hexend` used to be TypeScript (Hono/tRPC); as of 2026-09-17 it's a
from-scratch Rust rewrite (axum), consolidated from a parallel
`apps/hexend-rs` that existed briefly during the migration. There is no
TS server anymore — don't go looking for `apps/hexend/src/*.ts`.

- Its message schema is defined in `schema/hexen/v1/*.proto`.
  `crates/hexen-proto` codegens it into Rust (prost for the structs,
  pbjson for protobuf's canonical JSON mapping — see "wire-format
  gotcha" below); hexend re-exports it as `crate::pb`. It compiles the
  protos with protox (pure Rust), so **no `protoc` needed** to build.
- Run things through `nix develop --command bash -c '...'` anyway —
  that's where the wasm toolchain for `apps/presentation-rs` lives.
- Listens on **port 4000** by default (`PORT` env var to override).
- `bun run dev:hexend` from the repo root runs `cargo run
  --manifest-path apps/hexend/Cargo.toml` — it's a plain Cargo command
  wrapped in a bun script, not a bun-native dev server.
- Not part of the bun workspace (`package.json`'s `workspaces` only
  globs `packages/*`/`apps/*` for *.json-having packages that bun
  resolves — a Cargo crate with no package.json is invisible to
  `bun run --filter '*'`). It's a member of the root **Cargo
  workspace** (`Cargo.toml`, one `Cargo.lock`, one `target/`) instead.

## Rust front ends: moving off TypeScript

The TS apps are being ported to Rust/Leptos one at a time; both
front ends now have Rust versions. Workspace members:

- `crates/hexen-proto` — generated schema types, shared by hexend
  (native) and the wasm front ends. Rust clients deserialize hexend's
  messages with the same pbjson types hexend serializes them with, so
  **the `wireFormat.ts` seam doesn't exist on the Rust side**.
- `crates/project-ops` — everything about a project that touches no
  disk or socket: YAML parse/serialize/validate, the Command reducer,
  undo `History`, Obsidian note parsing, image sizing. hexend wraps it
  with tokio::fs + the WS session; editor-rs's browser-folder storage
  runs the same code in wasm. One implementation, one YAML dialect.
  - `LinkPatch.icon`/`color` set to `""` **clear** the field (proto3
    JSON can't send null). The TS editor's "None" icon/blank color were
    silently ignored by hexend for exactly that reason.
- `crates/map-core` — framework-free port of `packages/map-core`: hex/
  square grid geometry, fog cell addressing (keys must stay identical
  to `fog.ts` while the TS editor still writes them), Perlin fog
  texture, link icons, plus `viewport` (pan/zoom math). Natively
  testable: `cargo test -p map-core`.
- `crates/hexen-web` — Leptos pieces both front ends share: `MapCanvas`
  and the /ws `live_session` client, plus `map.css`. **No Leaflet**:
  the map is one SVG in image-pixel space (image, grid path, fog path)
  under a `Viewport` transform, with HTML pins/popups over it and
  pan/wheel/pinch and the editor tools (click-to-place, fog painting)
  done by hand with pointer events. Keep it that way.
- `apps/presentation-rs` — port of `apps/presentation` (trunk, :5175).
- `apps/editor-rs` — port of `apps/editor` (trunk, :5176). `ui_state.rs`
  is the zustand store as plain methods (tested natively); `storage/`
  is ProjectStorage with two backends:
  - server: `hexen_web::live_session` + plain HTTP for images;
  - browser folder (File System Access API, Chromium only): runs
    project-ops in wasm, notifies from memory immediately, and writes
    the file in the background, **coalescing** writes so a fast fog
    stroke can't land an older state last. This backend now reads and
    writes hexend's dialect — the old "two dialects" problem below is
    gone on the Rust side; old-dialect files get an error pointing at
    `scripts/migrate-project-to-wire-format.ts`.
  - `showDirectoryPicker`/`requestPermission` are bound by hand in
    `storage/local_fs.rs` (web-sys gates them behind
    `web_sys_unstable_apis`; don't turn that cfg on for the build).
  - hexend's URL is baked in at build time: `HEXEND_URL=… trunk build`
    (default `http://localhost:4000`).
- `wasm-bindgen` is pinned `=0.2.127` (hexen-web and editor-rs) to
  match nixpkgs' `wasm-bindgen-cli_0_2_127` in the flake — trunk needs
  the two identical, and trunk's own downloaded binary won't run on
  NixOS. Bump all of them together.
- `apps/presentation` and `apps/editor` (TS) still exist until the Rust
  versions are signed off; then delete them, and `packages/*` with them.

## Desktop app: `apps/desktop` (Tauri 2)

One binary: the editor in a native window, hexend **in-process** (it's
a library — no sidecars), and the presentation app served to extra
windows and, opt-in, the LAN.

- `src/servers.rs` is the substance (natively tested):
  - local server: `hexend::server::app` on **127.0.0.1**, random port,
    for the editor + local presentation windows;
  - LAN server (off by default, toggled from the editor): hexend's
    `viewer_app` on 0.0.0.0, port 4747 if free — read-only /ws that
    only attaches to projects already open locally, images only from
    those projects' dirs, no /directory, /project or uploads.
  - both serve presentation-rs (built with `--public-url
    /presentation/` into `apps/desktop/presentation-dist/`, embedded
    via rust-embed) under `/presentation/`.
- `src/lib.rs`: Tauri wiring. The editor window gets
  `window.__HEXEN_DESKTOP__ = { serverUrl, lanUrl, version }` via an
  initialization script — **that global is the feature flag**
  (`apps/editor-rs/src/desktop.rs`): with it, the editor talks to the
  embedded server, the picker offers the native file dialog instead of
  the browser-folder backend, and the sidebar gets "Server &
  presentation…". Same editor build works in a browser without it.
  Commands: `server_info`, `set_lan_sharing`, `open_presentation_window`,
  `pick_project_file`. Only the `main` window has IPC capabilities;
  presentation windows are plain http:// pages.
- Not in the workspace's `default-members` (needs WebKitGTK and the
  front-end builds), so root `cargo test` skips it; test it with
  `cargo test -p hex-enductor-desktop`.
- Build: `cd apps/desktop && cargo tauri build` (its beforeBuildCommand
  runs both trunk builds; `--no-bundle` for just the binary, `--bundles
  deb` etc. for packages). The binary is `target/{debug,release}/hex-enductor`.
- Headless checks in a container: `Xvfb :99`, run the binary under
  `dbus-launch` with `WEBKIT_DISABLE_COMPOSITING_MODE=1`, drive it with
  `xdotool`, screenshot with ImageMagick's `import -window root`.

**hexend's standalone binary still binds 0.0.0.0 with no auth**, and
its `GET /image` takes `dir` from the caller (so it can read any file
the process can) — pre-existing, not changed here; the desktop app
avoids it by keeping the full API on loopback. Worth fixing in the
standalone server too (bind 127.0.0.1 by default, or restrict `dir`
to open projects like `viewer_app` does).

Browser-testing the folder backend: Playwright can't drive the native
picker, so seed OPFS (`navigator.storage.getDirectory()`) and stub
`window.showDirectoryPicker` to return that directory via
`addInitScript` — the app code runs unchanged.

**Latent TS bug the Rust port surfaced:** proto3 JSON omits
zero-valued scalars (e.g. `b1: {x: 60}` with no `y`), and
`wireFormat.ts` doesn't restore defaults, so `hexMath.ts` computes on
`undefined` → NaN polygons → no grid at all in the TS presentation
(and anywhere else a zero coordinate lands). Not fixed on the TS side;
prost defaults make it a non-issue in Rust.

## Schema: still mid-migration to protobuf

**`packages/hexen-schema` (hand-written Zod) has not been regenerated
from proto** — it's the same shapes, modeled the old "type"-discriminated
way, and is what `map-core`, `project-ops`, and the editor's in-memory
project representation all still use. There's a referenced-but-not-yet-built
`packages/hexen-proto-ts` that would eventually replace it and make the
wire-format seam below unnecessary.

### The wire-format gotcha (read this before touching Command/Grid/Content)

protobuf's canonical JSON mapping serializes a `oneof` as
`{ <selected field name>: value }`. Zod's discriminated unions (what
`hexen-schema` and everything above it uses) look like
`{ type: "...", ...fields }`. These are genuinely different shapes for
every oneof-backed type: `Grid` (hex/square), `ProjectContent` and
`LocationContent` (obsidian/inline), and the `Command` union, plus the
`ClientMessage`/`ServerMessage` envelope itself.

**`packages/live-session/src/wireFormat.ts` is the one seam that
translates between them**, used by:
- `packages/live-session/src/liveSession.ts` — every WS message in
  and out of `connectLiveSession`.
- `apps/editor/src/ProjectPicker.tsx` — the "New project" POST body's
  `content` field (via `projectContentToWire`).

If you add a new oneof-shaped field to a `.proto` file, or a new
`Command` variant, **you must add/extend a converter in
`wireFormat.ts`** or anything going through `connectLiveSession` (or
project creation) will fail silently — no error, just a hung
"Connecting…" or a 400 from the server. `wireFormat.test.ts` round-trips
every existing converter — extend it alongside the schema. Only convert
what's actually used: this module briefly carried a full reverse
direction (`openedProjectDataToWire`, `commandFromWire`,
`locationContentToWire`) for the old TS server to speak wire format
back; that's gone now that hexend is Rust-only and gets the wire shape
for free from prost/pbjson — don't resurrect it without a real caller.

Plain (non-oneof) fields need no conversion: protobuf JSON's default
camelCase field naming already matches `hexen-schema`'s field names
(`locationId`, `revealedCells`, `distancePerCell`, etc.).

### A second, un-migrated dialect: local-fs storage

`apps/editor`'s browser-native storage backend (File System Access API,
`storage/localFsStorage.ts`) reads and writes `.hexen.yml` files
directly via `hexen-schema`'s `parseHexenProject`/`serializeHexenProject`
— the **old** "type"-discriminated YAML shape, not hexend's oneof-shaped
one. A project file touched by the server and a project file touched by
local-fs storage are, right now, two different on-disk dialects that
can't necessarily read each other back (this is exactly the bug that
made the old TS `apps/hexend` unable to open `examples/demo/demo.hexen.yml`
after it was migrated — see git history around 2026-09-17). Not fixed;
flagged as a real follow-up. Fixing it properly means either migrating
`hexen-schema` itself to the oneof convention, or giving
`localFsStorage.ts` a `wireFormat.ts`-style conversion step of its own.
**Fixed in `apps/editor-rs`** (it uses crates/project-ops, i.e. the
server's dialect); only the TS editor still has this problem, and it
goes away when the TS editor is deleted.

## Running things for manual/browser testing

```
nix develop --command bash -c 'cd apps/hexend && cargo run'          # :4000
nix develop --command bash -c 'bun run --cwd apps/editor dev'        # vite picks a free port
nix develop --command bash -c 'bun run --cwd apps/presentation dev'  # same
nix develop --command bash -c 'cd apps/presentation-rs && trunk serve' # :5175
nix develop --command bash -c 'cd apps/editor-rs && trunk serve'       # :5176
nix develop --command bash -c 'cd apps/desktop && cargo tauri build --no-bundle && ../../target/release/hex-enductor'
```

Editor: open the picker, paste the absolute path to a `.hexen.yml`
(e.g. `examples/demo/demo.hexen.yml`) under "open a project on a
locally-running server". Presentation (either one): append
`?server=http://localhost:4000&path=<ABSOLUTE path>` to its URL — it's
read-only, driven by whatever the editor does.

## Fog of war (issue #6) — landed 2026-09-17

- `Location.fog: FogOfWar | null` — presence (even empty) means fog is
  on. `FogOfWar.revealedCells: string[]` — opaque cell keys, meaningless
  to the server.
- Commands: `setFog` (start with `{revealedCells:[]}` / clear with
  `null`) and `setFogCells` (batch-sets a list of cells' revealed state
  explicitly — plural and idempotent-by-design so a click-drag paint
  stroke lands as one command per ~80ms tick, not one per cell).
- Cell addressing is a **fixed 64px grid over the image**, independent
  of whatever terrain grid (hex/square/none) the Location has — see
  `packages/map-core/src/fog.ts`.
- **Control lives in the editor** (`apps/editor`), via two independent
  toggles in `store.ts`/`App.tsx`:
  - `gmMode` alone: a read-only, Krita-style "layers panel" — a
    fog-layer visibility checkbox that only affects the GM's own view
    (never sends a command), plus a status line that always states
    what players currently see, regardless of that local peek. See
    `FogControls.tsx`.
  - `gmMode` + `editMode` together: unlocks the "Paint fog" tool
    (armed like Add Location — click-drag reveals, shift reverses to
    restore, decided once per stroke) and "Fog entire map"/"Reveal
    entire map" blanket actions behind a confirm modal.
  - Holding shift while painting also live-dims the fog overlay so you
    can see what you're about to re-cover (`MapCanvas.tsx`'s
    `shiftHeld` state, cosmetic only — doesn't touch the paint logic).
- `apps/presentation` is pure read-only: it renders `location.fog` like
  any other state, no editing surface. GM control was briefly built
  there too (`?gm=1`) before moving to the editor — don't resurrect
  that path without a reason, it'd duplicate `FogControls.tsx`.
- **Known rough edge:** Leaflet renders markers in `markerPane`, which
  stacks above the fog overlay's pane — link pins stay visible through
  fog for both GM and player views. Not addressed; would need either a
  custom pane order or hiding markers under hidden cells explicitly in
  `MapCanvas.tsx`.

## Testing

```
nix develop --command bash -c 'bun run --filter "*" test'   # every TS workspace
nix develop --command bash -c 'cargo test'                   # Cargo workspace (minus desktop)
nix develop --command bash -c 'cargo test -p hex-enductor-desktop'  # needs presentation-dist built
nix develop --command bash -c 'cargo clippy -p presentation-rs -p editor-rs --target wasm32-unknown-unknown'
nix develop --command bash -c 'bun run typecheck'
```

Known pre-existing (unrelated) typecheck failure:
`scripts/sync-link-icons.ts:81` — `TS2532: Object is possibly
'undefined'`. Not caused by any hexend/fog work; low priority, hasn't
been triaged.

## Issue tracking

Feature gaps get filed as GitHub issues (`gh issue create`), not just
mentioned in chat — run `gh issue list --state all` before starting a
review so you don't re-flag something already tracked.
