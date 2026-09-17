import { useEffect, useMemo, useRef, useState } from "react";
import { CRS, divIcon } from "leaflet";
import { MapContainer, ImageOverlay, Polygon, Marker, Popup, useMap, useMapEvents } from "react-leaflet";
import type { FogOfWar, Grid, ImageRef, Link, Point } from "@hex-enductor/hexen-schema";
import { loadHexBasis, buildHexPolygons } from "./hexMath.ts";
import { buildSquarePolygons } from "./squareMath.ts";
import { pxToLatLng, polygonToLatLngs, latLngToPx } from "./coords.ts";
import { findLinkIcon } from "./linkIcons.ts";
import { fogCellAt, fogCellCorners, hiddenFogCells } from "./fog.ts";
import { fogNoiseTextureDataUrl, FOG_TEXTURE_SIZE } from "./fogTexture.ts";
import "leaflet/dist/leaflet.css";

const FOG_PATTERN_ID = "hexenductor-fog-noise";

// Leaflet's default (SVG) renderer draws Polygons as <path> elements
// inside one <svg> per map, but gives no way to add arbitrary <defs>
// through react-leaflet's own API — so this reaches into that <svg>
// directly and injects a <pattern> once, referenced by the fog
// polygons below as `fill="url(#hexenductor-fog-noise)"`.
// patternUnits="userSpaceOnUse" ties the tile to map coordinates (not
// screen pixels), so it pans and zooms with the map like anything else
// drawn on it, instead of looking pasted onto the viewport.
function FogNoiseDefs() {
  const map = useMap();
  useEffect(() => {
    const svg = map.getPane("overlayPane")?.querySelector("svg");
    if (!svg || svg.querySelector(`#${FOG_PATTERN_ID}`)) return;

    const ns = "http://www.w3.org/2000/svg";
    let defs = svg.querySelector("defs");
    if (!defs) {
      defs = document.createElementNS(ns, "defs");
      svg.insertBefore(defs, svg.firstChild);
    }

    const pattern = document.createElementNS(ns, "pattern");
    pattern.setAttribute("id", FOG_PATTERN_ID);
    pattern.setAttribute("patternUnits", "userSpaceOnUse");
    pattern.setAttribute("width", String(FOG_TEXTURE_SIZE));
    pattern.setAttribute("height", String(FOG_TEXTURE_SIZE));

    const image = document.createElementNS(ns, "image");
    image.setAttribute("href", fogNoiseTextureDataUrl());
    image.setAttribute("width", String(FOG_TEXTURE_SIZE));
    image.setAttribute("height", String(FOG_TEXTURE_SIZE));
    pattern.appendChild(image);
    defs.appendChild(pattern);

    return () => pattern.remove();
  }, [map]);

  return null;
}

// Mounted only while the Add Location tool is active — has no
// rendered output of its own, it just wires the map's native click
// event to onPlace so the parent can turn it into a new pin. The Ping
// tool reuses this exact same mechanic — armed click-to-place — just
// wired to a different callback.
function ClickToPlace({ imageHeight, onPlace }: { imageHeight: number; onPlace: (point: Point) => void }) {
  useMapEvents({
    click: (e) => onPlace(latLngToPx(imageHeight, e.latlng)),
  });
  return null;
}

const PING_ANIMATION_NAME = "hexenductor-ping";
let pingStyleInjected = false;

// Injected once, globally — cheaper than redeclaring the @keyframes
// inside every ping marker's own divIcon HTML (which would still work,
// just duplicated on every ping).
function ensurePingStyleInjected(): void {
  if (pingStyleInjected) return;
  pingStyleInjected = true;
  const style = document.createElement("style");
  style.textContent = `@keyframes ${PING_ANIMATION_NAME} { 0% { transform: scale(0.3); opacity: 1; } 100% { transform: scale(2.6); opacity: 0; } }`;
  document.head.appendChild(style);
}

const PING_RING_COUNT = 3;
const PING_RING_DELAY_S = 0.35;
const PING_RING_DURATION_S = 1.3;
const PING_RING_SIZE = 40;
const PING_BOX_SIZE = 120; // must comfortably fit the largest ring at its final (2.6x) scale

/** Total time the whole radiating effect takes, in ms — callers should clear `pingAt` no sooner than this, or the last ring cuts off mid-animation. */
export const PING_EFFECT_DURATION_MS = ((PING_RING_COUNT - 1) * PING_RING_DELAY_S + PING_RING_DURATION_S) * 1000;

