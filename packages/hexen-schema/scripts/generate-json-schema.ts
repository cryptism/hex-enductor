import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod";
import { HexenProjectSchema } from "../src/project.ts";

/**
 * Compiles HexenProjectSchema to a standalone JSON Schema document —
 * the .describe() calls throughout hexen-schema's src/ become each
 * field's `description`. Written to docs/hexen.schema.json at the
 * repo root, and referenceable directly from a .hexen.yml file for
 * editor validation/autocomplete:
 *
 *   # yaml-language-server: $schema=../../docs/hexen.schema.json
 */
export function generateJsonSchema(): object {
  return z.toJSONSchema(HexenProjectSchema, { target: "draft-7" });
}

if (import.meta.main) {
  const schema = generateJsonSchema();
  const outPath = join(import.meta.dir, "..", "..", "..", "docs", "hexen.schema.json");
  await writeFile(outPath, JSON.stringify(schema, null, 2) + "\n", "utf-8");
  console.log(`Wrote ${outPath}`);
}
