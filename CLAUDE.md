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

The plan is to port the TS apps to Rust/Leptos one at a time,
presentation first, then the editor. Workspace members:

- `crates/hexen-proto` — generated schema types, shared by hexend
  (native) and the wasm front ends. Rust clients deserialize hexend's
  messages with the same pbjson types hexend serializes them with, so
  **the `wireFormat.ts` seam doesn't exist on the Rust side**.
- `crates/map-core` — framework-free port of `packages/map-core`: hex/
  square grid geometry, fog cell addressing (keys must stay identical
  to `fog.ts` while the TS editor still writes them), Perlin fog
  texture, link icons, plus `viewport` (pan/zoom math). Natively
  testable: `cargo test -p map-core`.
- `apps/presentation-rs` — Leptos (CSR) port of `apps/presentation`,
  built with trunk. **No Leaflet**: the map (`src/map_canvas.rs`) is
  one SVG in image-pixel space (image, grid path, fog path) under a
  `Viewport` transform, with HTML pins/popups over it and pan/wheel/
  pinch done by hand with pointer events. Keep it that way for the
  editor port too.
  - `wasm-bindgen` is pinned `=0.2.127` to match nixpkgs'
    `wasm-bindgen-cli_0_2_127` in the flake — trunk needs the two
    identical, and trunk's own downloaded binary won't run on NixOS.
    Bump both together.
  - `apps/presentation` (TS) still exists until presentation-rs is
    signed off; then delete it.

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

## Running things for manual/browser testing

```
nix develop --command bash -c 'cd apps/hexend && cargo run'          # :4000
nix develop --command bash -c 'bun run --cwd apps/editor dev'        # vite picks a free port
nix develop --command bash -c 'bun run --cwd apps/presentation dev'  # same
nix develop --command bash -c 'cd apps/presentation-rs && trunk serve' # :5175
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
nix develop --command bash -c 'cargo test'                   # whole Cargo workspace
nix develop --command bash -c 'cargo clippy -p presentation-rs --target wasm32-unknown-unknown'
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
