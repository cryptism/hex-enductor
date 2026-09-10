import { parse as parseYaml } from "yaml";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

const FRONTMATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/;

/**
 * Splits a leading `---`-delimited YAML frontmatter block off a note's
 * body. Not gray-matter: gray-matter's dependency chain reaches for
 * Node's Buffer at module scope, which breaks when this file is
 * bundled for the browser (see ../parseNote note below). Obsidian
 * notes only ever use this one frontmatter style, so a full
 * multi-format parser is more than this needs anyway.
 */
function splitFrontmatter(raw: string): { data: Record<string, unknown>; body: string } {
  const match = FRONTMATTER.exec(raw);
  if (!match) return { data: {}, body: raw };

  const [, frontmatterText, body] = match;
  let data: unknown;
  try {
    data = parseYaml(frontmatterText!);
  } catch {
    return { data: {}, body: raw };
  }
  return { data: data && typeof data === "object" ? (data as Record<string, unknown>) : {}, body: body! };
}

/**
 * Parses a vault note's raw text into title/summary/body — pure, no
 * file I/O, so it's the one part of Obsidian content resolution that's
 * shared between the Node-only resolver in ./index.ts and the
 * browser-native (File System Access API) storage in apps/editor,
 * which reads bytes a different way and must not import node:fs or
 * node:path at all. Import this module directly rather than through
 * ./index.ts's barrel if you're bundling for the browser.
 */
export function parseObsidianNote(raw: string, ref: string): ResolvedContent {
  const { data, body } = splitFrontmatter(raw);
  const fallbackTitle = ref.split("/").pop()!.replace(/\.[^.]+$/, "");

  return {
    title: typeof data.title === "string" && data.title.length > 0 ? data.title : fallbackTitle,
    summary: typeof data.summary === "string" ? data.summary : undefined,
    body: body.trim(),
  };
}
