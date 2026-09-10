import { parseHexenProject, serializeHexenProject, type HexenProject } from "@hex-enductor/hexen-schema";
import {
  saveLink as applySaveLink,
  saveLocationContent as applySaveLocationContent,
  addLocationLink as applyAddLocationLink,
  saveGrid as applySaveGrid,
  saveImage as applySaveImage,
  readImageSize,
} from "@hex-enductor/project-ops";
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

// No vault reader in the browser yet (see issue #18) — an obsidian
// Location resolves fine on the server, but here it can only report
// why it can't: local mode has no filesystem access outside the one
// folder the user granted.
function resolveContent(project: HexenProject): Pick<OpenedProjectData, "resolvedContent" | "resolveErrors"> {
  const resolvedContent: OpenedProjectData["resolvedContent"] = {};
  const resolveErrors: OpenedProjectData["resolveErrors"] = {};
  for (const location of project.locations) {
    if (!location.content) continue;
    if (location.content.type === "inline") {
      resolvedContent[location.id] = { title: location.content.title, body: location.content.body };
    } else {
      resolveErrors[location.id] =
        `Location "${location.id}" has obsidian content, but a browser-opened project can't read a vault yet.`;
    }
  }
  return { resolvedContent, resolveErrors };
}

/** The File System Access API back end — a project opened as a folder in the browser, no server involved at all. */
export function createLocalFsStorage(dirHandle: FileSystemDirectoryHandle): ProjectStorage {
  let fileHandle: FileSystemFileHandle | null = null;

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
    return { project, warnings, ...resolveContent(project) };
  }

  async function commit(mutate: (project: HexenProject) => HexenProject): Promise<OpenedProjectData> {
    const project = mutate(await readProject());
    await writeProject(project);
    return open();
  }

  return {
    label: dirHandle.name,
    open,
    refresh: open,

    async getImageUrl(file) {
      const handle = await traverseToFile(dirHandle, file, false);
      return URL.createObjectURL(await handle.getFile());
    },

    saveLink(locationId, linkId, patch) {
      return commit((project) => applySaveLink(project, locationId, linkId, patch));
    },

    saveLocationContent(locationId, patch) {
      return commit((project) => applySaveLocationContent(project, locationId, patch));
    },

    addLocationLink(parentLocationId, targetLocationId, x, y, type) {
      return commit((project) => applyAddLocationLink(project, parentLocationId, targetLocationId, x, y, type));
    },

    saveGrid(locationId, grid) {
      return commit((project) => applySaveGrid(project, locationId, grid));
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

      return commit((project) =>
        applySaveImage(project, locationId, { file: `_assets/${safeName}`, width: size.width, height: size.height }),
      );
    },

    removeImage(locationId) {
      return commit((project) => applySaveImage(project, locationId, null));
    },
  };
}
