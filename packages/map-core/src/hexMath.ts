import type { HexGrid, Point } from "@hex-enductor/hexen-schema";

// Ported from illuminated-world/site/maps/lib/app.js — see that file's
// own comment block for the derivation. A calibration is an origin
// pixel plus two basis vectors (b1, b2) spanning the hex lattice; any
// pixel converts to axial (q, r) via the inverse of the 2x2 matrix
// [b1 b2]. A regular hexagon's corners sit at radius |b1|/sqrt(3),
// starting 30deg off b1's angle and stepping every 60deg — true
// regardless of grid orientation or which two neighbour directions
// were used to calibrate.

interface Matrix2x2Inverse {
  a: number;
  b: number;
  c: number;
  d: number;
}

export interface HexBasis {
  origin: Point;
  b1: Point;
  b2: Point;
  inv: Matrix2x2Inverse;
}

export interface Axial {
  q: number;
  r: number;
}

function invert2x2(b1: Point, b2: Point): Matrix2x2Inverse | null {
  const det = b1.x * b2.y - b2.x * b1.y;
  if (Math.abs(det) < 1e-9) return null;
  return {
    a: b2.y / det,
    b: -b2.x / det,
    c: -b1.y / det,
    d: b1.x / det,
  };
}

export function loadHexBasis(grid: HexGrid): HexBasis | null {
  const inv = invert2x2(grid.b1, grid.b2);
  if (!inv) return null;
  return { origin: grid.origin, b1: grid.b1, b2: grid.b2, inv };
}

export function pxToAxial(basis: HexBasis, p: Point): Axial {
  const dx = p.x - basis.origin.x;
  const dy = p.y - basis.origin.y;
  const q = basis.inv.a * dx + basis.inv.b * dy;
  const r = basis.inv.c * dx + basis.inv.d * dy;
  return { q: Math.round(q), r: Math.round(r) };
}

export function hexDistance(basis: HexBasis, pA: Point, pB: Point): number {
  const a = pxToAxial(basis, pA);
  const b = pxToAxial(basis, pB);
  const dq = a.q - b.q;
  const dr = a.r - b.r;
  return (Math.abs(dq) + Math.abs(dq + dr) + Math.abs(dr)) / 2;
}

export function hexCorners(center: Point, b1: Point): Point[] {
  const s = Math.hypot(b1.x, b1.y);
  const r = s / Math.sqrt(3);
  const angle0 = Math.atan2(b1.y, b1.x) + Math.PI / 6; // b1's angle + 30deg
  const pts: Point[] = [];
  for (let k = 0; k < 6; k++) {
    const a = angle0 + k * (Math.PI / 3);
    pts.push({ x: center.x + r * Math.cos(a), y: center.y + r * Math.sin(a) });
  }
  return pts;
}

/** Every hex polygon (as pixel-space corner arrays) whose centre falls within one hex of the image bounds. */
export function buildHexPolygons(
  basis: HexBasis,
  imageWidth: number,
  imageHeight: number,
): Point[][] {
  const corners = [
    { x: 0, y: 0 },
    { x: imageWidth, y: 0 },
    { x: 0, y: imageHeight },
    { x: imageWidth, y: imageHeight },
  ];
  const qs: number[] = [];
  const rs: number[] = [];
  for (const c of corners) {
    const dx = c.x - basis.origin.x;
    const dy = c.y - basis.origin.y;
    qs.push(basis.inv.a * dx + basis.inv.b * dy);
    rs.push(basis.inv.c * dx + basis.inv.d * dy);
  }
  const qMin = Math.floor(Math.min(...qs)) - 1;
  const qMax = Math.ceil(Math.max(...qs)) + 1;
  const rMin = Math.floor(Math.min(...rs)) - 1;
  const rMax = Math.ceil(Math.max(...rs)) + 1;

  const polygons: Point[][] = [];
  for (let q = qMin; q <= qMax; q++) {
    for (let r = rMin; r <= rMax; r++) {
      const center = {
        x: basis.origin.x + q * basis.b1.x + r * basis.b2.x,
        y: basis.origin.y + q * basis.b1.y + r * basis.b2.y,
      };
      if (
        center.x < -100 ||
        center.x > imageWidth + 100 ||
        center.y < -100 ||
        center.y > imageHeight + 100
      ) {
        continue;
      }
      polygons.push(hexCorners(center, basis.b1));
    }
  }
  return polygons;
}
