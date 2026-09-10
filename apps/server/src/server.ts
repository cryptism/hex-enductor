import { mkdir, readFile, writeFile } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { trpcServer } from "@hono/trpc-server";
import { appRouter } from "./router.ts";
import { readImageSize } from "@hex-enductor/project-ops";

const UPLOAD_EXTENSIONS: Record<string, string> = { png: "png", jpg: "jpg", jpeg: "jpg" };

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

// A location's base map image: the client posts raw bytes for a given
// project dir + location id, we settle on a filename ourselves (so
// re-uploading for the same location always replaces the same file
// rather than accumulating orphans) and hand back what saveGrid's
// sibling mutation, saveImage, needs to write into the .hexen.yml.
app.post("/image", async (c) => {
  const projectDir = c.req.query("dir");
  const locationId = c.req.query("locationId");
  const extParam = c.req.query("ext")?.toLowerCase();
  if (!projectDir || !locationId || !extParam) {
    return c.text("Missing dir, locationId, or ext query param", 400);
  }
  const ext = UPLOAD_EXTENSIONS[extParam];
  if (!ext) {
    return c.text(`Unsupported image extension "${extParam}" — use png or jpg/jpeg`, 400);
  }

  const safeName = `${locationId.replace(/[^a-zA-Z0-9_-]/g, "-")}.${ext}`;
  const assetsDir = resolve(projectDir, "_assets");
  const target = resolve(assetsDir, safeName);
  if (!target.startsWith(assetsDir + sep)) {
    return c.text("file escapes the project directory", 400);
  }

  const bytes = new Uint8Array(await c.req.arrayBuffer());
  const size = readImageSize(bytes);
  if (!size) {
    return c.text("Couldn't read image dimensions — is this really a PNG or JPEG?", 400);
  }

  await mkdir(assetsDir, { recursive: true });
  await writeFile(target, bytes);

  return c.json({ file: `_assets/${safeName}`, width: size.width, height: size.height });
});

app.get("/", (c) => c.text("hex-enductor server"));
