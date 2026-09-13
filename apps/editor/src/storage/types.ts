import type { Command } from "@hex-enductor/project-ops";
import type { OpenedProjectData } from "@hex-enductor/live-session";

export type { OpenedProjectData };

/**
 * Everywhere in the editor that touches project data goes through one
 * of these instead of talking to a transport directly — ServerStorage
 * wraps hexend's /ws session (via @hex-enductor/live-session),
 * LocalFsStorage wraps the File System Access API. `execute` never
 * returns the resulting state directly: every state change, this
 * backend's own or (for the server backend) another connected
 * client's, arrives through `subscribe` instead — that's what lets a
 * presentation window ride the exact same mechanism read-only.
 */
export interface ProjectStorage {
  /** Shown in the UI — the server path, or the chosen folder's name. */
  readonly label: string;
  open(): Promise<OpenedProjectData>;
  subscribe(onUpdate: (data: OpenedProjectData) => void): () => void;
  execute(command: Command): void;
  undo(): void;
  redo(): void;
  getImageUrl(file: string): Promise<string>;
  uploadImage(locationId: string, file: File): Promise<void>;
  removeImage(locationId: string): void;
}
