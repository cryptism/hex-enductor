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
});
