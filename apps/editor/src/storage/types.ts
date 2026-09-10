import type { Grid, HexenProject, ImageRef, Link } from "@hex-enductor/hexen-schema";
import type { ResolvedContent } from "@hex-enductor/content-resolver";

export interface OpenedProjectData {
  project: HexenProject;
  warnings: string[];
  resolvedContent: Record<string, ResolvedContent>;
  resolveErrors: Record<string, string>;
}

/**
 * Everywhere in the editor that touches project data goes through one
 * of these instead of talking to a transport directly — ServerStorage
 * wraps the existing tRPC/server calls, LocalFsStorage wraps the
 * File System Access API. Every mutating method returns the freshly
 * reopened data, same as the old trpc-mutate-then-invalidate dance,
 * so a caller never has to know which backend it's driving.
 */
export interface ProjectStorage {
  /** Shown in the UI — the server path, or the chosen folder's name. */
  readonly label: string;
  open(): Promise<OpenedProjectData>;
  refresh(): Promise<OpenedProjectData>;
  getImageUrl(file: string): Promise<string>;
  saveLink(locationId: string, linkId: string, patch: Partial<Link>): Promise<OpenedProjectData>;
  saveLocationContent(
    locationId: string,
    patch: { title?: string; body?: string },
  ): Promise<OpenedProjectData>;
  addLocationLink(
    parentLocationId: string,
    targetLocationId: string,
    x: number,
    y: number,
    type: string,
  ): Promise<OpenedProjectData>;
  saveGrid(locationId: string, grid: Grid | null): Promise<OpenedProjectData>;
  uploadImage(locationId: string, file: File): Promise<OpenedProjectData>;
  removeImage(locationId: string): Promise<OpenedProjectData>;
}
