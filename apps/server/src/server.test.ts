import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { app } from "./server.ts";

let projectDir: string;

function fakePng(width: number, height: number): Buffer {
  const buf = Buffer.alloc(24);
  buf.write("IHDR", 12, "ascii");
  buf.writeUInt32BE(width, 16);
  buf.writeUInt32BE(height, 20);
  return buf;
}

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "server-test-"));
  await mkdir(join(projectDir, "_assets"), { recursive: true });
  await writeFile(join(projectDir, "_assets", "town.png"), Buffer.from([0x89, 0x50, 0x4e, 0x47]));
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

describe("GET /image", () => {
  test("serves a file inside the project directory", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&file=${encodeURIComponent("_assets/town.png")}`,
    );
    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/png");
  });

  test("400s on a path that escapes the project directory", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&file=${encodeURIComponent("../../../etc/passwd")}`,
    );
    expect(res.status).toBe(400);
  });

  test("400s when dir or file is missing", async () => {
    const res = await app.request(`/image?dir=${encodeURIComponent(projectDir)}`);
    expect(res.status).toBe(400);
  });

  test("404s for a file that doesn't exist", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&file=${encodeURIComponent("nope.png")}`,
    );
    expect(res.status).toBe(404);
  });
});

describe("POST /image", () => {
  test("writes the file into _assets and reports its dimensions", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&locationId=old-mill&ext=png`,
      { method: "POST", body: fakePng(400, 300) },
    );
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ file: "_assets/old-mill.png", width: 400, height: 300 });

    const written = await readFile(join(projectDir, "_assets", "old-mill.png"));
    expect(written.equals(fakePng(400, 300))).toBe(true);
  });

  test("normalizes jpeg to a .jpg file", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&locationId=inn&ext=jpeg`,
      { method: "POST", body: fakePng(1, 1) }, // dimensions don't need to be a real jpeg for this check
    );
    expect(res.status).toBe(200);
    const body = (await res.json()) as { file: string };
    expect(body.file).toBe("_assets/inn.jpg");
  });

  test("re-uploading for the same location overwrites rather than accumulating files", async () => {
    await app.request(`/image?dir=${encodeURIComponent(projectDir)}&locationId=well&ext=png`, {
      method: "POST",
      body: fakePng(10, 10),
    });
    const res = await app.request(`/image?dir=${encodeURIComponent(projectDir)}&locationId=well&ext=png`, {
      method: "POST",
      body: fakePng(20, 20),
    });
    expect(await res.json()).toEqual({ file: "_assets/well.png", width: 20, height: 20 });

    const written = await readFile(join(projectDir, "_assets", "well.png"));
    expect(written.equals(fakePng(20, 20))).toBe(true);
  });

  test("sanitizes a location id that looks like a path", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&locationId=${encodeURIComponent("../../etc/evil")}&ext=png`,
      { method: "POST", body: fakePng(5, 5) },
    );
    expect(res.status).toBe(200);
    const body = (await res.json()) as { file: string };
    expect(body.file).not.toContain("..");
  });

  test("400s on an unsupported extension", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&locationId=town&ext=gif`,
      { method: "POST", body: fakePng(1, 1) },
    );
    expect(res.status).toBe(400);
  });

  test("400s when the posted bytes aren't a readable image", async () => {
    const res = await app.request(
      `/image?dir=${encodeURIComponent(projectDir)}&locationId=town&ext=png`,
      { method: "POST", body: new TextEncoder().encode("not an image") },
    );
    expect(res.status).toBe(400);
  });

  test("400s when dir, locationId, or ext is missing", async () => {
    const res = await app.request(`/image?dir=${encodeURIComponent(projectDir)}`, {
      method: "POST",
      body: fakePng(1, 1),
    });
    expect(res.status).toBe(400);
  });
});

describe("GET /", () => {
  test("responds", async () => {
    const res = await app.request("/");
    expect(res.status).toBe(200);
  });
});
