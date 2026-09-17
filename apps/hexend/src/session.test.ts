import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseHexenProject } from "@hex-enductor/hexen-schema";
import { applyAndBroadcast, getOrCreateSession, redo, undo, _resetSessionsForTest, type SessionSocket } from "./session.ts";

let projectDir: string;

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

async function writeProject(): Promise<string> {
  const path = join(projectDir, `${crypto.randomUUID()}.hexen.yml`);
  await writeFile(path, YAML, "utf-8");
  return path;
}

class FakeSocket implements SessionSocket {
  messages: unknown[] = [];
  send(data: string): void {
    this.messages.push(JSON.parse(data));
  }
}

beforeAll(async () => {
  projectDir = await mkdtemp(join(tmpdir(), "session-test-"));
});

afterAll(async () => {
  await rm(projectDir, { recursive: true, force: true });
});

afterEach(() => {
  _resetSessionsForTest();
});

describe("applyAndBroadcast", () => {
  test("applies a command, persists it to disk, and broadcasts to every connected socket", async () => {
    const path = await writeProject();
    const session = await getOrCreateSession(path);
    const a = new FakeSocket();
    const b = new FakeSocket();
    session.sockets.add(a);
    session.sockets.add(b);

    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { hidden: true } });

    expect(session.project.locations[0]!.links[0]!.hidden).toBe(true);
    for (const socket of [a, b]) {
      const [msg] = socket.messages as [{ state: { project: { locations: { links: { hidden: boolean }[] }[] } } }];
      expect(msg.state).toBeTruthy();
      expect(msg.state.project.locations[0]!.links[0]!.hidden).toBe(true);
    }

    // Persistence is fire-and-forget — give the disk write a turn to land.
    await new Promise((r) => setTimeout(r, 20));
    const onDisk = parseHexenProject(await readFile(path, "utf-8")).project;
    expect(onDisk.locations[0]!.links[0]!.hidden).toBe(true);
  });

  test("refreshes resolvedContent for an inline saveLocationContent command", async () => {
    const path = await writeProject();
    const session = await getOrCreateSession(path);
    applyAndBroadcast(session, { type: "saveLocationContent", locationId: "inn", patch: { body: "Warm and loud." } });
    expect(session.resolvedContent.inn).toEqual({ title: "The Inn", body: "Warm and loud." });
  });
});

describe("undo/redo", () => {
  test("undo reverts to the previous snapshot, redo reapplies it", async () => {
    const path = await writeProject();
    const session = await getOrCreateSession(path);

    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { hidden: true } });
    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { x: 9 } });
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: true, x: 9 });

    undo(session);
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: true, x: 1 });

    undo(session);
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: false, x: 1 });

    // Undo past the start is a no-op, not an error.
    undo(session);
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: false, x: 1 });

    redo(session);
    redo(session);
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: true, x: 9 });

    // Redo past the end is a no-op.
    redo(session);
    expect(session.project.locations[0]!.links[0]!).toMatchObject({ hidden: true, x: 9 });
  });

  test("a new command after an undo discards the redo branch", async () => {
    const path = await writeProject();
    const session = await getOrCreateSession(path);

    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { x: 2 } });
    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { x: 3 } });
    undo(session);
    applyAndBroadcast(session, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { x: 99 } });

    expect(session.project.locations[0]!.links[0]!.x).toBe(99);
    redo(session);
    expect(session.project.locations[0]!.links[0]!.x).toBe(99);
  });
});

describe("getOrCreateSession", () => {
  test("caches the session per path — a second call doesn't re-read the file", async () => {
    const path = await writeProject();
    const first = await getOrCreateSession(path);
    applyAndBroadcast(first, { type: "saveLink", locationId: "town", linkId: "front-door", patch: { hidden: true } });

    const second = await getOrCreateSession(path);
    expect(second).toBe(first);
    expect(second.project.locations[0]!.links[0]!.hidden).toBe(true);
  });
});
