# hex-enductor — working notes

## Two servers, one in-flight migration

- `apps/hexend` (TypeScript, Hono/tRPC) is the original server. Its WS
  session (`src/session.ts`) speaks an ad hoc envelope:
  `{type:"command"|"undo"|"redo", ...}` in, `{type:"state",data:...}` out.
- `apps/hexend-rs` (Rust, axum) is a from-scratch rewrite — **the
  intended production/presentation server** — currently untracked/WIP.
  It shares no code with `apps/hexend`; each command/mutation is
  hand-ported (see the `//! Port of ...` doc comment at the top of each
  `apps/hexend-rs/src/*.rs` file — keep both sides in sync by hand when
  editing either one).
- Building/testing `apps/hexend-rs` requires `protoc`/`buf`, which only
  exist in the nix devShell: run everything through
  `nix develop --command bash -c '...'`, not directly.
- `apps/hexend-rs` listens on **port 4001** by default (`PORT` env var
  to override) — not 4000.

## Schema: mid-migration to protobuf

`schema/hexen/v1/*.proto` is the canonical schema definition now.
`apps/hexend-rs/build.rs` codegens it into Rust (prost for structs,
pbjson for protobuf's canonical JSON mapping). **`packages/hexen-schema`
(hand-written Zod) has not been regenerated from proto** — it's the
same shapes, modeled the old way, and is what `map-core`, `project-ops`,
and the editor all still import. There's a referenced-but-not-yet-built
`packages/hexen-proto-ts` that would eventually replace it.

### The wire-format gotcha (read this before touching Command/Grid/Content)

protobuf's canonical JSON mapping serializes a `oneof` as
`{ <selected field name>: value }`. Zod's discriminated unions (what
`hexen-schema` and everything above it uses) look like
`{ type: "...", ...fields }`. These are genuinely different shapes for
every oneof-backed type: `Grid` (hex/square), `ProjectContent` and
`LocationContent` (obsidian/inline), and the `Command` union, plus the
`ClientMessage`/`ServerMessage` envelope itself.

**`packages/live-session/src/wireFormat.ts` is the one seam that
translates between them.** It was missing until 2026-09-17 — before
that, `packages/live-session` (and therefore `apps/presentation`) sent
and expected `apps/hexend`'s ad hoc envelope, which `apps/hexend-rs`
does not speak, so the presentation app silently hung forever at
"Connecting…" with no error. If you add a new oneof-shaped field to a
`.proto` file, or a new `Command` variant, **you must add/extend a
converter in `wireFormat.ts`** or anything going through
`connectLiveSession` will fail the same silent way. `wireFormat.test.ts`
round-trips every existing converter — extend it alongside the schema.

Plain (non-oneof) fields need no conversion: protobuf JSON's default
camelCase field naming already matches `hexen-schema`'s field names
(`locationId`, `revealedCells`, `distancePerCell`, etc.).

As of now, **`apps/presentation` (via `live-session`) only works
against `apps/hexend-rs`**, not `apps/hexend` — the two servers'
session code diverged the moment `live-session` started speaking
protobuf JSON.

## Running things for manual/browser testing

```
nix develop --command bash -c 'cd apps/hexend-rs && cargo run'   # :4001
nix develop --command bash -c 'bun run --cwd apps/presentation dev'  # :5173-ish, vite picks a free port
```

Then open:
`http://localhost:<vite-port>/?server=http://localhost:4001&path=<ABSOLUTE path to a .hexen.yml>&gm=1`

`&gm=1` is the GM/operator window — it's the only one with fog-of-war
controls. Drop it for the player-facing/projector window.

## Fog of war (issue #6) — basic PoC, landed 2026-09-17

- `Location.fog: FogOfWar | null` — presence (even empty) means fog is
  on. `FogOfWar.revealedCells: string[]` — opaque cell keys, meaningless
  to the server.
- Commands: `setFog` (start with `{revealedCells:[]}` / clear with
  `null`) and `toggleFogCell` (errors if fog hasn't been started).
- Cell addressing is a **fixed 64px grid over the image**, independent
  of whatever terrain grid (hex/square/none) the Location has — see
  `packages/map-core/src/fog.ts`. This means a location without any
  terrain grid can still have fog.
- The control surface is **the presentation app itself** (`?gm=1`),
  not the editor — clicking the map toggles whichever fog cell was
  clicked. This matches the issue's own wording ("GM can apply and
  selectively remove fog on a map in the Presentation view").
- **Known rough edge:** Leaflet renders markers in `markerPane`, which
  stacks above the fog overlay's pane — link pins stay visible through
  fog for both GM and player views. Not addressed yet; would need
  either a custom pane order or hiding markers under hidden cells
  explicitly in `MapCanvas.tsx`.

## Testing

```
nix develop --command bash -c 'bun run --filter "*" test'   # every TS workspace
nix develop --command bash -c 'cd apps/hexend-rs && cargo test'
nix develop --command bash -c 'bun run typecheck'
```

Known pre-existing (unrelated) typecheck failure:
`scripts/sync-link-icons.ts:81` — `TS2532: Object is possibly
'undefined'`. Not caused by any fog/session work; low priority, hasn't
been triaged.

## Issue tracking

Feature gaps get filed as GitHub issues (`gh issue create`), not just
mentioned in chat — run `gh issue list --state all` before starting a
review so you don't re-flag something already tracked.
