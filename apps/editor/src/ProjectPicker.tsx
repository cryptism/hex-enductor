import { useEffect, useState } from "react";
import { useAppStore } from "./store.ts";
import { trpc } from "./trpc.ts";
import { getRecentProjects } from "./recentProjects.ts";
import { addLocalRecent, getLocalRecents, type LocalRecentEntry } from "./localRecents.ts";
import { createLocalFsStorage, supportsLocalFs } from "./storage/index.ts";

function joinPath(dir: string, name: string): string {
  return dir.endsWith("/") ? `${dir}${name}` : `${dir}/${name}`;
}

function slugify(title: string): string {
  return title.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "location";
}

function DirectoryBrowser({ onOpen }: { onOpen: (path: string) => void }) {
  const [browsePath, setBrowsePath] = useState<string | undefined>(undefined);
  const listing = trpc.listDirectory.useQuery({ path: browsePath });

  return (
    <div className="browser">
      {listing.isLoading && <p className="status">Loading…</p>}
      {listing.isError && <p className="status error">{listing.error.message}</p>}
      {listing.data && (
        <>
          <div className="browser-path">{listing.data.path}</div>
          <ul className="browser-list">
            {listing.data.parent !== null && (
              <li>
                <button onClick={() => setBrowsePath(listing.data!.parent!)}>.. (up)</button>
              </li>
            )}
            {listing.data.entries.map((entry) => (
              <li key={entry.name}>
                <button
                  onClick={() => {
                    const full = joinPath(listing.data!.path, entry.name);
                    if (entry.isDirectory) setBrowsePath(full);
                    else onOpen(full);
                  }}
                >
                  {entry.isDirectory ? `${entry.name}/` : entry.name}
                </button>
              </li>
            ))}
            {listing.data.entries.length === 0 && <li className="muted">Nothing to open here.</li>}
          </ul>
        </>
      )}
    </div>
  );
}

function NewProjectForm({
  onCreate,
  saving,
  error,
}: {
  onCreate: (values: { path: string; title: string }) => void;
  saving: boolean;
  error?: string;
}) {
  const [title, setTitle] = useState("");
  const [path, setPath] = useState("");

  const canCreate = title.trim().length > 0 && path.trim().length > 0;

  return (
    <form
      className="link-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (canCreate) onCreate({ title: title.trim(), path: path.trim() });
      }}
    >
      <label>
        Title
        <input type="text" value={title} onChange={(e) => setTitle(e.target.value)} placeholder="My Realm" autoFocus />
      </label>
      <label>
        Save as
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/path/to/my-realm.hexen.yml"
        />
      </label>
      {error && <span className="field-error">{error}</span>}
      <button type="submit" disabled={!canCreate || saving}>
        {saving ? "Creating…" : "Create"}
      </button>
    </form>
  );
}

export interface ProjectPickerProps {
  /** Passed when mounted as an in-editor overlay rather than the full-page landing state — closes itself once a project opens. */
  onClose?: () => void;
}

// The landing screen's guts, extracted so the exact same open/browse/
// create/recent surface can also open as a panel from inside the
// editor — same component either way, per the toolbox review's "file
// operations are the landing screen, reused" recommendation.
export function ProjectPicker({ onClose }: ProjectPickerProps) {
  const [pathInput, setPathInput] = useState("");
  const [browsing, setBrowsing] = useState(false);
  const [creating, setCreating] = useState(false);
  const openServerProject = useAppStore((s) => s.openServerProject);
  const setStorage = useAppStore((s) => s.setStorage);
  const [recent] = useState(() => getRecentProjects());
  const [localRecent, setLocalRecent] = useState<LocalRecentEntry[]>([]);
  const [localError, setLocalError] = useState<string | undefined>(undefined);
  const createProject = trpc.createProject.useMutation();
  const localFsSupported = supportsLocalFs();

  useEffect(() => {
    if (localFsSupported) getLocalRecents().then(setLocalRecent);
  }, [localFsSupported]);

  function openAndClose(path: string) {
    openServerProject(path);
    onClose?.();
  }

  async function openFromBrowser() {
    setLocalError(undefined);
    try {
      const handle = await window.showDirectoryPicker();
      setStorage(createLocalFsStorage(handle));
      await addLocalRecent(handle);
      onClose?.();
    } catch (err) {
      // Closing the picker without choosing anything isn't an error.
      if (err instanceof Error && err.name === "AbortError") return;
      setLocalError(err instanceof Error ? err.message : String(err));
    }
  }

  async function openLocalRecent(entry: LocalRecentEntry) {
    setLocalError(undefined);
    try {
      const permission = await entry.handle.requestPermission({ mode: "readwrite" });
      if (permission !== "granted") {
        setLocalError(`Permission to "${entry.name}" was denied.`);
        return;
      }
      setStorage(createLocalFsStorage(entry.handle));
      await addLocalRecent(entry.handle);
      onClose?.();
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="project-picker">
      {onClose && (
        <button type="button" className="link-button picker-close" onClick={onClose}>
          Cancel
        </button>
      )}

      {localFsSupported ? (
        <button type="button" className="tool-button" onClick={openFromBrowser}>
          Open from this browser…
        </button>
      ) : (
        <p className="muted">
          This browser can't open a project folder directly (Chrome and Edge can) — use a locally-running server
          instead, below.
        </p>
      )}
      {localError && <span className="field-error">{localError}</span>}

      {localRecent.length > 0 && (
        <div className="recent-projects">
          <h3>Recent (this browser)</h3>
          <ul>
            {localRecent.map((entry) => (
              <li key={entry.name}>
                <button className="link-button" onClick={() => openLocalRecent(entry)}>
                  {entry.name}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      <p>Or, open a .hexen.yml project on a locally-running server, by its absolute path.</p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (pathInput.trim()) openAndClose(pathInput.trim());
        }}
      >
        <input
          type="text"
          value={pathInput}
          onChange={(e) => setPathInput(e.target.value)}
          placeholder="/path/to/project.hexen.yml"
        />
        <button type="submit">Open</button>
      </form>

      <button className="link-button" onClick={() => setBrowsing((b) => !b)}>
        {browsing ? "Hide browser" : "Browse for a project…"}
      </button>
      {browsing && <DirectoryBrowser onOpen={openAndClose} />}

      <button className="link-button" onClick={() => setCreating((c) => !c)}>
        {creating ? "Cancel new project" : "New project…"}
      </button>
      {creating && (
        <NewProjectForm
          saving={createProject.isPending}
          error={createProject.error?.message}
          onCreate={(values) =>
            createProject.mutate(
              { path: values.path, title: values.title, defaultLocationId: slugify(values.title) },
              { onSuccess: () => openAndClose(values.path) },
            )
          }
        />
      )}

      {recent.length > 0 && (
        <div className="recent-projects">
          <h3>Recent (server)</h3>
          <ul>
            {recent.map((path) => (
              <li key={path}>
                <button className="link-button" onClick={() => openAndClose(path)}>
                  {path}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
