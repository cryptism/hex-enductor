import { createTRPCReact } from "@trpc/react-query";
import { httpBatchLink } from "@trpc/client";
import type { AppRouter } from "@hex-enductor/hexend/src/router.ts";

export const trpc = createTRPCReact<AppRouter>();

const SERVER_URL = import.meta.env.VITE_SERVER_URL ?? "http://localhost:4000";

export const trpcClient = trpc.createClient({
  links: [httpBatchLink({ url: `${SERVER_URL}/trpc` })],
});

export function serverUrl(): string {
  return SERVER_URL;
}
