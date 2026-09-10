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
  openProject: (path: string) => void;
  setCurrentLocation: (id: string | null) => void;
  selectLink: (id: string | null) => void;
  setEditMode: (editMode: boolean) => void;
  setGridVisible: (gridVisible: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  projectPath: null,
  currentLocationId: null,
  selectedLinkId: null,
  editMode: false,
  gridVisible: true,
  openProject: (path) => {
    addRecentProject(path);
    set({ projectPath: path, currentLocationId: null, selectedLinkId: null, editMode: false });
  },
  setCurrentLocation: (id) => set({ currentLocationId: id, selectedLinkId: null }),
  selectLink: (id) => set({ selectedLinkId: id }),
  setEditMode: (editMode) => set({ editMode, selectedLinkId: null }),
  setGridVisible: (gridVisible) => set({ gridVisible }),
}));
