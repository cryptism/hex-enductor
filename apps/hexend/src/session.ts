import { z } from "zod";
import type { HexenProject } from "@hex-enductor/hexen-schema";
import { applyCommand, CommandSchema } from "@hex-enductor/project-ops";
import { openProject, saveProject, type OpenedProject } from "./projectIO.ts";

export interface SessionSocket {
  send(data: string): void;
}

interface LogEntry {
  command: z.infer<typeof CommandSchema>;
  snapshot: HexenProject;
}

export interface ProjectSession {
  readonly path: string;
  project: HexenProject;
  resolvedContent: OpenedProject["resolvedContent"];
  resolveErrors: OpenedProject["resolveErrors"];
  readonly warnings: string[];
  readonly sockets: Set<SessionSocket>;
  /** Command history with a cursor into it — undo/redo just move the cursor. */
  log: LogEntry[];
  cursor: number;
  /** The project as it stood before this session's first command — where undo bottoms out. */
  readonly baseSnapshot: HexenProject;
}

export const ClientMessageSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("command"), command: CommandSchema }),
  z.object({ type: z.literal("undo") }),
  z.object({ type: z.literal("redo") }),
]);
export type ClientMessage = z.infer<typeof ClientMessageSchema>;

// Sessions live for the server process's lifetime, keyed by absolute
// project path — no eviction. Fine at single-user/dev-server scale;
// a real follow-up if this stops being a local tool.
const sessions = new Map<string, ProjectSession>();

export async function getOrCreateSession(path: string): Promise<ProjectSession> {
  const existing = sessions.get(path);
  if (existing) return existing;

  const opened = await openProject(path);
  const session: ProjectSession = {
    path,
    project: opened.project,
    resolvedContent: opened.resolvedContent,
    resolveErrors: opened.resolveErrors,
    warnings: opened.warnings,
    sockets: new Set(),
    log: [],
    cursor: -1,
    baseSnapshot: structuredClone(opened.project),
  };
  sessions.set(path, session);
  return session;
}

export interface SessionState {
  project: HexenProject;
  warnings: string[];
  resolvedContent: OpenedProject["resolvedContent"];
  resolveErrors: OpenedProject["resolveErrors"];
}

export function sessionState(session: ProjectSession): SessionState {
  return {
    project: session.project,
    warnings: session.warnings,
    resolvedContent: session.resolvedContent,
    resolveErrors: session.resolveErrors,
  };
}

// Inline content lives directly on the Location, so it never needs a
// vault file re-read — only these five commands' locations can ever
// change, and only inline ones. Obsidian-resolved entries are stable
// across every command this session handles, so they're never touched.
function refreshInlineContent(session: ProjectSession): void {
  for (const location of session.project.locations) {
    if (location.content?.type === "inline") {
      session.resolvedContent[location.id] = { title: location.content.title, body: location.content.body };
      delete session.resolveErrors[location.id];
    }
  }
}

function persist(session: ProjectSession): void {
  void saveProject(session.path, session.project).catch((err: unknown) => {
    console.error(`Failed to persist "${session.path}":`, err);
  });
}

function broadcast(session: ProjectSession): void {
  const message = JSON.stringify({ type: "state", data: sessionState(session) });
  for (const socket of session.sockets) socket.send(message);
}

function settle(session: ProjectSession, project: HexenProject): void {
  session.project = project;
  refreshInlineContent(session);
  broadcast(session);
  persist(session);
}

export function applyAndBroadcast(session: ProjectSession, command: z.infer<typeof CommandSchema>): void {
  // A command issued after an undo discards whatever redo branch existed.
  if (session.cursor < session.log.length - 1) {
    session.log = session.log.slice(0, session.cursor + 1);
  }
  const next = applyCommand(structuredClone(session.project), command);
  session.log.push({ command, snapshot: structuredClone(next) });
  session.cursor++;
  settle(session, next);
}

export function undo(session: ProjectSession): void {
  if (session.cursor < 0) return;
  session.cursor--;
  const snapshot = session.cursor === -1 ? session.baseSnapshot : session.log[session.cursor]!.snapshot;
  settle(session, structuredClone(snapshot));
}

export function redo(session: ProjectSession): void {
  if (session.cursor >= session.log.length - 1) return;
  session.cursor++;
  settle(session, structuredClone(session.log[session.cursor]!.snapshot));
}

/** Test-only: drop every cached session so each test starts from a clean file read. */
export function _resetSessionsForTest(): void {
  sessions.clear();
}
