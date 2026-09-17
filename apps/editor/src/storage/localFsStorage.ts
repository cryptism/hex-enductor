import { parseHexenProject, serializeHexenProject, type HexenProject } from "@hex-enductor/hexen-schema";
import { applyCommand, readImageSize, type Command } from "@hex-enductor/project-ops";
// Deep import, not the package's index.ts barrel — that one also pulls
// in node:fs/node:path for the server-side resolver, which Vite can't
// bundle for the browser. This file is pure (gray-matter only).
import { parseObsidianNote } from "@hex-enductor/content-obsidian/src/parseNote.ts";
import type { OpenedProjectData, ProjectStorage } from "./types.ts";

async function findHexenFileHandle(dir: FileSystemDirectoryHandle): Promise<FileSystemFileHandle> {
  for await (const entry of dir.values()) {
    if (entry.kind === "file" && entry.name.endsWith(".hexen.yml")) {
      return entry as FileSystemFileHandle;
    }
  }
  throw new Error("No .hexen.yml file found in this folder.");
}

async function traverseToFile(
  root: FileSystemDirectoryHandle,
  filePath: string,
  create: boolean,
): Promise<FileSystemFileHandle> {
  const parts = filePath.split("/").filter(Boolean);
  let dir = root;
  for (let i = 0; i < parts.length - 1; i++) {
    dir = await dir.getDirectoryHandle(parts[i]!, { create });
  }
  return dir.getFileHandle(parts[parts.length - 1]!, { create });
}

function extensionFor(file: File): string | null {
  if (file.type === "image/png") return "png";
  if (file.type === "image/jpeg") return "jpeg";
  const m = /\.(png|jpe?g)$/i.exec(file.name);
  return m ? m[1]!.toLowerCase() : null;
}

// Mirrors node:path.join's collapsing of "." and empty segments, for
// the one join (vaultRoot + a location's ref) this storage needs —
// no node:path here, this file has to stay bundlable for the browser.
function joinPath(...parts: string[]): string {
  return parts
    .join("/")
    .split("/")
    .filter((part) => part && part !== ".")
    .join("/");
}

async function resolveContent(
  dirHandle: FileSystemDirectoryHandle,
  project: HexenProject,
): Promise<Pick<OpenedProjectData, "resolvedContent" | "resolveErrors">> {
  const resolvedContent: OpenedProjectData["resolvedContent"] = {};
  const resolveErrors: OpenedProjectData["resolveErrors"] = {};
  const vaultRoot = project.content.type === "obsidian" ? project.content.vaultRoot : null;

  await Promise.all(
    project.locations
      .filter((location) => location.content !== null)
      .map(async (location) => {
        const content = location.content!;
        try {
          if (content.type === "inline") {
            resolvedContent[location.id] = { title: content.title, body: content.body };
          } else if (vaultRoot !== null) {
            const handle = await traverseToFile(dirHandle, joinPath(vaultRoot, content.ref), false);
            const raw = await (await handle.getFile()).text();
            resolvedContent[location.id] = parseObsidianNote(raw, content.ref);
          } else {
            throw new Error(`Location "${location.id}" has obsidian content, but this project has no vault configured`);
          }
        } catch (err) {
          resolveErrors[location.id] = err instanceof Error ? err.message : String(err);
        }
      }),
  );

  return { resolvedContent, resolveErrors };
}

/** The File System Access API back end — a project opened as a folder in the browser, no server involved at all. */
export function createLocalFsStorage(dirHandle: FileSystemDirectoryHandle): ProjectStorage {
  let fileHandle: FileSystemFileHandle | null = null;
  const listeners = new Set<(data: OpenedProjectData) => void>();

  // There's no server here to be authoritative over, so undo/redo is
  // just a local command log + cursor, same shape as hexend's session
  // but scoped to this one tab.
  let log: { command: Command; snapshot: HexenProject }[] = [];
  let cursor = -1;
  let baseSnapshot: HexenProject | null = null;

  async function getFileHandle(): Promise<FileSystemFileHandle> {
    fileHandle ??= await findHexenFileHandle(dirHandle);
    return fileHandle;
  }

  async function readProject(): Promise<HexenProject> {
    const handle = await getFileHandle();
    const text = await (await handle.getFile()).text();
    return parseHexenProject(text).project;
  }

  async function writeProject(project: HexenProject): Promise<void> {
    const handle = await getFileHandle();
    const writable = await handle.createWritable();
    await writable.write(serializeHexenProject(project));
    await writable.close();
  }

  async function open(): Promise<OpenedProjectData> {
    const handle = await getFileHandle();
    const { project, warnings } = parseHexenProject(await (await handle.getFile()).text());
    return { project, warnings, ...(await resolveContent(dirHandle, project)) };
  }

  async function notify(): Promise<void> {
    const data = await open();
    for (const listener of listeners) listener(data);
  }

  async function settle(project: HexenProject): Promise<void> {
    await writeProject(project);
    await notify();
  }

  async function execute(command: Command): Promise<void> {
    baseSnapshot ??= structuredClone(await readProject());
    if (cursor < log.length - 1) log = log.slice(0, cursor + 1);
    const current = cursor === -1 ? baseSnapshot : log[cursor]!.snapshot;
    const next = applyCommand(structuredClone(current), command);
    log.push({ command, snapshot: structuredClone(next) });
    cursor++;
    await settle(next);
  }

  return {
    label: dirHandle.name,
    open,

    subscribe(onUpdate) {
      listeners.add(onUpdate);
      return () => listeners.delete(onUpdate);
    },

    // No live session to broadcast over — a no-op, not an error, so
    // callers (the editor's Ping/Follow mode tools) don't need to know
    // or care which backend is active.
    ping() {},
    onPing() {
      return () => {};
    },
    followView() {},
    onFollowView() {
      return () => {};
    },

    execute,

    async undo() {
      if (cursor < 0 || !baseSnapshot) return;
      cursor--;
      const snapshot = cursor === -1 ? baseSnapshot : log[cursor]!.snapshot;
      await settle(structuredClone(snapshot));
    },

    async redo() {
      if (cursor >= log.length - 1) return;
      cursor++;
      await settle(structuredClone(log[cursor]!.snapshot));
    },

    async getImageUrl(file) {
      const handle = await traverseToFile(dirHandle, file, false);
      return URL.createObjectURL(await handle.getFile());
    },

    async uploadImage(locationId, file) {
      const ext = extensionFor(file);
      if (!ext) throw new Error("Only PNG or JPEG images are supported.");
      const bytes = new Uint8Array(await file.arrayBuffer());
      const size = readImageSize(bytes);
      if (!size) throw new Error("Couldn't read image dimensions — is this really a PNG or JPEG?");

      const safeExt = ext === "jpeg" ? "jpg" : ext;
      const safeName = `${locationId.replace(/[^a-zA-Z0-9_-]/g, "-")}.${safeExt}`;
      const assetsDir = await dirHandle.getDirectoryHandle("_assets", { create: true });
      const imageHandle = await assetsDir.getFileHandle(safeName, { create: true });
      const writable = await imageHandle.createWritable();
      await writable.write(bytes);
      await writable.close();

      await execute({
        type: "saveImage",
        locationId,
        image: { file: `_assets/${safeName}`, width: size.width, height: size.height },
      });
    },

    removeImage(locationId) {
      void execute({ type: "saveImage", locationId, image: null });
    },
  };
}
