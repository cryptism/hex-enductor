const SERVER_URL = import.meta.env.VITE_SERVER_URL ?? "http://localhost:4000";

export function serverUrl(): string {
  return SERVER_URL;
}
