import { useState } from "react";
import { useAppStore } from "./store.ts";
import { trpc } from "./trpc.ts";
import { getRecentProjects } from "./recentProjects.ts";

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
  const openProject = useAppStore((s) => s.openProject);
  const [recent] = useState(() => getRecentProjects());
  const createProject = trpc.createProject.useMutation();

  function openAndClose(path: string) {
    openProject(path);
    onClose?.();
  }

  return (
    <div className="project-picker">
      {onClose && (
        <button type="button" className="link-button picker-close" onClick={onClose}>
          Cancel
        </button>
      )}

      <p>Open a .hexen.yml project by its absolute path.</p>
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
          autoFocus
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
          <h3>Recent</h3>
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
