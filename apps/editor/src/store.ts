import { create } from "zustand";
import { addRecentProject } from "./recentProjects.ts";

// Client/UI state only — the project itself (locations, resolved
// content) lives in react-query's cache via trpc.openProject, not here.
interface AppState {
  projectPath: string | null;
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
  openProject: (path: string) => void;
  setCurrentLocation: (id: string | null) => void;
  selectLink: (id: string | null) => void;
  setEditMode: (editMode: boolean) => void;
  setGridVisible: (gridVisible: boolean) => void;
  setPlacingLocation: (placingLocation: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  projectPath: null,
  currentLocationId: null,
  selectedLinkId: null,
  editMode: false,
  gridVisible: true,
  placingLocation: false,
  openProject: (path) => {
    addRecentProject(path);
    set({
      projectPath: path,
      currentLocationId: null,
      selectedLinkId: null,
      editMode: false,
      placingLocation: false,
    });
  },
  setCurrentLocation: (id) => set({ currentLocationId: id, selectedLinkId: null, placingLocation: false }),
  selectLink: (id) => set({ selectedLinkId: id }),
  setEditMode: (editMode) => set({ editMode, selectedLinkId: null, placingLocation: false }),
  setGridVisible: (gridVisible) => set({ gridVisible }),
  setPlacingLocation: (placingLocation) => set({ placingLocation, selectedLinkId: null }),
}));
