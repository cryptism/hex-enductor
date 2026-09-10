export interface ImageSize {
  width: number;
  height: number;
}

/** PNG: 8-byte signature, then an IHDR chunk: 4-byte length, "IHDR", 4-byte width, 4-byte height (big-endian). */
function readPngSize(buf: Buffer): ImageSize | null {
  if (buf.length < 24 || buf.toString("ascii", 12, 16) !== "IHDR") return null;
  return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
}

/**
 * JPEG: SOI (0xFFD8), then a run of marker segments. Markers with no
 * length field (SOI/EOI/RSTn/TEM) are skipped bare; every other marker
 * is followed by a 2-byte big-endian segment length. The SOF marker
 * (0xFFC0-0xFFCF, excluding DHT/JPG/DAC at C4/C8/CC) carries height
 * then width as two big-endian shorts, right after a 1-byte precision.
 */
function readJpegSize(buf: Buffer): ImageSize | null {
  if (buf.length < 4 || buf[0] !== 0xff || buf[1] !== 0xd8) return null;

  let offset = 2;
  while (offset + 1 < buf.length) {
    if (buf[offset] !== 0xff) {
      offset++;
      continue;
    }
    const marker = buf[offset + 1]!;

    if (marker === 0xd8 || marker === 0xd9 || marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) {
      offset += 2;
      continue;
    }

    if (offset + 4 > buf.length) return null;
    const length = buf.readUInt16BE(offset + 2);
    const isSof = marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc;

    if (isSof) {
      if (offset + 9 > buf.length) return null;
      return { height: buf.readUInt16BE(offset + 5), width: buf.readUInt16BE(offset + 7) };
    }

    offset += 2 + length;
  }
  return null;
}

/** Dispatches on file signature, not the claimed extension — a mislabeled upload still measures correctly. */
export function readImageSize(buf: Buffer): ImageSize | null {
  return readPngSize(buf) ?? readJpegSize(buf);
}
