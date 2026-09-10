import { describe, expect, test } from "bun:test";
import { useAppStore } from "./store.ts";

describe("useAppStore", () => {
  test("opening a project resets location/link selection", () => {
    useAppStore.getState().setCurrentLocation("somewhere");
    useAppStore.getState().selectLink("something");

    useAppStore.getState().openProject("/tmp/test.hexen.yml");

    const state = useAppStore.getState();
    expect(state.projectPath).toBe("/tmp/test.hexen.yml");
    expect(state.currentLocationId).toBeNull();
    expect(state.selectedLinkId).toBeNull();
  });

  test("switching location clears link selection but not the location itself", () => {
    useAppStore.getState().selectLink("inn");
    useAppStore.getState().setCurrentLocation("town");

    const state = useAppStore.getState();
    expect(state.currentLocationId).toBe("town");
    expect(state.selectedLinkId).toBeNull();
  });

  test("starts in view mode, off by default", () => {
    expect(useAppStore.getState().editMode).toBe(false);
  });

  test("opening a project drops back to view mode", () => {
    useAppStore.getState().setEditMode(true);
    useAppStore.getState().openProject("/tmp/test.hexen.yml");

    expect(useAppStore.getState().editMode).toBe(false);
  });

  test("switching edit mode clears link selection", () => {
    useAppStore.getState().selectLink("inn");
    useAppStore.getState().setEditMode(true);

    expect(useAppStore.getState().selectedLinkId).toBeNull();
    expect(useAppStore.getState().editMode).toBe(true);
  });

  test("grid is visible by default, and survives opening a different project", () => {
    expect(useAppStore.getState().gridVisible).toBe(true);

    useAppStore.getState().setGridVisible(false);
    useAppStore.getState().openProject("/tmp/test.hexen.yml");

    expect(useAppStore.getState().gridVisible).toBe(false);
  });

  test("leaving edit mode exits the Add Location tool", () => {
    useAppStore.getState().setEditMode(true);
    useAppStore.getState().setPlacingLocation(true);
    useAppStore.getState().setEditMode(false);

    expect(useAppStore.getState().placingLocation).toBe(false);
  });

  test("switching location exits the Add Location tool", () => {
    useAppStore.getState().setEditMode(true);
    useAppStore.getState().setPlacingLocation(true);
    useAppStore.getState().setCurrentLocation("inn");

    expect(useAppStore.getState().placingLocation).toBe(false);
  });

  test("arming the Add Location tool clears link selection", () => {
    useAppStore.getState().selectLink("inn");
    useAppStore.getState().setPlacingLocation(true);

    expect(useAppStore.getState().selectedLinkId).toBeNull();
    expect(useAppStore.getState().placingLocation).toBe(true);
  });
});
