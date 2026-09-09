import { describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { generateJsonSchema } from "./generate-json-schema.ts";

describe("generateJsonSchema", () => {
  test("docs/hexen.schema.json is up to date with the schema source", async () => {
    const checkedIn = await readFile(
      join(import.meta.dir, "..", "..", "..", "docs", "hexen.schema.json"),
      "utf-8",
    );
    const fresh = JSON.stringify(generateJsonSchema(), null, 2) + "\n";
    expect(checkedIn).toBe(fresh);
  });
});
