#!/usr/bin/env bun
// One command instead of the four-terminal-plus-manual-paste dance in
// CLAUDE.md's "Running things for manual/browser testing": starts
// hexend, the editor, and presentation, waits for each to come up, and
// opens a browser tab for the editor and one for presentation, both
// already pointed at the given project via query params — the
// editor's own "?path=" (and "?server="), see App.tsx/server.ts, and
// presentation's pre-existing "?server=&path=", see PresentationApp.tsx.
//
// Usage: bun run launch --project examples/demo/demo.hexen.yml
// Must run inside `nix develop` — same as every other hexend-touching
// command in this repo (protoc/buf/cargo aren't on PATH otherwise).
import { resolve } from "node:path";

function parseArgs(argv: string[]) {
  const args: Record<string, string> = {};
  const flags = new Set<string>();
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]!;
    if (!arg.startsWith("--")) throw new Error(`Expected a --flag, got "${arg}"`);
    const name = arg.slice(2);
    const value = argv[i + 1];
    if (value === undefined || value.startsWith("--")) {
      flags.add(name);
      continue;
    }
    args[name] = value;
    i++;
  }
  return { args, flags };
}

function usageError(message: string): never {
  console.error(message);
  console.error(
    "Usage: bun run launch --project <file.hexen.yml> [--hexend-port 4000] [--editor-port 5173] [--presentation-port 5174] [--no-browser]",
  );
  process.exit(1);
}

interface Service {
  name: string;
  proc: ReturnType<typeof Bun.spawn>;
}

const services: Service[] = [];
let shuttingDown = false;

function spawnService(name: string, cmd: string[], opts: { cwd?: string; env?: Record<string, string> } = {}): ReturnType<typeof Bun.spawn> {
  console.log(`$ ${cmd.join(" ")}${opts.cwd ? ` (in ${opts.cwd})` : ""}`);
  const proc = Bun.spawn(cmd, {
    cwd: opts.cwd,
    env: { ...process.env, ...opts.env },
    stdout: "inherit",
    stderr: "inherit",
  });
  services.push({ name, proc });
  return proc;
}

/** Polls until something answers on `url`, or throws after `timeoutMs`. Doesn't care about the response's status — a listening socket is all "up" means here. */
async function waitForHttp(name: string, url: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await fetch(url);
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  throw new Error(`Timed out waiting for ${name} at ${url}`);
}

function openBrowser(url: string): void {
  const cmd =
    process.platform === "darwin"
      ? ["open", url]
      : process.platform === "win32"
        ? ["cmd", "/c", "start", "", url]
        : ["xdg-open", url];
  try {
    Bun.spawn(cmd, { stdout: "ignore", stderr: "ignore" }).unref();
  } catch {
    console.warn(`Couldn't open a browser automatically — open this yourself:\n  ${url}`);
  }
}

function killAll(): void {
  if (shuttingDown) return;
  shuttingDown = true;
  for (const { proc } of services) {
    try {
      proc.kill();
    } catch {
      // already gone
    }
  }
}

async function main() {
  const { args, flags } = parseArgs(process.argv.slice(2));

  const projectArg = args["project"];
  if (!projectArg) usageError("Missing --project <file.hexen.yml>");
  const projectPath = resolve(projectArg);
  if (!(await Bun.file(projectPath).exists())) usageError(`No such file: ${projectPath}`);

  const hexendPort = Number(args["hexend-port"] ?? 4000);
  const editorPort = Number(args["editor-port"] ?? 5173);
  const presentationPort = Number(args["presentation-port"] ?? 5174);
  const openBrowsers = !flags.has("no-browser");

  process.on("SIGINT", () => {
    killAll();
    process.exit(0);
  });
  process.on("SIGTERM", () => {
    killAll();
    process.exit(0);
  });

  console.log(`Starting hexend on :${hexendPort}...`);
  spawnService("hexend", ["cargo", "run", "--manifest-path", "apps/hexend/Cargo.toml"], {
    env: { PORT: String(hexendPort) },
  });
  await waitForHttp("hexend", `http://localhost:${hexendPort}/`, 60_000);

  console.log(`Starting the editor on :${editorPort}...`);
  spawnService("editor", ["bun", "run", "--cwd", "apps/editor", "dev", "--", "--port", String(editorPort), "--strictPort"]);
  await waitForHttp("editor", `http://localhost:${editorPort}/`, 30_000);

  console.log(`Starting presentation on :${presentationPort}...`);
  spawnService("presentation", [
    "bun",
    "run",
    "--cwd",
    "apps/presentation",
    "dev",
    "--",
    "--port",
    String(presentationPort),
    "--strictPort",
  ]);
  await waitForHttp("presentation", `http://localhost:${presentationPort}/`, 30_000);

  const server = `http://localhost:${hexendPort}`;
  const editorUrl = `http://localhost:${editorPort}/?server=${encodeURIComponent(server)}&path=${encodeURIComponent(projectPath)}`;
  const presentationUrl = `http://localhost:${presentationPort}/?server=${encodeURIComponent(server)}&path=${encodeURIComponent(projectPath)}`;

  console.log("\nEverything's up:");
  console.log(`  editor:       ${editorUrl}`);
  console.log(`  presentation: ${presentationUrl}`);

  if (openBrowsers) {
    openBrowser(editorUrl);
    openBrowser(presentationUrl);
  }

  console.log("\nPress Ctrl+C to stop everything.");
  await Promise.race(services.map(({ proc }) => proc.exited));
  // One of them died on its own (a crash, or a port fight) — bring the rest down too rather than leaving orphans.
  killAll();
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err));
  killAll();
  process.exit(1);
});
