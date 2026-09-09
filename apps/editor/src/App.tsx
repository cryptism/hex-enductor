import { useEffect, useState } from "react";
import { MapCanvas } from "@hex-enductor/map-core";
import { useAppStore } from "./store.ts";
import { trpc, serverUrl } from "./trpc.ts";
import { LinkForm } from "./LinkForm.tsx";
import type { Link } from "@hex-enductor/hexen-schema";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

function imageUrl(projectPath: string, file: string): string {
  const dir = dirname(projectPath);
  return `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&file=${encodeURIComponent(file)}`;
}

function OpenProjectForm() {
  const [pathInput, setPathInput] = useState("");
  const openProject = useAppStore((s) => s.openProject);

  return (
    <div className="open-project">
      <h1>hex-enductor</h1>
      <p>Open a .hexen.yml project — an absolute path, per docs/PLAN.md §9 (ad hoc, no project directory).</p>
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
    </div>
  );
}

function App() {
  const projectPath = useAppStore((s) => s.projectPath);
  const currentLocationId = useAppStore((s) => s.currentLocationId);
  const selectedLinkId = useAppStore((s) => s.selectedLinkId);
  const setCurrentLocation = useAppStore((s) => s.setCurrentLocation);
  const selectLink = useAppStore((s) => s.selectLink);

  const query = trpc.openProject.useQuery(
    { path: projectPath! },
    { enabled: projectPath !== null },
  );
  const utils = trpc.useUtils();
  const saveLink = trpc.saveLink.useMutation({
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
  if (query.isLoading) return <div className="status">Loading {projectPath}…</div>;
  if (query.isError) return <div className="status error">{query.error.message}</div>;
  if (!project) return null;

  const currentLocation = project.locations.find((l) => l.id === currentLocationId);
  if (!currentLocation) return <div className="status error">Unknown location "{currentLocationId}"</div>;

  const linkTitles = Object.fromEntries(
    currentLocation.links.map((link) => [
      link.id,
      query.data?.resolvedContent[link.id]?.title ?? link.id,
    ]),
  );

  const selectedLink = currentLocation.links.find((l) => l.id === selectedLinkId);
  // A selected link's own target location, if it has a grid of its own
  // — i.e. it's not just a pin, it's a map you can click into
  // (docs/ROADMAP.md: click-through to sub-maps).
  const selectedTargetLocation = selectedLink
    ? project.locations.find((l) => l.id === selectedLink.id)
    : undefined;

  const warnings = query.data?.warnings ?? [];
  const resolveErrors = query.data?.resolveErrors ?? {};

  return (
    <div className="app">
      <aside className="sidebar">
        <h2>{query.data?.resolvedContent[currentLocation.id]?.title ?? currentLocation.id}</h2>
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
          <MapCanvas
            image={currentLocation.image}
            imageUrl={imageUrl(projectPath, currentLocation.image.file)}
            grid={currentLocation.grid}
            links={currentLocation.links}
            linkTitles={linkTitles}
            selectedLinkId={selectedLinkId ?? undefined}
            onSelectLink={selectLink}
          />
        ) : (
          <div className="status">"{currentLocation.id}" has no grid/image — nothing to render.</div>
        )}
      </main>

      {selectedLink && (
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

export default App;
