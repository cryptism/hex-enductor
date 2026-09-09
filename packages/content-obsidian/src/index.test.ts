import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createObsidianResolver } from "./index.ts";

let projectDir: string;

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "content-obsidian-test-"));
  await mkdir(join(projectDir, "_vault"), { recursive: true });
  await writeFile(
    join(projectDir, "_vault", "with-title.md"),
    "---\ntitle: The Rusty Tankard\nsummary: A cozy inn.\n---\nWarm firelight inside.\n",
  );
  await writeFile(join(projectDir, "_vault", "No Frontmatter.md"), "Just a body, no frontmatter.\n");
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

describe("createObsidianResolver", () => {
  const resolver = () => createObsidianResolver({ projectDir, vaultRoot: "_vault" });

  test("reads title, summary, and body from frontmatter", async () => {
    const result = await resolver().resolve({ type: "obsidian", ref: "with-title.md" });
    expect(result.title).toBe("The Rusty Tankard");
    expect(result.summary).toBe("A cozy inn.");
    expect(result.body).toBe("Warm firelight inside.");
  });

  test("falls back to the filename when there's no title in frontmatter", async () => {
    const result = await resolver().resolve({ type: "obsidian", ref: "No Frontmatter.md" });
    expect(result.title).toBe("No Frontmatter");
    expect(result.summary).toBeUndefined();
  });

  test("rejects a content block that isn't its own type", async () => {
    // @ts-expect-error — deliberately the wrong discriminant, to exercise the runtime guard
    await expect(resolver().resolve({ type: "notion", ref: "x" })).rejects.toThrow();
  });
});
