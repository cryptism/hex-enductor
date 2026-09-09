import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { app } from "./server.ts";

let projectDir: string;

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

describe("GET /", () => {
  test("responds", async () => {
    const res = await app.request("/");
    expect(res.status).toBe(200);
  });
});
