import { trpcClient, serverUrl } from "../trpc.ts";
import type { ProjectStorage } from "./types.ts";

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

/** Wraps the existing tRPC calls — the storage backend the app has had all along, just behind the ProjectStorage interface now. */
export function createServerStorage(path: string): ProjectStorage {
  return {
    label: path,

    open() {
      return trpcClient.openProject.query({ path });
    },

    refresh() {
      return trpcClient.openProject.query({ path });
    },

    async getImageUrl(file) {
      const dir = dirname(path);
      return `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&file=${encodeURIComponent(file)}`;
    },

    saveLink(locationId, linkId, patch) {
      return trpcClient.saveLink.mutate({ path, locationId, linkId, patch });
    },

    saveLocationContent(locationId, patch) {
      return trpcClient.saveLocationContent.mutate({ path, locationId, patch });
    },

    addLocationLink(parentLocationId, locationId, x, y, type) {
      return trpcClient.addLocationLink.mutate({ path, parentLocationId, locationId, x, y, type });
    },

    saveGrid(locationId, grid) {
      return trpcClient.saveGrid.mutate({ path, locationId, grid });
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
      const image = await res.json();

      return trpcClient.saveImage.mutate({ path, locationId, image });
    },

    removeImage(locationId) {
      return trpcClient.saveImage.mutate({ path, locationId, image: null });
    },
  };
}
