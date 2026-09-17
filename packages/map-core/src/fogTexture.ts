import { fbm2 } from "./perlinNoise.ts";

/** Tile size of the generated texture, in pixels — also what callers should use as the SVG pattern's tile size so it maps 1:1, no scaling artifacts. */
export const FOG_TEXTURE_SIZE = 128;

const FOG_BASE_COLOR: [number, number, number] = [10, 10, 8]; // #0a0a08, the flat color this replaces
const FOG_LIGHT_COLOR: [number, number, number] = [42, 44, 35]; // a mottled, slightly warmer highlight

let cached: string | null = null;

/**
 * A tiled, Perlin-fbm-textured version of the fog's base color, as a
 * data: URL — computed once per page load and cached, since it's the
 * same for every fog cell on every map. Fully opaque; the existing
 * per-polygon `fillOpacity` (see MapCanvas.tsx) still does the
 * shift-to-peek dimming, independent of this texture.
 */
export function fogNoiseTextureDataUrl(): string {
  if (cached) return cached;

  const size = FOG_TEXTURE_SIZE;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context unavailable");

  const image = ctx.createImageData(size, size);
  const scale = 4; // how many noise "cells" the tile spans — higher = finer mottling
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const n = fbm2((x / size) * scale, (y / size) * scale);
      const t = (n + 1) / 2; // 0..1
      const i = (y * size + x) * 4;
      image.data[i] = Math.round(FOG_BASE_COLOR[0] + t * (FOG_LIGHT_COLOR[0] - FOG_BASE_COLOR[0]));
      image.data[i + 1] = Math.round(FOG_BASE_COLOR[1] + t * (FOG_LIGHT_COLOR[1] - FOG_BASE_COLOR[1]));
      image.data[i + 2] = Math.round(FOG_BASE_COLOR[2] + t * (FOG_LIGHT_COLOR[2] - FOG_BASE_COLOR[2]));
      image.data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);

  cached = canvas.toDataURL("image/png");
  return cached;
}
