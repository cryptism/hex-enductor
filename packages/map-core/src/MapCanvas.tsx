import { useMemo } from "react";
import { CRS, divIcon } from "leaflet";
import { MapContainer, ImageOverlay, Polygon, Marker, Popup, useMapEvents } from "react-leaflet";
import type { Grid, ImageRef, Link, Point } from "@hex-enductor/hexen-schema";
import { loadHexBasis, buildHexPolygons } from "./hexMath.ts";
import { buildSquarePolygons } from "./squareMath.ts";
import { pxToLatLng, polygonToLatLngs, latLngToPx } from "./coords.ts";
import "leaflet/dist/leaflet.css";

// Mounted only while the Add Location tool is active — has no
// rendered output of its own, it just wires the map's native click
// event to onPlace so the parent can turn it into a new pin.
function ClickToPlace({ imageHeight, onPlace }: { imageHeight: number; onPlace: (point: Point) => void }) {
  useMapEvents({
    click: (e) => onPlace(latLngToPx(imageHeight, e.latlng)),
  });
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
  /** link.id -> the referenced Location's resolved title (a Link never carries its own title — see @hex-enductor/hexen-schema). */
  linkTitles: Record<string, string>;
  selectedLinkId?: string;
  onSelectLink?: (linkId: string) => void;
  /** The Add Location tool: while true, clicking the map calls onPlaceLocation instead of panning-only. */
  placing?: boolean;
  onPlaceLocation?: (point: Point) => void;
  /** Presentation/wiki-embed mode — no interaction, just the rendered map. Not exercised anywhere yet. */
  readOnly?: boolean;
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

  return (
    <MapContainer
      crs={CRS.Simple}
      bounds={bounds}
      className={placing ? "placing" : undefined}
      style={{ width: "100%", height: "100%", background: "#12150f" }}
      zoomSnap={0.25}
      minZoom={-4}
      maxZoom={3}
    >
      {placing && onPlaceLocation && <ClickToPlace imageHeight={image.height} onPlace={onPlaceLocation} />}
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
          const icon = divIcon({
            className: "",
            html: `<div style="
              width: ${isSelected ? 20 : 16}px;
              height: ${isSelected ? 20 : 16}px;
              border-radius: 50%;
              background: rgba(23,25,20,0.88);
              border: 2px solid ${color};
              box-shadow: 0 0 0 2px rgba(0,0,0,0.35);
            "></div>`,
            iconSize: [20, 20],
            iconAnchor: [10, 10],
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
                <strong>{linkTitles[link.id] ?? link.id}</strong>
                <br />
                <span style={{ opacity: 0.7 }}>{link.type}</span>
              </Popup>
            </Marker>
          );
        })}
    </MapContainer>
  );
}
