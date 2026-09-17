import { useMemo, useState, type FormEvent } from "react";
import type { HexenProject } from "@hex-enductor/hexen-schema";
import type { OpenedProjectData, ProjectStorage } from "./storage/index.ts";

export interface LocationBrowserProps {
  project: HexenProject;
  resolvedContent: OpenedProjectData["resolvedContent"];
  currentLocationId: string | null;
  onSelect: (id: string) => void;
  onClose: () => void;
  /** Gates "New map…" — creating a Location is a mutation like any other. */
  editMode: boolean;
  storage: ProjectStorage;
}

// The pin-by-pin hierarchy is the normal way through a project, but
// there's no way to jump straight to "that one shop three maps deep"
// without following it — this is every Location, flat, filterable,
// independent of which map you're currently standing on. "Maps only"
// and "Orphans only" narrow it to exactly the locations a GM would
// want when staging new maps ahead of linking them in: has an image,
// and isn't the target of any Link yet (or the project's own
// defaultLocation, which is reachable by definition).
export function LocationBrowser({
  project,
  resolvedContent,
  currentLocationId,
  onSelect,
  onClose,
  editMode,
  storage,
}: LocationBrowserProps) {
  const [query, setQuery] = useState("");
  const [mapsOnly, setMapsOnly] = useState(false);
  const [orphansOnly, setOrphansOnly] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newId, setNewId] = useState("");
  const [createError, setCreateError] = useState<string | undefined>(undefined);

  const linkedTargets = useMemo(() => {
    const targets = new Set<string>();
    for (const location of project.locations) {
      for (const link of location.links) targets.add(link.target);
    }
    return targets;
  }, [project.locations]);

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return project.locations
      .map((location) => ({
        id: location.id,
        title: resolvedContent[location.id]?.title ?? location.id,
        hasImage: location.image !== null,
        isOrphan: location.id !== project.defaultLocation && !linkedTargets.has(location.id),
      }))
      .filter((row) => !mapsOnly || row.hasImage)
      .filter((row) => !orphansOnly || row.isOrphan)
      .filter((row) => !q || row.id.toLowerCase().includes(q) || row.title.toLowerCase().includes(q))
      .sort((a, b) => a.title.localeCompare(b.title));
  }, [project.locations, project.defaultLocation, resolvedContent, linkedTargets, query, mapsOnly, orphansOnly]);

  function createLocation(e: FormEvent) {
    e.preventDefault();
    const id = newId.trim();
    if (!id) return;
    if (project.locations.some((l) => l.id === id)) {
      setCreateError(`"${id}" already exists.`);
      return;
    }
    storage.execute({ type: "addLocation", locationId: id });
    onSelect(id);
    onClose();
  }

  return (
    <div className="location-browser">
      <button type="button" className="link-button picker-close" onClick={onClose}>
        Cancel
      </button>
      <h3>All locations ({project.locations.length})</h3>
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by name or id…"
        autoFocus
      />

      <div className="browser-filters">
        <label>
          <input type="checkbox" checked={mapsOnly} onChange={(e) => setMapsOnly(e.target.checked)} />
          Maps only (has an image)
        </label>
        <label>
          <input type="checkbox" checked={orphansOnly} onChange={(e) => setOrphansOnly(e.target.checked)} />
          Orphans only (not linked from anywhere)
        </label>
      </div>

      <div className="browser">
        <ul className="link-list">
          {rows.map((row) => (
            <li key={row.id}>
              <button
                className={row.id === currentLocationId ? "selected" : ""}
                onClick={() => {
                  onSelect(row.id);
                  onClose();
                }}
              >
                {row.title}
                {row.isOrphan ? " (orphan)" : ""}
              </button>
            </li>
          ))}
          {rows.length === 0 && <li className="muted">No locations match.</li>}
        </ul>
      </div>

      {editMode &&
        (creating ? (
          <form className="link-form" onSubmit={createLocation}>
            <label>
              New location id
              <input
                type="text"
                value={newId}
                onChange={(e) => {
                  setNewId(e.target.value);
                  setCreateError(undefined);
                }}
                placeholder="e.g. old-mill-cellar"
                autoFocus
              />
            </label>
            {createError && <span className="field-error">{createError}</span>}
            <div className="form-actions">
              <button type="submit" disabled={!newId.trim()}>
                Create
              </button>
              <button type="button" className="link-button" onClick={() => setCreating(false)}>
                Cancel
              </button>
            </div>
          </form>
        ) : (
          <button type="button" className="link-button" onClick={() => setCreating(true)}>
            New map…
          </button>
        ))}
    </div>
  );
}
