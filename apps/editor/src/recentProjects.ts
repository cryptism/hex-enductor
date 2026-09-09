const STORAGE_KEY = "hex-enductor:recent-projects";
const MAX_RECENT = 8;

export function getRecentProjects(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((p): p is string => typeof p === "string") : [];
  } catch {
    return [];
  }
}

export function addRecentProject(path: string): void {
  try {
    const deduped = getRecentProjects().filter((p) => p !== path);
    const updated = [path, ...deduped].slice(0, MAX_RECENT);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch {
    // Private browsing / storage disabled — the recent list is a convenience, not a requirement.
  }
}
