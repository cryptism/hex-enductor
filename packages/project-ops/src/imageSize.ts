export interface ImageSize {
  width: number;
  height: number;
}

/**
 * Uint8Array/DataView-based, not Node's Buffer, so this runs
 * unmodified in the browser as well as under Bun/Node — Buffer is a
 * Uint8Array subclass, so passing one in from server code still works.
 */

/** PNG: 8-byte signature, then an IHDR chunk: 4-byte length, "IHDR", 4-byte width, 4-byte height (big-endian). */
function readPngSize(bytes: Uint8Array): ImageSize | null {
  if (bytes.length < 24) return null;
  if (new TextDecoder("ascii").decode(bytes.subarray(12, 16)) !== "IHDR") return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return { width: view.getUint32(16, false), height: view.getUint32(20, false) };
}

/**
 * JPEG: SOI (0xFFD8), then a run of marker segments. Markers with no
 * length field (SOI/EOI/RSTn/TEM) are skipped bare; every other marker
 * is followed by a 2-byte big-endian segment length. The SOF marker
 * (0xFFC0-0xFFCF, excluding DHT/JPG/DAC at C4/C8/CC) carries height
 * then width as two big-endian shorts, right after a 1-byte precision.
 */
function readJpegSize(bytes: Uint8Array): ImageSize | null {
  if (bytes.length < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8) return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

  let offset = 2;
  while (offset + 1 < bytes.length) {
    if (bytes[offset] !== 0xff) {
      offset++;
      continue;
    }
    const marker = bytes[offset + 1]!;

    if (marker === 0xd8 || marker === 0xd9 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) {
      offset += 2;
      continue;
    }

    if (offset + 4 > bytes.length) return null;
    const length = view.getUint16(offset + 2, false);
    const isSof = marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc;

    if (isSof) {
      if (offset + 9 > bytes.length) return null;
      return { height: view.getUint16(offset + 5, false), width: view.getUint16(offset + 7, false) };
    }

    offset += 2 + length;
  }
  return null;
}

/** Dispatches on file signature, not the claimed extension — a mislabeled upload still measures correctly. */
export function readImageSize(bytes: Uint8Array): ImageSize | null {
  return readPngSize(bytes) ?? readJpegSize(bytes);
}
