// Same idea as apps/editor/src/recentProjects.ts, but a presentation
// window's URL carries both pieces (?server=&path=) since it has no
// fixed default server the way the editor does — so a "recent" entry
// here is the pair, not just a path.
const STORAGE_KEY = "hex-enductor:recent-targets";
const MAX_RECENT = 8;

export interface RecentTarget {
  server: string;
  path: string;
}

function isRecentTarget(value: unknown): value is RecentTarget {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as RecentTarget).server === "string" &&
    typeof (value as RecentTarget).path === "string"
  );
}

export function getRecentTargets(): RecentTarget[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isRecentTarget) : [];
  } catch {
    return [];
  }
}

export function addRecentTarget(target: RecentTarget): void {
  try {
    const deduped = getRecentTargets().filter((t) => !(t.server === target.server && t.path === target.path));
    const updated = [target, ...deduped].slice(0, MAX_RECENT);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch {
    // Private browsing / storage disabled — the recent list is a convenience, not a requirement.
  }
}
