import { useMemo, useState } from "react";
import type { HexenProject } from "@hex-enductor/hexen-schema";
import type { OpenedProjectData } from "./storage/index.ts";

export interface LocationBrowserProps {
  project: HexenProject;
  resolvedContent: OpenedProjectData["resolvedContent"];
  currentLocationId: string | null;
  onSelect: (id: string) => void;
  onClose: () => void;
}

// The pin-by-pin hierarchy is the normal way through a project, but
// there's no way to jump straight to "that one shop three maps deep"
// without following it — this is every Location, flat, filterable,
// independent of which map you're currently standing on.
export function LocationBrowser({ project, resolvedContent, currentLocationId, onSelect, onClose }: LocationBrowserProps) {
  const [query, setQuery] = useState("");

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return project.locations
      .map((location) => ({ id: location.id, title: resolvedContent[location.id]?.title ?? location.id }))
      .filter((row) => !q || row.id.toLowerCase().includes(q) || row.title.toLowerCase().includes(q))
      .sort((a, b) => a.title.localeCompare(b.title));
  }, [project.locations, resolvedContent, query]);

  return (
    <div className="location-browser">
      <button type="button" className="link-button picker-close" onClick={onClose}>
        Cancel
      </button>
      <h3>
        All locations ({project.locations.length})
      </h3>
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search by name or id…"
        autoFocus
      />
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
              </button>
            </li>
          ))}
          {rows.length === 0 && <li className="muted">No locations match "{query}".</li>}
        </ul>
      </div>
    </div>
  );
}
