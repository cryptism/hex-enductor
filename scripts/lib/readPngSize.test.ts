import { describe, expect, test } from "bun:test";
import { readPngSize } from "./readPngSize.ts";

function fakePng(width: number, height: number): Buffer {
  const buf = Buffer.alloc(24);
  buf.write("IHDR", 12, "ascii");
  buf.writeUInt32BE(width, 16);
  buf.writeUInt32BE(height, 20);
  return buf;
}

describe("readPngSize", () => {
  test("reads width/height out of the IHDR chunk", () => {
    expect(readPngSize(fakePng(400, 300))).toEqual({ width: 400, height: 300 });
  });

  test("returns null for a buffer that's too short", () => {
    expect(readPngSize(Buffer.alloc(10))).toBeNull();
  });

  test("returns null when the IHDR marker isn't where expected", () => {
    const buf = fakePng(1, 1);
    buf.write("XXXX", 12, "ascii");
    expect(readPngSize(buf)).toBeNull();
  });
});
