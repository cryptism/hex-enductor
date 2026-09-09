// Bun dev entrypoint (docs/PLAN.md §9: Bun for dev). A Node production
// entrypoint is just `serve(app)` from `@hono/node-server` wrapping the
// same `app` export — not written yet, not needed to prove this out.
import { app } from "./server.ts";

const port = Number(process.env.PORT ?? 4000);

console.log(`hex-enductor server listening on http://localhost:${port}`);

export default {
  port,
  fetch: app.fetch,
};
