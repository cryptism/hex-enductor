import type { Command } from "@hex-enductor/project-ops";
import type { OpenedProjectData } from "./protocol.ts";
import { commandToWire, openedProjectDataFromWire } from "./wireFormat.ts";

export interface PingEvent {
  locationId: string;
  x: number;
  y: number;
}

/**
 * A live connection to one project's session on hexend. `initial` is
 * the state at the moment of connecting; every state after that —
 * whether it's the result of this client's own `execute`/`undo`/`redo`
 * or another connected client's — arrives through `subscribe`. There's
 * no other way to learn a project's state: hexend is authoritative, so
 * a client never applies a command locally and just waits to be told
 * what actually happened.
 *
 * `ping`/`onPing` are the one exception to all of that: purely
 * ephemeral, never touches project state, never persisted — hexend
 * just re-broadcasts it verbatim to every socket watching the same
 * session, sender included.
 *
 * A read-only consumer (the presentation app) is exactly this same
 * type, minus ever calling `execute`/`undo`/`redo`/`ping` — there's no
 * separate read-only variant to keep in sync.
 */
export interface LiveSession {
  readonly initial: OpenedProjectData;
  subscribe(onUpdate: (data: OpenedProjectData) => void): () => void;
  onPing(onPing: (ping: PingEvent) => void): () => void;
  execute(command: Command): void;
  ping(locationId: string, x: number, y: number): void;
  undo(): void;
  redo(): void;
  close(): void;
}

// hexend's ServerMessage — protobuf JSON mapping, not a discriminated
// union: `state`/`ping` are just this message's two fields (a proto
// oneof, but that's invisible in JSON — only one is ever present at a
// time). `state`'s own oneof-shaped fields still need
// wireFormat.ts's conversion; `ping`'s fields are all plain.
interface WireServerMessage {
  state?: Parameters<typeof openedProjectDataFromWire>[0];
  ping?: PingEvent;
}

function isWireServerMessage(value: unknown): value is WireServerMessage {
  return typeof value === "object" && value !== null && ("state" in value || "ping" in value);
}

function wsUrl(serverUrl: string, path: string): string {
  return `${serverUrl.replace(/^http/, "ws")}/ws?path=${encodeURIComponent(path)}`;
}

export function connectLiveSession(serverUrl: string, path: string): Promise<LiveSession> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl(serverUrl, path));
    const listeners = new Set<(data: OpenedProjectData) => void>();
    const pingListeners = new Set<(ping: PingEvent) => void>();
    let settled = false;

    socket.addEventListener("error", () => {
      if (!settled) {
        settled = true;
        reject(new Error(`Couldn't connect to ${serverUrl}`));
      }
    });

    socket.addEventListener("close", () => {
      if (!settled) {
        settled = true;
        reject(new Error(`Connection to ${serverUrl} closed before it opened`));
      }
    });

    // The very first message is always a "state" handshake — it
    // resolves this promise instead of going to `subscribe` listeners,
    // since nothing could have registered one before this function
    // returns. A "ping" can never arrive first in practice, but is
    // handled defensively (ignored) if it somehow did.
    socket.addEventListener("message", (evt) => {
      if (typeof evt.data !== "string") return;
      let message: unknown;
      try {
        message = JSON.parse(evt.data);
      } catch {
        return;
      }
      if (!isWireServerMessage(message)) return;

      if (message.ping) {
        for (const listener of pingListeners) listener(message.ping);
        return;
      }
      if (!message.state) return;
      const data = openedProjectDataFromWire(message.state);

      if (!settled) {
        settled = true;
        resolve({
          initial: data,
          subscribe(onUpdate) {
            listeners.add(onUpdate);
            return () => listeners.delete(onUpdate);
          },
          onPing(onPing) {
            pingListeners.add(onPing);
            return () => pingListeners.delete(onPing);
          },
          execute(command) {
            socket.send(JSON.stringify({ command: commandToWire(command) }));
          },
          ping(locationId, x, y) {
            socket.send(JSON.stringify({ ping: { locationId, x, y } }));
          },
          undo() {
            socket.send(JSON.stringify({ undo: {} }));
          },
          redo() {
            socket.send(JSON.stringify({ redo: {} }));
          },
          close() {
            socket.close();
          },
        });
        return;
      }

      for (const listener of listeners) listener(data);
    });
  });
}
