import { describe, expect, test } from "bun:test";
import { createLocalFsStorage } from "./localFsStorage.ts";

// A minimal in-memory stand-in for the File System Access API — just
// enough of FileSystemDirectoryHandle/FileSystemFileHandle for
// LocalFsStorage to drive, so this suite runs without a real browser.
class FakeFileHandle {
  readonly kind = "file" as const;
  content = new Uint8Array();
  constructor(public name: string) {}

  async getFile(): Promise<File> {
    return new File([this.content], this.name);
  }

  async createWritable() {
    let pending = new Uint8Array();
    const self = this;
    return {
      async write(data: string | Uint8Array | ArrayBuffer) {
        pending = typeof data === "string" ? new TextEncoder().encode(data) : new Uint8Array(data);
      },
      async close() {
        self.content = pending;
      },
    };
  }
}

class FakeDirectoryHandle {
  readonly kind = "directory" as const;
  files = new Map<string, FakeFileHandle>();
  dirs = new Map<string, FakeDirectoryHandle>();
  constructor(public name: string) {}

  async *values() {
    yield* this.files.values();
    yield* this.dirs.values();
  }

  async getFileHandle(name: string, opts?: { create?: boolean }): Promise<FakeFileHandle> {
    let handle = this.files.get(name);
    if (!handle) {
      if (!opts?.create) throw new Error(`NotFoundError: ${name}`);
      handle = new FakeFileHandle(name);
      this.files.set(name, handle);
    }
    return handle;
  }

  async getDirectoryHandle(name: string, opts?: { create?: boolean }): Promise<FakeDirectoryHandle> {
    let handle = this.dirs.get(name);
    if (!handle) {
      if (!opts?.create) throw new Error(`NotFoundError: ${name}`);
      handle = new FakeDirectoryHandle(name);
      this.dirs.set(name, handle);
    }
    return handle;
  }
}

function fakeRoot(yamlText: string): FakeDirectoryHandle {
  const root = new FakeDirectoryHandle("my-realm");
  const file = new FakeFileHandle("project.hexen.yml");
  file.content = new TextEncoder().encode(yamlText);
  root.files.set("project.hexen.yml", file);
  return root;
}

function addFile(root: FakeDirectoryHandle, path: string, content: string): void {
  const parts = path.split("/");
  let dir = root;
  for (let i = 0; i < parts.length - 1; i++) {
    let next = dir.dirs.get(parts[i]!);
    if (!next) {
      next = new FakeDirectoryHandle(parts[i]!);
      dir.dirs.set(parts[i]!, next);
    }
    dir = next;
  }
  const file = new FakeFileHandle(parts[parts.length - 1]!);
  file.content = new TextEncoder().encode(content);
  dir.files.set(parts[parts.length - 1]!, file);
}

const YAML = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
    links:
      - id: front-door
        target: inn
        x: 1
        y: 1
        type: settlement
        color: null
        hidden: false
  - id: inn
    content: { type: inline, title: The Inn, body: "" }
`;

const YAML_WITH_OBSIDIAN = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    content: { type: obsidian, ref: town.md }
`;

