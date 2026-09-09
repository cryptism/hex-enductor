import { readFile } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { trpcServer } from "@hono/trpc-server";
import { appRouter } from "./router.ts";

const MIME_TYPES: Record<string, string> = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
};

export const app = new Hono();

app.use("*", cors());

app.use(
  "/trpc/*",
  trpcServer({
    router: appRouter,
  }),
);

// No auth for v1 (local/LAN trust), but a file server still shouldn't
// let a caller walk out of the project directory it was handed — see
// server.test.ts for the traversal case this guards against.
app.get("/image", async (c) => {
  const projectDir = c.req.query("dir");
  const file = c.req.query("file");
  if (!projectDir || !file) {
    return c.text("Missing dir or file query param", 400);
  }

  const base = resolve(projectDir);
  const target = resolve(base, file);
  if (target !== base && !target.startsWith(base + sep)) {
    return c.text("file escapes the project directory", 400);
  }

  try {
    const bytes = await readFile(target);
    const contentType = MIME_TYPES[extname(target).toLowerCase()] ?? "application/octet-stream";
    return new Response(new Uint8Array(bytes), { headers: { "Content-Type": contentType } });
  } catch {
    return c.text("Not found", 404);
  }
});

app.get("/", (c) => c.text("hex-enductor server"));
