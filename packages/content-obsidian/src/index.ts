import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type { LocationContent } from "@hex-enductor/hexen-schema";
import type { ContentResolver, ResolvedContent } from "@hex-enductor/content-resolver";
import { parseObsidianNote } from "./parseNote.ts";

export { parseObsidianNote } from "./parseNote.ts";

export interface ObsidianResolverConfig {
  /** Directory the .hexen.yml file lives in — vaultRoot resolves against this. */
  projectDir: string;
  /** content.vaultRoot from the project's top-level content block. */
  vaultRoot: string;
}

/** See index.test.ts for the fallback-title and wrong-content-type behavior. */
export function createObsidianResolver(config: ObsidianResolverConfig): ContentResolver {
  const vaultDir = join(config.projectDir, config.vaultRoot);

  return {
    async resolve(content: LocationContent): Promise<ResolvedContent> {
      if (content.type !== "obsidian") {
        throw new Error(`content-obsidian can't resolve content of type "${content.type}"`);
      }

      const filePath = join(vaultDir, content.ref);
      const raw = await readFile(filePath, "utf-8");
      return parseObsidianNote(raw, content.ref);
    },
  };
}
