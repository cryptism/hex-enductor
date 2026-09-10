import matter from "gray-matter";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

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
  const { data, content: body } = matter(raw);
  const fallbackTitle = ref.split("/").pop()!.replace(/\.[^.]+$/, "");

  return {
    title: typeof data.title === "string" && data.title.length > 0 ? data.title : fallbackTitle,
    summary: typeof data.summary === "string" ? data.summary : undefined,
    body: body.trim(),
  };
}
