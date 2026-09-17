const SERVER_URL = import.meta.env.VITE_SERVER_URL ?? "http://localhost:4000";

// A "?server=..." query param overrides the default, the same idea as
// apps/presentation's own "?server=&path=" — lets a launch script point
// the editor at a hexend instance running on a non-default port.
export function serverUrl(): string {
  return new URLSearchParams(window.location.search).get("server") ?? SERVER_URL;
}
