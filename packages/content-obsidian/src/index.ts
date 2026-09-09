import { readFile } from "node:fs/promises";
import { basename, extname, join } from "node:path";
import matter from "gray-matter";
import type { LocationContent } from "@hex-enductor/hexen-schema";
import type { ContentResolver, ResolvedContent } from "@hex-enductor/content-resolver";

export interface ObsidianResolverConfig {
  /** Directory the .hexen.yml file lives in — vaultRoot resolves against this. */
  projectDir: string;
  /** content.vaultRoot from the project's top-level content block. */
  vaultRoot: string;
}

/**
 * Reads title/summary/body straight out of the vault's own YAML
 * frontmatter + Markdown body — independently reimplemented rather
 * than sharing Quartz's frontmatter parser, since taking a runtime
 * dependency on the Quartz toolchain here would be the wrong direction
 * of coupling. See index.test.ts for the fallback-title and
 * wrong-content-type behavior.
 */
export function createObsidianResolver(config: ObsidianResolverConfig): ContentResolver {
  const vaultDir = join(config.projectDir, config.vaultRoot);

  return {
    async resolve(content: LocationContent): Promise<ResolvedContent> {
      if (content.type !== "obsidian") {
        throw new Error(`content-obsidian can't resolve content of type "${content.type}"`);
      }

      const filePath = join(vaultDir, content.ref);
      const raw = await readFile(filePath, "utf-8");
      const { data, content: body } = matter(raw);

      const fallbackTitle = basename(content.ref, extname(content.ref));

      return {
        title: typeof data.title === "string" && data.title.length > 0 ? data.title : fallbackTitle,
        summary: typeof data.summary === "string" ? data.summary : undefined,
        body: body.trim(),
      };
    },
  };
}