describe("createLocalFsStorage", () => {
  test("open() parses the .hexen.yml found in the folder", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const data = await storage.open();
    expect(data.project.title).toBe("Test Realm");
    expect(data.resolvedContent.town).toEqual({ title: "The Town", body: "" });
    expect(data.warnings).toEqual([]);
  });

  test("resolves obsidian content by reading the vault file through the directory handle", async () => {
    const root = fakeRoot(YAML_WITH_OBSIDIAN);
    addFile(root, "_vault/town.md", "---\ntitle: The Town\n---\nA quiet place.");
    const storage = createLocalFsStorage(root as unknown as FileSystemDirectoryHandle);
    const data = await storage.open();
    expect(data.resolvedContent.town).toEqual({ title: "The Town", body: "A quiet place." });
    expect(data.resolveErrors.town).toBeUndefined();
  });

  test("falls back to the filename when a vault note has no title in frontmatter", async () => {
    const root = fakeRoot(YAML_WITH_OBSIDIAN);
    addFile(root, "_vault/town.md", "No frontmatter here.");
    const storage = createLocalFsStorage(root as unknown as FileSystemDirectoryHandle);
    const data = await storage.open();
    expect(data.resolvedContent.town).toEqual({ title: "town", body: "No frontmatter here." });
  });

  test("reports a resolveError when the vault file is missing", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML_WITH_OBSIDIAN) as unknown as FileSystemDirectoryHandle);
    const data = await storage.open();
    expect(data.resolvedContent.town).toBeUndefined();
    expect(data.resolveErrors.town).toMatch(/NotFoundError/);
  });

  test("reports a resolveError for obsidian Location content when the project has no vault configured", async () => {
    const yaml = YAML_WITH_OBSIDIAN.replace("content: { type: obsidian, vaultRoot: _vault }", "content: { type: inline }");
    const storage = createLocalFsStorage(fakeRoot(yaml) as unknown as FileSystemDirectoryHandle);
    const data = await storage.open();
    expect(data.resolvedContent.town).toBeUndefined();
    expect(data.resolveErrors.town).toMatch(/no vault configured/);
  });

  test("saveLink writes the patch back to the file", async () => {
    const root = fakeRoot(YAML);
    const storage = createLocalFsStorage(root as unknown as FileSystemDirectoryHandle);
    const data = await storage.saveLink("town", "front-door", { hidden: true });
    expect(data.project.locations[0]!.links[0]!.hidden).toBe(true);

    // and it's really on disk (the fake's disk), not just in memory
    const reopened = await storage.open();
    expect(reopened.project.locations[0]!.links[0]!.hidden).toBe(true);
  });

  test("saveLocationContent updates the inline content", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const data = await storage.saveLocationContent("inn", { body: "Warm and loud." });
    expect(data.resolvedContent.inn).toEqual({ title: "The Inn", body: "Warm and loud." });
  });

  test("addLocationLink creates a new Location and links to it", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const data = await storage.addLocationLink("town", "old-mill", 5, 6, "landmark");
    expect(data.project.locations.find((l) => l.id === "old-mill")).toBeTruthy();
    expect(data.project.locations[0]!.links.map((l) => l.target)).toContain("old-mill");
  });

  test("saveGrid sets and clears the grid", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const grid = {
      type: "hex" as const,
      origin: { x: 0, y: 0 },
      b1: { x: 10, y: 0 },
      b2: { x: 5, y: 8 },
      style: { color: "#fff", weight: 1, opacity: 0.5 },
    };
    const withGrid = await storage.saveGrid("town", grid);
    expect(withGrid.project.locations[0]!.grid).toEqual(grid);

    const cleared = await storage.saveGrid("town", null);
    expect(cleared.project.locations[0]!.grid).toBeNull();
  });

  test("uploadImage writes into _assets and sets location.image", async () => {
    const root = fakeRoot(YAML);
    const storage = createLocalFsStorage(root as unknown as FileSystemDirectoryHandle);

    // a real, minimal PNG-shaped buffer (24 bytes, IHDR at the right offset)
    const bytes = new Uint8Array(24);
    const view = new DataView(bytes.buffer);
    bytes.set([0x49, 0x48, 0x44, 0x52], 12);
    view.setUint32(16, 400, false);
    view.setUint32(20, 300, false);
    const file = new File([bytes], "map.png", { type: "image/png" });

    const data = await storage.uploadImage("town", file);
    expect(data.project.locations[0]!.image).toEqual({ file: "_assets/town.png", width: 400, height: 300 });
    expect(root.dirs.get("_assets")?.files.has("town.png")).toBe(true);
  });

  test("uploadImage rejects an unsupported file type", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const file = new File([new Uint8Array([1, 2, 3])], "map.gif", { type: "image/gif" });
    await expect(storage.uploadImage("town", file)).rejects.toThrow(/PNG or JPEG/);
  });

  test("removeImage clears location.image", async () => {
    const storage = createLocalFsStorage(fakeRoot(YAML) as unknown as FileSystemDirectoryHandle);
    const data = await storage.removeImage("town");
    expect(data.project.locations[0]!.image).toBeNull();
  });

  test("getImageUrl reads the file via the folder handle", async () => {
    const root = fakeRoot(YAML);
    const assetsDir = new FakeDirectoryHandle("_assets");
    const imageFile = new FakeFileHandle("town.png");
    imageFile.content = new Uint8Array([1, 2, 3, 4]);
    assetsDir.files.set("town.png", imageFile);
    root.dirs.set("_assets", assetsDir);

    const storage = createLocalFsStorage(root as unknown as FileSystemDirectoryHandle);
    const url = await storage.getImageUrl("_assets/town.png");
    expect(url).toMatch(/^blob:/);
  });

  test("throws a clear error when the folder has no .hexen.yml", async () => {
    const storage = createLocalFsStorage(new FakeDirectoryHandle("empty") as unknown as FileSystemDirectoryHandle);
    await expect(storage.open()).rejects.toThrow(/No \.hexen\.yml file/);
  });
});
