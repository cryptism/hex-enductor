import { useEffect, useState, type ReactNode } from "react";
import { MapCanvas } from "@hex-enductor/map-core";
import { useAppStore } from "./store.ts";
import { trpc, serverUrl } from "./trpc.ts";
import { LinkForm } from "./LinkForm.tsx";
import { LocationContentForm } from "./LocationContentForm.tsx";
import { getRecentProjects } from "./recentProjects.ts";
import { BrandMark } from "./Logo.tsx";
import { LoadingScreen } from "./LoadingScreen.tsx";
import type { Link } from "@hex-enductor/hexen-schema";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

function imageUrl(projectPath: string, file: string): string {
  const dir = dirname(projectPath);
  return `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&file=${encodeURIComponent(file)}`;
}

function joinPath(dir: string, name: string): string {
  return dir.endsWith("/") ? `${dir}${name}` : `${dir}/${name}`;
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

function OpenProjectForm() {
  const [pathInput, setPathInput] = useState("");
  const [browsing, setBrowsing] = useState(false);
  const openProject = useAppStore((s) => s.openProject);
  const [recent] = useState(() => getRecentProjects());

  return (
    <div className="open-project">
      <h1 className="brand-heading">
        <BrandMark size={64} />
      </h1>
      <p>Open a .hexen.yml project by its absolute path.</p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (pathInput.trim()) openProject(pathInput.trim());
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
      {browsing && <DirectoryBrowser onOpen={openProject} />}

      {recent.length > 0 && (
        <div className="recent-projects">
          <h3>Recent</h3>
          <ul>
            {recent.map((path) => (
              <li key={path}>
                <button className="link-button" onClick={() => openProject(path)}>
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

function App() {
  const projectPath = useAppStore((s) => s.projectPath);
  const currentLocationId = useAppStore((s) => s.currentLocationId);
  const selectedLinkId = useAppStore((s) => s.selectedLinkId);
  const editMode = useAppStore((s) => s.editMode);
  const gridVisible = useAppStore((s) => s.gridVisible);
  const setCurrentLocation = useAppStore((s) => s.setCurrentLocation);
  const selectLink = useAppStore((s) => s.selectLink);
  const setEditMode = useAppStore((s) => s.setEditMode);
  const setGridVisible = useAppStore((s) => s.setGridVisible);

  const query = trpc.openProject.useQuery(
    { path: projectPath! },
    { enabled: projectPath !== null },
  );
  const utils = trpc.useUtils();
  const saveLink = trpc.saveLink.useMutation({
    onSuccess: () => utils.openProject.invalidate({ path: projectPath! }),
  });
  const saveLocationContent = trpc.saveLocationContent.useMutation({
    onSuccess: () => utils.openProject.invalidate({ path: projectPath! }),
  });

  const project = query.data?.project;

  // Default to the project's defaultLocation once it loads.
  useEffect(() => {
    if (project && currentLocationId === null) {
      setCurrentLocation(project.defaultLocation);
    }
  }, [project, currentLocationId, setCurrentLocation]);

  if (projectPath === null) return <OpenProjectForm />;
  if (query.isError) return <div className="status error">{query.error.message}</div>;

  let mainContent: ReactNode = null;
  if (project) {
    const currentLocation = project.locations.find((l) => l.id === currentLocationId);
    if (!currentLocation) {
      mainContent = <div className="status error">Unknown location "{currentLocationId}"</div>;
    } else {
      const linkTitles = Object.fromEntries(
        currentLocation.links.map((link) => [
          link.id,
          query.data?.resolvedContent[link.id]?.title ?? link.id,
        ]),
      );

      const selectedLink = currentLocation.links.find((l) => l.id === selectedLinkId);
      // A selected link's own target location, if it has a grid of its own
      // — i.e. it's not just a pin, it's a map you can click into.
      const selectedTargetLocation = selectedLink
        ? project.locations.find((l) => l.id === selectedLink.id)
        : undefined;

      const warnings = query.data?.warnings ?? [];
      const resolveErrors = query.data?.resolveErrors ?? {};

      const locationTitle = query.data?.resolvedContent[currentLocation.id]?.title ?? "";
      const locationBody = query.data?.resolvedContent[currentLocation.id]?.body ?? "";
      const isInlineContent = currentLocation.content === null || currentLocation.content.type === "inline";

      mainContent = (
        <div className="app">
          <aside className="sidebar">
            <label className="mode-toggle">
              <input type="checkbox" checked={editMode} onChange={(e) => setEditMode(e.target.checked)} />
              <span className="mode-toggle-track" aria-hidden="true" />
              <span className="mode-toggle-label">{editMode ? "Editing" : "Viewing"}</span>
            </label>

            {editMode && isInlineContent ? (
              <LocationContentForm
                key={currentLocation.id}
                locationId={currentLocation.id}
                title={locationTitle}
                body={locationBody}
                saving={saveLocationContent.isPending}
                onSave={(patch) =>
                  saveLocationContent.mutate({
                    path: projectPath,
                    locationId: currentLocation.id,
                    patch,
                  })
                }
              />
            ) : (
              <div className="location-heading">
                <h2>{locationTitle || currentLocation.id}</h2>
                {isInlineContent && locationBody && <p className="location-body">{locationBody}</p>}
              </div>
            )}
            {currentLocation.id !== project.defaultLocation && (
              <button className="link-button" onClick={() => setCurrentLocation(project.defaultLocation)}>
                ← {project.title}
              </button>
            )}

            {(warnings.length > 0 || Object.keys(resolveErrors).length > 0) && (
              <details className="warnings">
                <summary>
                  {warnings.length + Object.keys(resolveErrors).length} warning(s)
                </summary>
                <ul>
                  {warnings.map((w, i) => (
                    <li key={`w${i}`}>{w}</li>
                  ))}
                  {Object.entries(resolveErrors).map(([id, err]) => (
                    <li key={id}>
                      {id}: {err}
                    </li>
                  ))}
                </ul>
              </details>
            )}

            <ul className="link-list">
              {currentLocation.links.map((link) => (
                <li key={link.id}>
                  <button
                    className={link.id === selectedLinkId ? "selected" : ""}
                    onClick={() => selectLink(link.id)}
                  >
                    {linkTitles[link.id]}
                    {link.hidden ? " (hidden)" : ""}
                  </button>
                </li>
              ))}
              {currentLocation.links.length === 0 && <li className="muted">No locations pinned here yet.</li>}
            </ul>
          </aside>

          <main className="map-area">
            {currentLocation.grid && currentLocation.image ? (
              <>
                <MapCanvas
                  image={currentLocation.image}
                  imageUrl={imageUrl(projectPath, currentLocation.image.file)}
                  grid={currentLocation.grid}
                  gridVisible={gridVisible}
                  links={currentLocation.links}
                  linkTitles={linkTitles}
                  selectedLinkId={selectedLinkId ?? undefined}
                  onSelectLink={selectLink}
                />
                <label className="grid-toggle">
                  <input
                    type="checkbox"
                    checked={gridVisible}
                    onChange={(e) => setGridVisible(e.target.checked)}
                  />
                  Show grid
                </label>
              </>
            ) : (
              <div className="status">"{currentLocation.id}" has no grid/image — nothing to render.</div>
            )}
          </main>

          {editMode && selectedLink && (
            <aside className="edit-panel">
              <LinkForm
                key={selectedLink.id}
                link={selectedLink}
                title={linkTitles[selectedLink.id] ?? selectedLink.id}
                saving={saveLink.isPending}
                onSave={(patch) =>
                  saveLink.mutate({
                    path: projectPath,
                    locationId: currentLocation.id,
                    linkId: selectedLink.id,
                    patch: patch as Partial<Link>,
                  })
                }
              />
              {selectedTargetLocation?.grid && (
                <button className="link-button" onClick={() => setCurrentLocation(selectedTargetLocation.id)}>
                  View map →
                </button>
              )}
            </aside>
          )}
        </div>
      );
    }
  }

  return (
    <>
      <LoadingScreen active={!project} />
      {mainContent}
    </>
  );
}

export default App;
