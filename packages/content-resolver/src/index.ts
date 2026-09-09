import type { LocationContent } from "@hex-enductor/hexen-schema";

export interface ResolvedContent {
  title: string;
  summary?: string;
  body: string;
}

/**
 * A Location's title/summary/body always come from its `content`
 * block, resolved at load time, never duplicated into .hexen.yml.
 * Obsidian (@hex-enductor/content-obsidian) is the only implementation
 * today — a second content backend later is a second package
 * implementing this interface, not a fork of the format or a change to
 * hexen-schema.
 */
export interface ContentResolver {
  resolve(content: LocationContent): Promise<ResolvedContent>;
}
