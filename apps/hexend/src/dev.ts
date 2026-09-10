// Bun dev entrypoint. A Node production entrypoint is just
// `serve(app)` from `@hono/node-server` wrapping the same `app` export
// — not written yet, not needed to prove this out.
import { app } from "./server.ts";

const port = Number(process.env.PORT ?? 4000);

const BANNER = `
  ██║  ██║███████╗██╗  ██╗███████╗███╗   ██╗██████╗
  ██╠══██║██╠════╝╚██╗██╔╝██╠════╝████╗  ██║██╠══██╗
  ███████║█████╗   ╚███╔╝ █████╗  ██╔██╗ ██║██║  ██║
  ██╠══██║██╠══╝   ██╔██╗ ██╠══╝  ██║╚██╗██║██║  ██║
  ██║  ██║███████╗██╔╝ ██╗███████╗██║ ╚████║██████╔╝
  ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝
`;

console.log(BANNER);
console.log(`  it has opened an eye at http://localhost:${port}, and it does not blink.`);
console.log(`  feed it a .hexen.yml, or feed it nothing — it will wait either way.\n`);

export default {
  port,
  fetch: app.fetch,
};