function pingIcon() {
  ensurePingStyleInjected();
  const half = PING_RING_SIZE / 2;
  const rings = Array.from({ length: PING_RING_COUNT }, (_, i) => {
    const delay = i * PING_RING_DELAY_S;
    return `<div style="
      position: absolute;
      top: 50%;
      left: 50%;
      width: ${PING_RING_SIZE}px;
      height: ${PING_RING_SIZE}px;
      margin: -${half}px 0 0 -${half}px;
      box-sizing: border-box;
      border-radius: 50%;
      border: 3px solid #c19a5f;
      opacity: 0;
      animation: ${PING_ANIMATION_NAME} ${PING_RING_DURATION_S}s ease-out ${delay}s forwards;
    "></div>`;
  }).join("");

  return divIcon({
    className: "",
    html: `<div style="position: relative; width: ${PING_BOX_SIZE}px; height: ${PING_BOX_SIZE}px;">
      <div style="
        position: absolute;
        top: 50%;
        left: 50%;
        width: 8px;
        height: 8px;
        margin: -4px 0 0 -4px;
        border-radius: 50%;
        background: #c19a5f;
      "></div>
      ${rings}
    </div>`,
    iconSize: [PING_BOX_SIZE, PING_BOX_SIZE],
    iconAnchor: [PING_BOX_SIZE / 2, PING_BOX_SIZE / 2],
  });
}

// How often a held-down paint stroke flushes its touched cells as one
// setFogCells command — batched so a fast drag across many cells lands
// (and broadcasts to every other viewer) as a handful of updates, not
// one per cell.
const FOG_PAINT_FLUSH_MS = 80;

const FOG_OPACITY = 0.92;
// Held down while painting, the fog layer dims so the GM can see what
// they're about to re-cover instead of painting blind.
const FOG_OPACITY_ERASING = 0.35;

