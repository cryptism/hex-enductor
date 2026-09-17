import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseHexenProject } from "@hex-enductor/hexen-schema";
import { app, websocket } from "./server.ts";

// End-to-end over a real socket, not the fake ones session.test.ts uses —
// this is what actually proves hexend is authoritative and broadcasts to
// every connected client, not just this one client's own request/response.

let projectDir: string;
let server: ReturnType<typeof Bun.serve>;

const YAML = `
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: inline }
locations:
  - id: town
    content: { type: inline, title: The Town, body: "" }
    links:
      - id: front-door
        target: inn
        x: 1
        y: 1
        type: settlement
        color: null
        hidden: false
  - id: inn
    content: { type: inline, title: The Inn, body: "" }
`;

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "ws-test-"));
  server = Bun.serve({ port: 0, fetch: app.fetch, websocket });
});

afterAll(async () => {
  server.stop(true);
  await rm(projectDir, { recursive: true, force: true });
});

function nextMessage(ws: WebSocket): Promise<any> {
  return new Promise((resolve) => {
    ws.addEventListener("message", (evt) => resolve(JSON.parse(evt.data as string)), { once: true });
  });
}

function connect(path: string): Promise<WebSocket> {
  const ws = new WebSocket(`ws://localhost:${server.port}/ws?path=${encodeURIComponent(path)}`);
  return new Promise((resolve) => ws.addEventListener("open", () => resolve(ws), { once: true }));
}

describe("GET /ws", () => {
  test("sends current state on connect, then broadcasts every command to every connected client", async () => {
    const path = join(projectDir, `${crypto.randomUUID()}.hexen.yml`);
    await writeFile(path, YAML, "utf-8");

    const clientA = await connect(path);
    const initialA = await nextMessage(clientA);
    expect(initialA.state).toBeTruthy();
    expect(initialA.state.project.title).toBe("Test Realm");

    const clientB = await connect(path);
    await nextMessage(clientB); // clientB's own initial state

    const [nextA, nextB] = await Promise.all([
      nextMessage(clientA),
      nextMessage(clientB),
      Promise.resolve(
        clientA.send(
          JSON.stringify({
            command: { saveLink: { locationId: "town", linkId: "front-door", patch: { hidden: true } } },
          }),
        ),
      ),
    ]);

    for (const msg of [nextA, nextB]) {
      expect(msg.state).toBeTruthy();
      expect(msg.state.project.locations[0].links[0].hidden).toBe(true);
    }

    await new Promise((r) => setTimeout(r, 20));
    const onDisk = parseHexenProject(await readFile(path, "utf-8")).project;
    expect(onDisk.locations[0]!.links[0]!.hidden).toBe(true);

    clientA.close();
    clientB.close();
  });

  test("closes with 1008 when no path is given", async () => {
    const ws = new WebSocket(`ws://localhost:${server.port}/ws`);
    const code = await new Promise<number>((resolve) => {
      ws.addEventListener("close", (evt) => resolve(evt.code));
    });
    expect(code).toBe(1008);
  });
});
