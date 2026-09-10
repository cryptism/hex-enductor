import { describe, expect, test } from "bun:test";
import { readImageSize } from "./imageSize.ts";

function fakePng(width: number, height: number): Buffer {
  const buf = Buffer.alloc(24);
  buf.write("IHDR", 12, "ascii");
  buf.writeUInt32BE(width, 16);
  buf.writeUInt32BE(height, 20);
  return buf;
}

function fakeJpeg(width: number, height: number): Buffer {
  const buf = Buffer.alloc(17);
  let o = 0;
  buf.writeUInt8(0xff, o++);
  buf.writeUInt8(0xd8, o++); // SOI
  buf.writeUInt8(0xff, o++);
  buf.writeUInt8(0xc0, o++); // SOF0
  buf.writeUInt16BE(11, o);
  o += 2; // segment length
  buf.writeUInt8(8, o++); // precision
  buf.writeUInt16BE(height, o);
  o += 2;
  buf.writeUInt16BE(width, o);
  o += 2;
  buf.writeUInt8(1, o++); // one component
  buf.writeUInt8(1, o++);
  buf.writeUInt8(0x11, o++);
  buf.writeUInt8(0, o++);
  buf.writeUInt8(0xff, o++);
  buf.writeUInt8(0xd9, o++); // EOI
  return buf;
}

describe("readImageSize", () => {
  test("reads a PNG's IHDR chunk", () => {
    expect(readImageSize(fakePng(400, 300))).toEqual({ width: 400, height: 300 });
  });

  test("reads a JPEG's SOF0 segment", () => {
    expect(readImageSize(fakeJpeg(640, 480))).toEqual({ width: 640, height: 480 });
  });

  test("skips a leading APP0 segment to find SOF0", () => {
    const app0 = Buffer.from([0xff, 0xe0, 0x00, 0x04, 0x00, 0x00]); // marker + 4-byte length (2 payload bytes)
    const jpeg = fakeJpeg(200, 100);
    const withApp0 = Buffer.concat([jpeg.subarray(0, 2), app0, jpeg.subarray(2)]);
    expect(readImageSize(withApp0)).toEqual({ width: 200, height: 100 });
  });

  test("returns null for a buffer that's too short", () => {
    expect(readImageSize(Buffer.alloc(3))).toBeNull();
  });

  test("returns null for neither PNG nor JPEG", () => {
    expect(readImageSize(Buffer.from("not an image, just text"))).toBeNull();
  });
});
