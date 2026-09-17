import { useEffect, useMemo, useRef, useState } from "react";
import Konva from "konva";
import { Stage, Layer, Image as KonvaImage, Line, Circle, Group, Shape } from "react-konva";
import useImage from "use-image";
import type { FogOfWar, Grid, ImageRef, Link, Point } from "@hex-enductor/hexen-schema";
import { loadHexBasis, buildHexPolygons } from "./hexMath.ts";
import { buildSquarePolygons } from "./squareMath.ts";
import { findLinkIcon } from "./linkIcons.ts";
import { FOG_CELL_SIZE, fogCellAt, hiddenFogCells } from "./fog.ts";
import { fogNoiseTextureDataUrl } from "./fogTexture.ts";

// Everything here works in the image's own pixel space (x right, y
// down) — the same space hexMath/squareMath/fog already use. A Konva
// Stage's own x/y/scale transform is what turns that into screen
// pixels; nothing downstream needs to know about it, unlike Leaflet's
// CRS.Simple + lat/lng round-trip this replaced (see git history
// around 2026-09-17 for that version, in packages/map-core/src/coords.ts).

const MIN_SCALE = 2 ** -4;
const MAX_SCALE = 2 ** 3;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/** A Stage event's pointer position, translated through the stage's current pan/zoom into image pixel space. */
function stagePointerToImagePoint(stage: Konva.Stage): Point | null {
  const pointer = stage.getPointerPosition();
  if (!pointer) return null;
  const p = stage.getAbsoluteTransform().copy().invert().point(pointer);
  return { x: p.x, y: p.y };
}

const PING_ANIMATION_COLOR = "#c19a5f";
const PING_RING_COUNT = 3;
const PING_RING_DELAY_S = 0.35;
const PING_RING_DURATION_S = 1.3;
const PING_RING_SIZE = 40;

/** Total time the whole radiating effect takes, in ms — callers should clear `pingAt` no sooner than this, or the last ring cuts off mid-animation. */
export const PING_EFFECT_DURATION_MS = ((PING_RING_COUNT - 1) * PING_RING_DELAY_S + PING_RING_DURATION_S) * 1000;

/** One radiating ring, scaling 0.3x -> 2.6x while fading out, starting after its own delay — a Konva.Tween equivalent of the old CSS @keyframes divIcon trick. */
function PingRing({ delay }: { delay: number }) {
  const ref = useRef<Konva.Circle>(null);

  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    node.scale({ x: 0.3, y: 0.3 });
    node.opacity(1);
    const timer = setTimeout(() => {
      new Konva.Tween({
        node,
        duration: PING_RING_DURATION_S,
        easing: Konva.Easings.EaseOut,
        scaleX: 2.6,
        scaleY: 2.6,
        opacity: 0,
      }).play();
    }, delay * 1000);
    return () => clearTimeout(timer);
  }, [delay]);

  return <Circle ref={ref} radius={PING_RING_SIZE / 2} stroke={PING_ANIMATION_COLOR} strokeWidth={3} opacity={0} listening={false} />;
}

