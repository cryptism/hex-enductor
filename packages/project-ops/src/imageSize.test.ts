import { describe, expect, test } from "bun:test";
import { readImageSize } from "./imageSize.ts";

function fakePng(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(24);
  const view = new DataView(bytes.buffer);
  bytes.set([0x49, 0x48, 0x44, 0x52], 12); // "IHDR"
  view.setUint32(16, width, false);
  view.setUint32(20, height, false);
  return bytes;
}

function fakeJpeg(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(17);
  const view = new DataView(bytes.buffer);
  bytes[0] = 0xff;
  bytes[1] = 0xd8; // SOI
  bytes[2] = 0xff;
  bytes[3] = 0xc0; // SOF0
  view.setUint16(4, 11, false); // segment length
  bytes[6] = 8; // precision
  view.setUint16(7, height, false);
  view.setUint16(9, width, false);
  bytes[11] = 1; // one component
  bytes[12] = 1;
  bytes[13] = 0x11;
  bytes[14] = 0;
  bytes[15] = 0xff;
  bytes[16] = 0xd9; // EOI
  return bytes;
}

describe("readImageSize", () => {
  test("reads a PNG's IHDR chunk", () => {
    expect(readImageSize(fakePng(400, 300))).toEqual({ width: 400, height: 300 });
  });

  test("reads a JPEG's SOF0 segment", () => {
    expect(readImageSize(fakeJpeg(640, 480))).toEqual({ width: 640, height: 480 });
  });

  test("skips a leading APP0 segment to find SOF0", () => {
    const app0 = new Uint8Array([0xff, 0xe0, 0x00, 0x04, 0x00, 0x00]);
    const jpeg = fakeJpeg(200, 100);
    const withApp0 = new Uint8Array(jpeg.length + app0.length);
    withApp0.set(jpeg.subarray(0, 2), 0);
    withApp0.set(app0, 2);
    withApp0.set(jpeg.subarray(2), 2 + app0.length);
    expect(readImageSize(withApp0)).toEqual({ width: 200, height: 100 });
  });

  test("returns null for a buffer that's too short", () => {
    expect(readImageSize(new Uint8Array(3))).toBeNull();
  });

  test("returns null for neither PNG nor JPEG", () => {
    expect(readImageSize(new TextEncoder().encode("not an image, just text"))).toBeNull();
  });

  test("works with a Node Buffer too, since it's a Uint8Array subclass", () => {
    expect(readImageSize(Buffer.from(fakePng(10, 20)))).toEqual({ width: 10, height: 20 });
  });
});