// Mounted only in fog-editable mode. Click-drag reveals whichever fog
// cells the cursor passes over; holding shift at the start of the
// stroke restores fog (hides) instead — the same mechanism, just the
// opposite direction, decided once per stroke. Disables map panning
// for the duration so a drag paints instead of scrolling the map.
function FogPaintHandler({
  imageHeight,
  onPaint,
}: {
  imageHeight: number;
  onPaint: (cells: string[], revealed: boolean) => void;
}) {
  const stroke = useRef<{
    revealed: boolean;
    pending: Set<string>;
    lastCell: string | null;
    flushTimer: ReturnType<typeof setInterval>;
  } | null>(null);

  function flush() {
    const s = stroke.current;
    if (!s || s.pending.size === 0) return;
    onPaint([...s.pending], s.revealed);
    s.pending.clear();
  }

  function endStroke() {
    const s = stroke.current;
    if (!s) return;
    clearInterval(s.flushTimer);
    flush();
    stroke.current = null;
  }

  const map = useMapEvents({
    mousedown: (e) => {
      const revealed = !(e.originalEvent as MouseEvent).shiftKey;
      const cell = fogCellAt(latLngToPx(imageHeight, e.latlng));
      stroke.current = {
        revealed,
        pending: new Set([cell]),
        lastCell: cell,
        flushTimer: setInterval(flush, FOG_PAINT_FLUSH_MS),
      };
    },
    mousemove: (e) => {
      const s = stroke.current;
      if (!s) return;
      const cell = fogCellAt(latLngToPx(imageHeight, e.latlng));
      if (cell === s.lastCell) return;
      s.lastCell = cell;
      s.pending.add(cell);
    },
    mouseup: endStroke,
  });

  useEffect(() => {
    map.dragging.disable();
    window.addEventListener("mouseup", endStroke);
    return () => {
      map.dragging.enable();
      window.removeEventListener("mouseup", endStroke);
      endStroke();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [map]);

  return null;
}

export interface MapCanvasProps {
  image: ImageRef;
  /** Wherever the caller has made image.file reachable — a dev server route, a blob URL, whatever. */
  imageUrl: string;
  grid: Grid | null;
  /** Show the grid overlay at all — a view toggle, independent of whether `grid` itself is configured. */
  gridVisible?: boolean;
  links: Link[];
  /** link.target -> the referenced Location's resolved title (a Link never carries its own title — see @hex-enductor/hexen-schema). */
  linkTitles: Record<string, string>;
  selectedLinkId?: string;
  onSelectLink?: (linkId: string) => void;
  /** The Add Location tool: while true, clicking the map calls onPlaceLocation instead of panning-only. */
  placing?: boolean;
  onPlaceLocation?: (point: Point) => void;
  /** Presentation/wiki-embed mode — no interaction, just the rendered map. Not exercised anywhere yet. */
  readOnly?: boolean;
  /** Null/absent means fog is off for this Location — nothing is drawn. */
  fog?: FogOfWar | null;
  /** While true (and fog is present), click-drag paints fog cells instead of panning the map. */
  fogEditable?: boolean;
  onPaintFogCells?: (cells: string[], revealed: boolean) => void;
  /** The Ping tool: while true, clicking the map calls onPing instead of panning/selecting. */
  pinging?: boolean;
  onPing?: (point: Point) => void;
  /** A transient "look here" highlight. Include a fresh `key` even for repeat pings at the same spot so the animation restarts — MapCanvas doesn't time this out on its own, the caller clears it. */
  pingAt?: { x: number; y: number; key: number } | null;
}

const DEFAULT_MARKER_COLOR = "#c19a5f";

export function MapCanvas({
  image,
  imageUrl,
  grid,
  gridVisible = true,
  links,
  linkTitles,
  selectedLinkId,
  onSelectLink,
  placing = false,
  onPlaceLocation,
  readOnly = false,
  fog = null,
  fogEditable = false,
  onPaintFogCells,
  pinging = false,
  onPing,
  pingAt = null,
}: MapCanvasProps) {
  const bounds: [[number, number], [number, number]] = [
    [0, 0],
    [image.height, image.width],
  ];

  const gridPolygons = useMemo(() => {
    if (!grid) return [];
    if (grid.type === "hex") {
      const basis = loadHexBasis(grid);
      return basis ? buildHexPolygons(basis, image.width, image.height) : [];
    }
    return buildSquarePolygons(grid, image.width, image.height);
  }, [grid, image.width, image.height]);

  const fogCellPolygons = useMemo(() => {
    if (!fog) return [];
    return hiddenFogCells(image, fog.revealedCells).map((key) => {
      const [col, row] = key.split(",").map(Number) as [number, number];
      return fogCellCorners(col, row, image);
    });
  }, [fog, image.width, image.height]);

  // Only tracked while the paint tool is armed — holding shift previews
  // erase mode by dimming the fog layer, same key FogPaintHandler reads
  // to decide a stroke's direction.
  const [shiftHeld, setShiftHeld] = useState(false);
  useEffect(() => {
    if (!fogEditable) {
      setShiftHeld(false);
      return;
    }
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Shift") setShiftHeld(true);
    }
    function onKeyUp(e: KeyboardEvent) {
      if (e.key === "Shift") setShiftHeld(false);
    }
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      setShiftHeld(false);
    };
  }, [fogEditable]);

  return (
    <MapContainer
      crs={CRS.Simple}
      bounds={bounds}
      className={placing || pinging ? "placing" : fogEditable && fog ? "painting-fog" : undefined}
      style={{ width: "100%", height: "100%", background: "#12150f" }}
      zoomSnap={0.25}
      minZoom={-4}
      maxZoom={3}
      attributionControl={false}
    >
      {placing && onPlaceLocation && <ClickToPlace imageHeight={image.height} onPlace={onPlaceLocation} />}
      {pinging && onPing && <ClickToPlace imageHeight={image.height} onPlace={onPing} />}
      {fogEditable && fog && onPaintFogCells && (
        <FogPaintHandler imageHeight={image.height} onPaint={onPaintFogCells} />
      )}
      {fogCellPolygons.length > 0 && <FogNoiseDefs />}
      <ImageOverlay url={imageUrl} bounds={bounds} />

      {gridVisible && gridPolygons.map((corners, i) => (
        <Polygon
          key={i}
          positions={polygonToLatLngs(image.height, corners)}
          pathOptions={{
            color: grid ? grid.style.color : DEFAULT_MARKER_COLOR,
            weight: grid ? grid.style.weight : 1,
            opacity: grid ? grid.style.opacity : 0.45,
            fill: false,
            interactive: false,
          }}
        />
      ))}

      {links
        .filter((link) => !link.hidden)
        .map((link) => {
          const color = link.color ?? DEFAULT_MARKER_COLOR;
          const isSelected = link.id === selectedLinkId;
          const size = isSelected ? 36 : 28;
          const glyph = findLinkIcon(link.icon)?.svg ?? "";
          const glyphSize = Math.round(size * 0.68);
          const icon = divIcon({
            className: "",
            html: `<div style="
              width: ${size}px;
              height: ${size}px;
              border-radius: 50%;
              background: rgba(23,25,20,0.95);
              border: 3px solid ${color};
              box-shadow: 0 0 0 2px rgba(0,0,0,0.45), 0 1px 4px rgba(0,0,0,0.6);
              display: flex;
              align-items: center;
              justify-content: center;
              color: #f2efe3;
            ">${glyph.replace("<svg ", `<svg width="${glyphSize}" height="${glyphSize}" `)}</div>`,
            iconSize: [size, size],
            iconAnchor: [size / 2, size / 2],
          });

          return (
            <Marker
              key={link.id}
              position={pxToLatLng(image.height, link)}
              icon={icon}
              eventHandlers={
                readOnly
                  ? {}
                  : {
                      click: () => onSelectLink?.(link.id),
                    }
              }
            >
              <Popup>
                <strong>{linkTitles[link.target] ?? link.target}</strong>
                <br />
                <span style={{ opacity: 0.7 }}>{link.type}</span>
              </Popup>
            </Marker>
          );
        })}

      {fogCellPolygons.map((corners, i) => (
        <Polygon
          key={i}
          positions={polygonToLatLngs(image.height, corners)}
          pathOptions={{
            color: "transparent",
            fillColor: `url(#${FOG_PATTERN_ID})`,
            fillOpacity: shiftHeld ? FOG_OPACITY_ERASING : FOG_OPACITY,
            interactive: false,
          }}
        />
      ))}

      {pingAt && <Marker key={pingAt.key} position={pxToLatLng(image.height, pingAt)} icon={pingIcon()} interactive={false} />}
    </MapContainer>
  );
}
