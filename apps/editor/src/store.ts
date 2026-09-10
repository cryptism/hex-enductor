import { create } from "zustand";
import { addRecentProject } from "./recentProjects.ts";
import { createServerStorage, type ProjectStorage } from "./storage/index.ts";

// Client/UI state only — the project's own data (locations, resolved
// content) is fetched through `storage` and held by App itself, not
// here.
interface AppState {
  storage: ProjectStorage | null;
  currentLocationId: string | null;
  selectedLinkId: string | null;
  // Gates every mutating surface (link edits, location content edits,
  // and whatever tools land later) behind an explicit switch, off by
  // default — reading a map at the table shouldn't risk changing it.
  editMode: boolean;
  // A view toggle, not a safety gate — unlike editMode this carries
  // across projects/locations rather than resetting, same as flipping
  // "show rulers" would.
  gridVisible: boolean;
  // The Add Location tool's armed state — a click on the map places a
  // pin while this is true. Exited whenever the surrounding context
  // changes (mode, location, project) so it never survives a navigation.
  placingLocation: boolean;
  /** The server-backed path — open remains keyed to an absolute path string, same as always. */
  openServerProject: (path: string) => void;
  /** Any other backend (today: the File System Access API one) — no path string to remember. */
  setStorage: (storage: ProjectStorage) => void;
  closeProject: () => void;
  setCurrentLocation: (id: string | null) => void;
  selectLink: (id: string | null) => void;
  setEditMode: (editMode: boolean) => void;
  setGridVisible: (gridVisible: boolean) => void;
  setPlacingLocation: (placingLocation: boolean) => void;
}

function reset(storage: ProjectStorage) {
  return {
    storage,
    currentLocationId: null,
    selectedLinkId: null,
    editMode: false,
    placingLocation: false,
  };
}

export const useAppStore = create<AppState>((set) => ({
  storage: null,
  currentLocationId: null,
  selectedLinkId: null,
  editMode: false,
  gridVisible: true,
  placingLocation: false,
  openServerProject: (path) => {
    addRecentProject(path);
    set(reset(createServerStorage(path)));
  },
  setStorage: (storage) => set(reset(storage)),
  closeProject: () => set({ storage: null, currentLocationId: null, selectedLinkId: null }),
  setCurrentLocation: (id) => set({ currentLocationId: id, selectedLinkId: null, placingLocation: false }),
  selectLink: (id) => set({ selectedLinkId: id }),
  setEditMode: (editMode) => set({ editMode, selectedLinkId: null, placingLocation: false }),
  setGridVisible: (gridVisible) => set({ gridVisible }),
  setPlacingLocation: (placingLocation) => set({ placingLocation, selectedLinkId: null }),
}));
