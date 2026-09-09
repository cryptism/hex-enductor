/** PNG: 8-byte signature, then an IHDR chunk: 4-byte length, "IHDR", 4-byte width, 4-byte height (big-endian). */
export function readPngSize(buf: Buffer): { width: number; height: number } | null {
  if (buf.length < 24 || buf.toString("ascii", 12, 16) !== "IHDR") return null;
  return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
}
