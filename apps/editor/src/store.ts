import { create } from "zustand";
import { addRecentProject } from "./recentProjects.ts";

// Client/UI state only — the project itself (locations, resolved
// content) lives in react-query's cache via trpc.openProject, not here.
interface AppState {
  projectPath: string | null;
  currentLocationId: string | null;
  selectedLinkId: string | null;
  openProject: (path: string) => void;
  setCurrentLocation: (id: string | null) => void;
  selectLink: (id: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  projectPath: null,
  currentLocationId: null,
  selectedLinkId: null,
  openProject: (path) => {
    addRecentProject(path);
    set({ projectPath: path, currentLocationId: null, selectedLinkId: null });
  },
  setCurrentLocation: (id) => set({ currentLocationId: id, selectedLinkId: null }),
  selectLink: (id) => set({ selectedLinkId: id }),
}));
