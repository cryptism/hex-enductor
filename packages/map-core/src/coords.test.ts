import { describe, expect, test } from "bun:test";
import { pxToLatLng, polygonToLatLngs } from "./coords.ts";

describe("pxToLatLng", () => {
  test("flips y against the image height (pixel (0,0) is top-left, latlng (0,0) is bottom-left)", () => {
    expect(pxToLatLng(100, { x: 5, y: 20 })).toEqual([80, 5]);
  });

  test("a point at the image's bottom edge maps to latlng y=0", () => {
    expect(pxToLatLng(100, { x: 0, y: 100 })).toEqual([0, 0]);
  });
});

describe("polygonToLatLngs", () => {
  test("maps every point in order", () => {
    const points = [
      { x: 0, y: 0 },
      { x: 10, y: 10 },
    ];
    expect(polygonToLatLngs(100, points)).toEqual([
      [100, 0],
      [90, 10],
    ]);
  });
});
