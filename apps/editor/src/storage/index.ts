export * from "./types.ts";
export { createServerStorage } from "./serverStorage.ts";
export { createLocalFsStorage } from "./localFsStorage.ts";

/** File System Access API support is Chromium-only today (no Firefox, no Safari). */
export function supportsLocalFs(): boolean {
  return typeof window !== "undefined" && "showDirectoryPicker" in window;
}
