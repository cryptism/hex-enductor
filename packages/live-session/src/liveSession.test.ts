import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { connectLiveSession } from "./liveSession.ts";

// A minimal stand-in for hexend-rs's /ws session, speaking its actual
// wire format (protobuf JSON mapping — see wireFormat.ts): sends an
// initial "state" on connect, then echoes whatever it receives back
// as another "state" so this suite can exercise connectLiveSession's
// protocol handling — handshake, subscribe fan-out, and
// execute/undo/redo wire format — without spinning up all of
// hexend-rs.
let server: ReturnType<typeof Bun.serve>;

const MINIMAL_WIRE_PROJECT = {
  schemaVersion: 1,
  title: "Test Realm",
  defaultLocation: "town",
  content: { inline: {} },
  locations: [{ id: "town" }],
};

beforeAll(() => {
  server = Bun.serve({
    port: 0,
    fetch(req, srv) {
      if (srv.upgrade(req, { data: undefined })) return undefined;
      return new Response("upgrade failed", { status: 500 });
    },
    websocket: {
      open(ws) {
        ws.send(JSON.stringify({ state: { project: MINIMAL_WIRE_PROJECT } }));
      },
      message(ws, message) {
        ws.send(
          JSON.stringify({
            state: { project: MINIMAL_WIRE_PROJECT, warnings: [JSON.stringify(JSON.parse(String(message)))] },
          }),
        );
      },
    },
  });
});

afterAll(() => {
  server.stop(true);
});

function serverUrl(): string {
  return `http://localhost:${server.port}`;
}

describe("connectLiveSession", () => {
  test("resolves with the first state message as `initial`, translated from the wire shape", async () => {
    const session = await connectLiveSession(serverUrl(), "/some/project.hexen.yml");
    expect(session.initial.project).toEqual({
      schemaVersion: 1,
      title: "Test Realm",
      defaultLocation: "town",
      content: { type: "inline" },
      locations: [{ id: "town", grid: null, image: null, content: null, links: [], fog: null }],
    });
    session.close();
  });

  test("subscribe receives every state message after the initial one", async () => {
    const session = await connectLiveSession(serverUrl(), "/some/project.hexen.yml");
    const received: string[] = [];
    const unsubscribe = session.subscribe((data) => received.push(data.warnings[0]!));

    session.execute({ type: "saveGrid", locationId: "town", grid: null });
    await new Promise((r) => setTimeout(r, 20));

    expect(received).toEqual([JSON.stringify({ command: { saveGrid: { locationId: "town", grid: null } } })]);

    unsubscribe();
    session.execute({ type: "saveGrid", locationId: "town", grid: null });
    await new Promise((r) => setTimeout(r, 20));
    expect(received).toHaveLength(1); // unsubscribed — no second entry

    session.close();
  });

  test("undo and redo send their own message shape", async () => {
    const session = await connectLiveSession(serverUrl(), "/some/project.hexen.yml");
    const received: string[] = [];
    session.subscribe((data) => received.push(data.warnings[0]!));

    session.undo();
    session.redo();
    await new Promise((r) => setTimeout(r, 20));

    expect(received).toEqual([JSON.stringify({ undo: {} }), JSON.stringify({ redo: {} })]);
    session.close();
  });
});
