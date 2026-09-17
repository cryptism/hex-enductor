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
  // A second, independent switch for fog-of-war: with gmMode alone you
  // get a read-only "layers panel" view of the fog (see App.tsx) and a
  // status readout of what players currently see; combined with
  // editMode you also get the fog paint tool and blanket apply/remove.
  // Independent of editMode on purpose — during live play a GM often
  // wants map authoring locked while still checking/controlling fog.
  gmMode: boolean;
  // The fog paint tool's armed state, same idea as placingLocation —
  // only meaningful with both editMode and gmMode on.
  paintingFog: boolean;
  // The Ping tool's armed state — gmMode alone, not editMode, since a
  // ping never touches project state. Unlike the other tools this one
  // doesn't auto-disarm after a single use — more a laser-pointer mode
  // you click on and off than a one-shot placement.
  pinging: boolean;
  // Another gmMode-alone sub-mode: while on, every pan/zoom on this
  // window's own map broadcasts as a followView, and the presentation
  // view drives its own map to match. Doesn't hijack clicks like the
  // other tools (panning/zooming already just works), so it survives
  // switching location — "until switched off" per its own name, not
  // reset by navigation the way the click-tools are.
  followMode: boolean;
  // A view toggle, not a safety gate — unlike editMode/gmMode this
  // carries across projects/locations rather than resetting, same as
  // flipping "show rulers" would.
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
  setGmMode: (gmMode: boolean) => void;
  setPaintingFog: (paintingFog: boolean) => void;
  setPinging: (pinging: boolean) => void;
  setFollowMode: (followMode: boolean) => void;
  setGridVisible: (gridVisible: boolean) => void;
  setPlacingLocation: (placingLocation: boolean) => void;
}

function reset(storage: ProjectStorage) {
  return {
    storage,
    currentLocationId: null,
    selectedLinkId: null,
    editMode: false,
    gmMode: false,
    paintingFog: false,
    pinging: false,
    followMode: false,
    placingLocation: false,
  };
}

export const useAppStore = create<AppState>((set) => ({
  storage: null,
  currentLocationId: null,
  selectedLinkId: null,
  editMode: false,
  gmMode: false,
  paintingFog: false,
  pinging: false,
  followMode: false,
  gridVisible: true,
  placingLocation: false,
  openServerProject: (path) => {
    addRecentProject(path);
    set(reset(createServerStorage(path)));
  },
  setStorage: (storage) => set(reset(storage)),
  closeProject: () => set({ storage: null, currentLocationId: null, selectedLinkId: null }),
  setCurrentLocation: (id) =>
    set({ currentLocationId: id, selectedLinkId: null, placingLocation: false, paintingFog: false, pinging: false }),
  selectLink: (id) => set({ selectedLinkId: id }),
  setEditMode: (editMode) => set({ editMode, selectedLinkId: null, placingLocation: false, paintingFog: false }),
  setGmMode: (gmMode) => set({ gmMode, paintingFog: false, pinging: false, followMode: false }),
  setPaintingFog: (paintingFog) => set({ paintingFog, selectedLinkId: null, pinging: false }),
  setPinging: (pinging) => set({ pinging, selectedLinkId: null, placingLocation: false, paintingFog: false }),
  setFollowMode: (followMode) => set({ followMode }),
  setGridVisible: (gridVisible) => set({ gridVisible }),
  setPlacingLocation: (placingLocation) => set({ placingLocation, selectedLinkId: null, pinging: false }),
}));
