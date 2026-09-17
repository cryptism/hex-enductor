import { connectLiveSession, type LiveSession } from "@hex-enductor/live-session";
import { serverUrl } from "../server.ts";
import type { OpenedProjectData, ProjectStorage } from "./types.ts";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

function extensionFor(file: File): string | null {
  if (file.type === "image/png") return "png";
  if (file.type === "image/jpeg") return "jpeg";
  const m = /\.(png|jpe?g)$/i.exec(file.name);
  return m ? m[1]!.toLowerCase() : null;
}

/** Wraps hexend's live /ws session — the storage backend the app has had all along, just server-authoritative now. */
export function createServerStorage(path: string): ProjectStorage {
  let session: LiveSession | null = null;
  // Registered before open() necessarily resolves — App.tsx's own
  // "open" and "subscribe" effects both fire in the same commit, with
  // no guarantee of which runs first, so subscribe can't require a
  // session to already exist (mirrors localFsStorage, which never
  // gates on anything either).
  const listeners = new Set<(data: OpenedProjectData) => void>();

  return {
    label: path,

    async open() {
      session = await connectLiveSession(serverUrl(), path);
      session.subscribe((data) => {
        for (const listener of listeners) listener(data);
      });
      return session.initial;
    },

    subscribe(onUpdate) {
      listeners.add(onUpdate);
      return () => listeners.delete(onUpdate);
    },

    execute(command) {
      session?.execute(command);
    },

    undo() {
      session?.undo();
    },

    redo() {
      session?.redo();
    },

    async getImageUrl(file) {
      const dir = dirname(path);
      return `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&file=${encodeURIComponent(file)}`;
    },

    async uploadImage(locationId, file) {
      const ext = extensionFor(file);
      if (!ext) throw new Error("Only PNG or JPEG images are supported.");

      const dir = dirname(path);
      const res = await fetch(
        `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&locationId=${encodeURIComponent(locationId)}&ext=${ext}`,
        { method: "POST", body: file },
      );
      if (!res.ok) throw new Error(await res.text());
      const image = (await res.json()) as { file: string; width: number; height: number };

      session?.execute({ type: "saveImage", locationId, image });
    },

    removeImage(locationId) {
      session?.execute({ type: "saveImage", locationId, image: null });
    },
  };
}
