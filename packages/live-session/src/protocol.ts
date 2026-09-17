import type { HexenProject } from "@hex-enductor/hexen-schema";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

/**
 * What hexend's /ws session sends as a "state" message — the same shape
 * both the editor and the presentation app render from, whether it's
 * the initial handshake or a broadcast after a command.
 */
export interface OpenedProjectData {
  project: HexenProject;
  warnings: string[];
  resolvedContent: Record<string, ResolvedContent>;
  resolveErrors: Record<string, string>;
}
