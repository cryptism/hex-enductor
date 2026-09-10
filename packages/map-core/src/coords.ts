import type { LatLng, LatLngExpression } from "leaflet";
import type { Point } from "@hex-enductor/hexen-schema";

/**
 * L.CRS.Simple maps image pixels to lat/lng with y flipped (image
 * pixel (0,0) is top-left, but latlng (0,0) is bottom-left) — same
 * convention as illuminated-world/site/maps/lib/app.js.
 */
export function pxToLatLng(imageHeight: number, p: Point): LatLngExpression {
  return [imageHeight - p.y, p.x];
}

export function polygonToLatLngs(imageHeight: number, points: Point[]): LatLngExpression[] {
  return points.map((p) => pxToLatLng(imageHeight, p));
}

/** The inverse of pxToLatLng — a map click's latlng back to an image pixel. */
export function latLngToPx(imageHeight: number, latlng: LatLng): Point {
  return { x: latlng.lng, y: imageHeight - latlng.lat };
}
