import { beforeEach, describe, expect, test } from "bun:test";
import { addRecentProject, getRecentProjects } from "./recentProjects.ts";

// bun:test runs outside a browser, so there's no real localStorage —
// stand in with the minimal subset recentProjects.ts actually calls.
class MemoryStorage {
  #data = new Map<string, string>();
  getItem(key: string): string | null {
    return this.#data.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.#data.set(key, value);
  }
  clear(): void {
    this.#data.clear();
  }
}

(globalThis as unknown as { localStorage: MemoryStorage }).localStorage = new MemoryStorage();

beforeEach(() => {
  localStorage.clear();
});

describe("recentProjects", () => {
  test("starts empty", () => {
    expect(getRecentProjects()).toEqual([]);
  });

  test("adds a project to the front of the list", () => {
    addRecentProject("/a.hexen.yml");
    addRecentProject("/b.hexen.yml");
    expect(getRecentProjects()).toEqual(["/b.hexen.yml", "/a.hexen.yml"]);
  });

  test("re-adding an existing path moves it to the front instead of duplicating it", () => {
    addRecentProject("/a.hexen.yml");
    addRecentProject("/b.hexen.yml");
    addRecentProject("/a.hexen.yml");
    expect(getRecentProjects()).toEqual(["/a.hexen.yml", "/b.hexen.yml"]);
  });

  test("caps the list at 8 entries", () => {
    for (let i = 0; i < 10; i++) addRecentProject(`/p${i}.hexen.yml`);
    const recent = getRecentProjects();
    expect(recent).toHaveLength(8);
    expect(recent[0]).toBe("/p9.hexen.yml");
  });
});
