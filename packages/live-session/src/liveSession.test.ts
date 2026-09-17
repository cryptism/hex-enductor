import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { connectLiveSession } from "./liveSession.ts";

// A minimal stand-in for hexend's /ws session, speaking its actual
// wire format (protobuf JSON mapping — see wireFormat.ts): sends an
// initial "state" on connect, then echoes whatever it receives back
// as another "state" so this suite can exercise connectLiveSession's
// protocol handling — handshake, subscribe fan-out, and
// execute/undo/redo wire format — without spinning up all of hexend.
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
        const parsed = JSON.parse(String(message));
        if (parsed.ping) {
          ws.send(JSON.stringify({ ping: parsed.ping }));
          return;
        }
        if (parsed.followView) {
          ws.send(JSON.stringify({ followView: parsed.followView }));
          return;
        }
        ws.send(
          JSON.stringify({
            state: { project: MINIMAL_WIRE_PROJECT, warnings: [JSON.stringify(parsed)] },
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

  test("ping sends its own message shape and onPing receives the rebroadcast", async () => {
    const session = await connectLiveSession(serverUrl(), "/some/project.hexen.yml");
    const received: unknown[] = [];
    const unsubscribe = session.onPing((ping) => received.push(ping));

    session.ping("town", 12.5, 34.5);
    await new Promise((r) => setTimeout(r, 20));

    expect(received).toEqual([{ locationId: "town", x: 12.5, y: 34.5 }]);

    unsubscribe();
    session.ping("town", 1, 1);
    await new Promise((r) => setTimeout(r, 20));
    expect(received).toHaveLength(1); // unsubscribed — no second entry

    session.close();
  });

  test("followView sends its own message shape and onFollowView receives the rebroadcast", async () => {
    const session = await connectLiveSession(serverUrl(), "/some/project.hexen.yml");
    const received: unknown[] = [];
    const unsubscribe = session.onFollowView((view) => received.push(view));

    session.followView("town", 100, 200, 1.5);
    await new Promise((r) => setTimeout(r, 20));

    expect(received).toEqual([{ locationId: "town", x: 100, y: 200, zoom: 1.5 }]);

    unsubscribe();
    session.followView("town", 0, 0, 0);
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
