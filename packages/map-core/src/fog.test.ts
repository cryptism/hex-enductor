import { describe, expect, test } from "bun:test";
import { FOG_CELL_SIZE, fogCellAt, fogCellCorners, fogCellKey, fogGridDims, hiddenFogCells } from "./fog.ts";

describe("fogGridDims", () => {
  test("ceils to cover a partial trailing cell", () => {
    expect(fogGridDims({ width: FOG_CELL_SIZE * 2 + 1, height: FOG_CELL_SIZE })).toEqual({ cols: 3, rows: 1 });
  });

  test("is never smaller than 1x1", () => {
    expect(fogGridDims({ width: 1, height: 1 })).toEqual({ cols: 1, rows: 1 });
  });
});

describe("fogCellAt", () => {
  test("maps a pixel to its containing cell", () => {
    expect(fogCellAt({ x: 0, y: 0 })).toBe(fogCellKey(0, 0));
    expect(fogCellAt({ x: FOG_CELL_SIZE + 5, y: FOG_CELL_SIZE * 2 + 5 })).toBe(fogCellKey(1, 2));
  });
});

describe("fogCellCorners", () => {
  test("gives a full cell's corners away from the image edge", () => {
    const image = { width: FOG_CELL_SIZE * 4, height: FOG_CELL_SIZE * 4 };
    expect(fogCellCorners(1, 1, image)).toEqual([
      { x: FOG_CELL_SIZE, y: FOG_CELL_SIZE },
      { x: FOG_CELL_SIZE * 2, y: FOG_CELL_SIZE },
      { x: FOG_CELL_SIZE * 2, y: FOG_CELL_SIZE * 2 },
      { x: FOG_CELL_SIZE, y: FOG_CELL_SIZE * 2 },
    ]);
  });

  test("clamps a trailing cell to the image bounds", () => {
    const image = { width: FOG_CELL_SIZE + 10, height: FOG_CELL_SIZE + 10 };
    expect(fogCellCorners(1, 1, image)).toEqual([
      { x: FOG_CELL_SIZE, y: FOG_CELL_SIZE },
      { x: FOG_CELL_SIZE + 10, y: FOG_CELL_SIZE },
      { x: FOG_CELL_SIZE + 10, y: FOG_CELL_SIZE + 10 },
      { x: FOG_CELL_SIZE, y: FOG_CELL_SIZE + 10 },
    ]);
  });
});

describe("hiddenFogCells", () => {
  test("returns every cell when nothing is revealed", () => {
    const image = { width: FOG_CELL_SIZE * 2, height: FOG_CELL_SIZE };
    expect(hiddenFogCells(image, [])).toEqual([fogCellKey(0, 0), fogCellKey(1, 0)]);
  });

  test("excludes revealed cells", () => {
    const image = { width: FOG_CELL_SIZE * 2, height: FOG_CELL_SIZE };
    expect(hiddenFogCells(image, [fogCellKey(0, 0)])).toEqual([fogCellKey(1, 0)]);
  });

  test("returns nothing once every cell is revealed", () => {
    const image = { width: FOG_CELL_SIZE, height: FOG_CELL_SIZE };
    expect(hiddenFogCells(image, [fogCellKey(0, 0)])).toEqual([]);
  });
});
