import { useState } from "react";
import { fogGridDims, hiddenFogCells } from "@hex-enductor/map-core";
import type { FogOfWar, ImageRef } from "@hex-enductor/hexen-schema";

interface FogControlsProps {
  image: ImageRef;
  fog: FogOfWar | null;
  editMode: boolean;
  layerVisible: boolean;
  onSetLayerVisible: (visible: boolean) => void;
  paintingFog: boolean;
  onSetPaintingFog: (painting: boolean) => void;
  onSetFog: (fog: FogOfWar | null) => void;
}

type BlanketAction = "cover" | "reveal";

/**
 * GM mode's fog panel — a Krita-style layer toggle (peek under the fog
 * without changing it) plus a status readout that stays truthful about
 * what players see regardless of that peek, always available in GM
 * mode. The paint tool and blanket apply/remove only appear once
 * editMode is also on, since they mutate the project.
 */
export function FogControls({
  image,
  fog,
  editMode,
  layerVisible,
  onSetLayerVisible,
  paintingFog,
  onSetPaintingFog,
  onSetFog,
}: FogControlsProps) {
  const [pendingAction, setPendingAction] = useState<BlanketAction | null>(null);

  const { cols, rows } = fogGridDims(image);
  const totalCells = cols * rows;
  const hiddenCount = fog ? hiddenFogCells(image, fog.revealedCells).length : 0;

  return (
    <div className="fog-controls">
      <p className="fog-status">
        {fog
          ? `Fog is live for players — ${hiddenCount} of ${totalCells} cells hidden.`
          : "Fog is off — players see the full map."}
      </p>

      <label className="grid-toggle fog-layer-toggle">
        <input type="checkbox" checked={layerVisible} onChange={(e) => onSetLayerVisible(e.target.checked)} />
        Show fog layer
      </label>

      {editMode && (
        <>
          <button
            type="button"
            className={`tool-button${paintingFog ? " active" : ""}`}
            disabled={!fog}
            onClick={() => onSetPaintingFog(!paintingFog)}
          >
            {paintingFog ? "Click the map…" : "Paint fog"}
          </button>

          <div className="tool-row">
            <button type="button" className="tool-button" onClick={() => setPendingAction("cover")}>
              Fog entire map
            </button>
            <button type="button" className="tool-button" onClick={() => setPendingAction("reveal")}>
              Reveal entire map
            </button>
          </div>
        </>
      )}

      {pendingAction && (
        <div className="modal-backdrop">
          <div className="modal-panel">
            <p>
              {pendingAction === "cover"
                ? "Cover the entire map in fog? Any cells already revealed to players will be hidden again."
                : "Reveal the entire map? Fog will be turned off and players will see everything."}
            </p>
            <div className="form-actions">
              <button
                type="button"
                className="tool-button danger"
                onClick={() => {
                  onSetFog(pendingAction === "cover" ? { revealedCells: [] } : null);
                  setPendingAction(null);
                }}
              >
                Yes, {pendingAction === "cover" ? "fog it" : "reveal it"}
              </button>
              <button type="button" className="link-button" onClick={() => setPendingAction(null)}>
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
