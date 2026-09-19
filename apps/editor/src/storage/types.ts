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
  /** Tears down whatever `open()` set up (a live session's socket, for ServerStorage) — call on unmount/re-open, notably in React 19 StrictMode's dev-only double-invoke of an effect, or `open()`'s own connection leaks. A no-op where there's nothing to tear down (LocalFsStorage). */
  close(): void;
  subscribe(onUpdate: (data: OpenedProjectData) => void): () => void;
  execute(command: Command): void;
  /** Purely ephemeral — never touches project state. A no-op on backends with no live session to broadcast over (local-fs). */
  ping(locationId: string, x: number, y: number): void;
  onPing(onPing: (ping: { locationId: string; x: number; y: number }) => void): () => void;
  /** Just as ephemeral as ping — the GM's own map view, for Follow mode. */
  followView(locationId: string, x: number, y: number, zoom: number): void;
  onFollowView(onFollowView: (view: { locationId: string; x: number; y: number; zoom: number }) => void): () => void;
  undo(): void;
  redo(): void;
  getImageUrl(file: string): Promise<string>;
  uploadImage(locationId: string, file: File): Promise<void>;
  removeImage(locationId: string): void;
}