function PingEffect({ x, y }: { x: number; y: number }) {
  return (
    <Group x={x} y={y} listening={false}>
      <Circle radius={4} fill={PING_ANIMATION_COLOR} />
      {Array.from({ length: PING_RING_COUNT }, (_, i) => (
        <PingRing key={i} delay={i * PING_RING_DELAY_S} />
      ))}
    </Group>
  );
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

function FogLayer({ image, fog, dimmed }: { image: ImageRef; fog: FogOfWar; dimmed: boolean }) {
  const [pattern] = useImage(fogNoiseTextureDataUrl());
  const hiddenCells = useMemo(() => hiddenFogCells(image, fog.revealedCells), [image, fog]);

  return (
    <Shape
      listening={false}
      opacity={dimmed ? FOG_OPACITY_ERASING : FOG_OPACITY}
      fillPatternImage={pattern}
      fillPatternRepeat="repeat"
      sceneFunc={(context, shape) => {
        context.beginPath();
        for (const key of hiddenCells) {
          const [col, row] = key.split(",").map(Number) as [number, number];
          const x0 = col * FOG_CELL_SIZE;
          const y0 = row * FOG_CELL_SIZE;
          const x1 = Math.min(x0 + FOG_CELL_SIZE, image.width);
          const y1 = Math.min(y0 + FOG_CELL_SIZE, image.height);
          context.rect(x0, y0, x1 - x0, y1 - y0);
        }
        context.fillStrokeShape(shape);
      }}
    />
  );
}

const DEFAULT_MARKER_COLOR = "#c19a5f";
// The SVG glyphs use fill="currentColor" for CSS-driven theming, which
// only works while they're live DOM — baked into a data: URI and
// decoded as a plain <img>, "currentColor" instead resolves to black.
// Substitute the intended color before encoding.
const GLYPH_COLOR = "#f2efe3";

function LinkMarker({
  link,
  color,
  isSelected,
  readOnly,
  onSelect,
}: {
  link: Link;
  color: string;
  isSelected: boolean;
  readOnly: boolean;
  onSelect: () => void;
}) {
  const size = isSelected ? 36 : 28;
  const glyphSize = Math.round(size * 0.68);
  const iconDef = findLinkIcon(link.icon);
  const iconUrl = iconDef ? `data:image/svg+xml,${encodeURIComponent(iconDef.svg.replace(/currentColor/g, GLYPH_COLOR))}` : "";
  const [glyphImg] = useImage(iconUrl);

  const handleSelect = (e: Konva.KonvaEventObject<MouseEvent | TouchEvent>) => {
    e.cancelBubble = true;
    onSelect();
  };

  return (
    <Group x={link.x} y={link.y} onClick={readOnly ? undefined : handleSelect} onTap={readOnly ? undefined : handleSelect}>
      <Circle radius={size / 2} fill="rgba(23,25,20,0.95)" stroke={color} strokeWidth={3} shadowColor="black" shadowOpacity={0.45} shadowBlur={4} />
      {glyphImg && <KonvaImage image={glyphImg} width={glyphSize} height={glyphSize} offsetX={glyphSize / 2} offsetY={glyphSize / 2} listening={false} />}
    </Group>
  );
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
  /** Follow mode, GM side: while set, reports this map's own center/zoom on every pan/zoom gesture. */
  onViewChange?: (view: { x: number; y: number; zoom: number }) => void;
  /** Follow mode, follower side: when non-null, imperatively drives this map to match — presentation only, never set alongside onViewChange. */
  followView?: { x: number; y: number; zoom: number } | null;
}

export function MapCanvas({
  image,
  imageUrl,
  grid,
  gridVisible = true,
  links,
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
  onViewChange,
  followView = null,
}: MapCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<Konva.Stage>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [baseImage] = useImage(imageUrl);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      setSize({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Fits the whole image in view once per location (keyed on imageUrl,
  // which changes whenever the underlying Location does) — deliberately
  // not reactive after that, same as react-leaflet's own `bounds` prop,
  // so it doesn't fight the user's own pan/zoom on every re-render.
  const fittedForRef = useRef<string | null>(null);
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage || size.width === 0 || size.height === 0) return;
    if (fittedForRef.current === imageUrl) return;
    const scale = clamp(Math.min(size.width / image.width, size.height / image.height), MIN_SCALE, MAX_SCALE);
    stage.scale({ x: scale, y: scale });
    stage.position({
      x: (size.width - image.width * scale) / 2,
      y: (size.height - image.height * scale) / 2,
    });
    stage.batchDraw();
    fittedForRef.current = imageUrl;
  }, [size.width, size.height, imageUrl, image.width, image.height]);

  // Follower side of Follow mode: drive the stage to match imperatively,
  // same spirit as the old ViewFollower's map.setView(..., {animate: true}).
  useEffect(() => {
    const stage = stageRef.current;
    if (!stage || !followView || size.width === 0 || size.height === 0) return;
    const scale = clamp(2 ** followView.zoom, MIN_SCALE, MAX_SCALE);
    stage.to({
      x: size.width / 2 - followView.x * scale,
      y: size.height / 2 - followView.y * scale,
      scaleX: scale,
      scaleY: scale,
      duration: 0.3,
      easing: Konva.Easings.EaseInOut,
    });
  }, [followView, size.width, size.height]);

  const reportViewTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  function reportView(stage: Konva.Stage) {
    if (!onViewChange) return;
    const scale = stage.scaleX();
    const center = stage.getAbsoluteTransform().copy().invert().point({ x: size.width / 2, y: size.height / 2 });
    onViewChange({ x: center.x, y: center.y, zoom: Math.log2(scale) });
  }
  function reportViewDebounced(stage: Konva.Stage) {
    if (reportViewTimer.current) clearTimeout(reportViewTimer.current);
    reportViewTimer.current = setTimeout(() => reportView(stage), 150);
  }

  function handleWheel(e: Konva.KonvaEventObject<WheelEvent>) {
    e.evt.preventDefault();
    const stage = e.target.getStage();
    if (!stage) return;
    const pointer = stage.getPointerPosition();
    if (!pointer) return;
    const oldScale = stage.scaleX();
    const mousePointTo = { x: (pointer.x - stage.x()) / oldScale, y: (pointer.y - stage.y()) / oldScale };
    const newScale = clamp(oldScale * Math.exp(-e.evt.deltaY * 0.001), MIN_SCALE, MAX_SCALE);
    stage.scale({ x: newScale, y: newScale });
    stage.position({ x: pointer.x - mousePointTo.x * newScale, y: pointer.y - mousePointTo.y * newScale });
    stage.batchDraw();
    reportViewDebounced(stage);
  }

  function handleDragEnd(e: Konva.KonvaEventObject<DragEvent>) {
    const stage = e.target.getStage();
    if (stage) reportView(stage);
  }

  function handleStageClick(e: Konva.KonvaEventObject<MouseEvent | TouchEvent>) {
    const stage = e.target.getStage();
    if (!stage) return;
    const point = stagePointerToImagePoint(stage);
    if (!point) return;
    if (placing && onPlaceLocation) onPlaceLocation(point);
    else if (pinging && onPing) onPing(point);
  }

  // Mounted only while fog painting is active. Click-drag reveals
  // whichever fog cells the cursor passes over; holding shift at the
  // start of the stroke restores fog (hides) instead — the same
  // mechanism, just the opposite direction, decided once per stroke.
  const stroke = useRef<{
    revealed: boolean;
    pending: Set<string>;
    lastCell: string | null;
    flushTimer: ReturnType<typeof setInterval>;
  } | null>(null);

  function flushStroke() {
    const s = stroke.current;
    if (!s || s.pending.size === 0 || !onPaintFogCells) return;
    onPaintFogCells([...s.pending], s.revealed);
    s.pending.clear();
  }

  function endStroke() {
    const s = stroke.current;
    if (!s) return;
    clearInterval(s.flushTimer);
    flushStroke();
    stroke.current = null;
  }

  useEffect(() => {
    if (!fogEditable || !fog) return;
    window.addEventListener("mouseup", endStroke);
    return () => {
      window.removeEventListener("mouseup", endStroke);
      endStroke();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fogEditable, fog]);

  function handleFogMouseDown(e: Konva.KonvaEventObject<MouseEvent>) {
    if (!fogEditable || !fog || !onPaintFogCells) return;
    const stage = e.target.getStage();
    if (!stage) return;
    const point = stagePointerToImagePoint(stage);
    if (!point) return;
    const revealed = !e.evt.shiftKey;
    const cell = fogCellAt(point);
    stroke.current = { revealed, pending: new Set([cell]), lastCell: cell, flushTimer: setInterval(flushStroke, FOG_PAINT_FLUSH_MS) };
  }

  function handleFogMouseMove(e: Konva.KonvaEventObject<MouseEvent>) {
    const s = stroke.current;
    if (!s) return;
    const stage = e.target.getStage();
    if (!stage) return;
    const point = stagePointerToImagePoint(stage);
    if (!point) return;
    const cell = fogCellAt(point);
    if (cell === s.lastCell) return;
    s.lastCell = cell;
    s.pending.add(cell);
  }

  // Held down while the paint tool is armed — previews erase mode by
  // dimming the fog layer, same key handleFogMouseDown reads to decide
  // a stroke's direction.
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

  const gridPolygons = useMemo(() => {
    if (!grid) return [];
    if (grid.type === "hex") {
      const basis = loadHexBasis(grid);
      return basis ? buildHexPolygons(basis, image.width, image.height) : [];
    }
    return buildSquarePolygons(grid, image.width, image.height);
  }, [grid, image.width, image.height]);

  const containerClass = placing || pinging ? "map-stage-container placing" : fogEditable && fog ? "map-stage-container painting-fog" : "map-stage-container";

  return (
    <div ref={containerRef} className={containerClass} style={{ width: "100%", height: "100%", background: "#12150f", overflow: "hidden" }}>
      {size.width > 0 && size.height > 0 && (
        <Stage
          ref={stageRef}
          width={size.width}
          height={size.height}
          draggable={!(fogEditable && fog)}
          onWheel={handleWheel}
          onDragEnd={handleDragEnd}
          onClick={handleStageClick}
          onTap={handleStageClick}
          onMouseDown={handleFogMouseDown}
          onMouseMove={handleFogMouseMove}
          onMouseUp={endStroke}
        >
          <Layer>{baseImage && <KonvaImage image={baseImage} width={image.width} height={image.height} listening={false} />}</Layer>

          <Layer>
            {gridVisible &&
              gridPolygons.map((corners, i) => (
                <Line
                  key={i}
                  points={corners.flatMap((p) => [p.x, p.y])}
                  closed
                  stroke={grid ? grid.style.color : DEFAULT_MARKER_COLOR}
                  strokeWidth={grid ? grid.style.weight : 1}
                  opacity={grid ? grid.style.opacity : 0.45}
                  listening={false}
                />
              ))}
          </Layer>

          <Layer>
            {links
              .filter((link) => !link.hidden)
              .map((link) => (
                <LinkMarker
                  key={link.id}
                  link={link}
                  color={link.color ?? DEFAULT_MARKER_COLOR}
                  isSelected={link.id === selectedLinkId}
                  readOnly={readOnly}
                  onSelect={() => onSelectLink?.(link.id)}
                />
              ))}
          </Layer>

          <Layer>{fog && <FogLayer image={image} fog={fog} dimmed={shiftHeld} />}</Layer>

          <Layer>{pingAt && <PingEffect key={pingAt.key} x={pingAt.x} y={pingAt.y} />}</Layer>
        </Stage>
      )}
    </div>
  );
}
