import { useEffect, useState } from "react";
import type { Grid, GridStyle, ImageRef } from "@hex-enductor/hexen-schema";

const DEFAULT_STYLE: GridStyle = { color: "#c19a5f", weight: 1, opacity: 0.45 };

function defaultGridFor(image: ImageRef): Grid {
  return {
    type: "hex",
    origin: { x: Math.round(image.width / 2), y: Math.round(image.height / 2) },
    b1: { x: 60, y: 0 },
    b2: { x: 30, y: 52 },
    style: DEFAULT_STYLE,
  };
}

export interface ConfigureGridFormProps {
  initialGrid: Grid | null;
  image: ImageRef;
  /** Called on every field change — feed straight into MapCanvas so the edit is seen live against the real map. */
  onPreview: (grid: Grid) => void;
  onApply: (grid: Grid) => void;
  onCancel: () => void;
  saving: boolean;
}

// A document property, not a tool — this is the one thing in the
// toolbox that gets a Cancel button, because getting the lattice
// wrong is only obvious by looking at it against the real image.
export function ConfigureGridForm({ initialGrid, image, onPreview, onApply, onCancel, saving }: ConfigureGridFormProps) {
  const [grid, setGrid] = useState<Grid>(initialGrid ?? defaultGridFor(image));

  // Push the starting value (possibly the just-computed default) into
  // the live preview immediately, rather than waiting on a first edit.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => onPreview(grid), []);

  function set(next: Grid) {
    setGrid(next);
    onPreview(next);
  }

  return (
    <form
      className="link-form configure-grid"
      onSubmit={(e) => {
        e.preventDefault();
        onApply(grid);
      }}
    >
      <h3>Configure grid</h3>

      <label>
        Type
        <select
          value={grid.type}
          onChange={(e) => {
            const type = e.target.value as Grid["type"];
            if (type === grid.type) return;
            set(
              type === "hex"
                ? { type: "hex", origin: grid.origin, b1: { x: 60, y: 0 }, b2: { x: 30, y: 52 }, distancePerCell: grid.distancePerCell, style: grid.style }
                : { type: "square", origin: grid.origin, cellSize: { x: 64, y: 64 }, distancePerCell: grid.distancePerCell, style: grid.style },
            );
          }}
        >
          <option value="hex">Hex</option>
          <option value="square">Square</option>
        </select>
      </label>

      <fieldset>
        <legend>Origin</legend>
        <label>
          X
          <input
            type="number"
            value={grid.origin.x}
            onChange={(e) => set({ ...grid, origin: { ...grid.origin, x: Number(e.target.value) } })}
          />
        </label>
        <label>
          Y
          <input
            type="number"
            value={grid.origin.y}
            onChange={(e) => set({ ...grid, origin: { ...grid.origin, y: Number(e.target.value) } })}
          />
        </label>
      </fieldset>

      {grid.type === "hex" ? (
        <>
          <fieldset>
            <legend>Basis 1 (to a neighbouring hex)</legend>
            <label>
              X
              <input
                type="number"
                value={grid.b1.x}
                onChange={(e) => set({ ...grid, b1: { ...grid.b1, x: Number(e.target.value) } })}
              />
            </label>
            <label>
              Y
              <input
                type="number"
                value={grid.b1.y}
                onChange={(e) => set({ ...grid, b1: { ...grid.b1, y: Number(e.target.value) } })}
              />
            </label>
          </fieldset>
          <fieldset>
            <legend>Basis 2</legend>
            <label>
              X
              <input
                type="number"
                value={grid.b2.x}
                onChange={(e) => set({ ...grid, b2: { ...grid.b2, x: Number(e.target.value) } })}
              />
            </label>
            <label>
              Y
              <input
                type="number"
                value={grid.b2.y}
                onChange={(e) => set({ ...grid, b2: { ...grid.b2, y: Number(e.target.value) } })}
              />
            </label>
          </fieldset>
        </>
      ) : (
        <fieldset>
          <legend>Cell size</legend>
          <label>
            Width
            <input
              type="number"
              value={grid.cellSize.x}
              onChange={(e) => set({ ...grid, cellSize: { ...grid.cellSize, x: Number(e.target.value) } })}
            />
          </label>
          <label>
            Height
            <input
              type="number"
              value={grid.cellSize.y}
              onChange={(e) => set({ ...grid, cellSize: { ...grid.cellSize, y: Number(e.target.value) } })}
            />
          </label>
        </fieldset>
      )}

      <label>
        Distance per cell
        <input
          type="number"
          value={grid.distancePerCell ?? ""}
          placeholder="(none)"
          onChange={(e) =>
            set({ ...grid, distancePerCell: e.target.value === "" ? undefined : Number(e.target.value) })
          }
        />
      </label>

      <fieldset>
        <legend>Style</legend>
        <label>
          Color
          <input
            type="text"
            value={grid.style.color}
            onChange={(e) => set({ ...grid, style: { ...grid.style, color: e.target.value } })}
          />
        </label>
        <label>
          Weight
          <input
            type="number"
            value={grid.style.weight}
            onChange={(e) => set({ ...grid, style: { ...grid.style, weight: Number(e.target.value) } })}
          />
        </label>
        <label>
          Opacity
          <input
            type="number"
            step="0.05"
            min="0"
            max="1"
            value={grid.style.opacity}
            onChange={(e) => set({ ...grid, style: { ...grid.style, opacity: Number(e.target.value) } })}
          />
        </label>
      </fieldset>

      <div className="form-actions">
        <button type="submit" disabled={saving}>
          {saving ? "Applying…" : "Apply"}
        </button>
        <button type="button" className="link-button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}
