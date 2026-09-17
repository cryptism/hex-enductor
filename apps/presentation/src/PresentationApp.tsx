import { useEffect, useMemo, useState } from "react";
import { MapCanvas, findLinkIcon } from "@hex-enductor/map-core";
import { connectLiveSession, type OpenedProjectData } from "@hex-enductor/live-session";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

interface Target {
  server: string;
  path: string;
}

// This app never edits and never browses the filesystem — it's handed
// exactly which project to watch via its own URL query string, the
// same idea as an embed link:
// ?server=http://localhost:4000&path=/abs/project.hexen.yml
function targetFromUrl(): Target | null {
  const params = new URLSearchParams(window.location.search);
  const server = params.get("server");
  const path = params.get("path");
  return server && path ? { server, path } : null;
}

// A read-only rider on hexend's live session: it never calls
// execute/undo/redo, only subscribe — so it renders whatever the
// editor (or anyone else connected to the same project) does, live,
// fog of war included, with no separate "read-only mode" to keep in
// sync elsewhere. Fog is applied/removed from the editor's own GM
// mode (see apps/editor), not here — this window just shows the
// result, same as it would any other command.
export function PresentationApp() {
  const target = useMemo(targetFromUrl, []);
  const [data, setData] = useState<OpenedProjectData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentLocationId, setCurrentLocationId] = useState<string | null>(null);
  const [selectedLinkId, setSelectedLinkId] = useState<string | null>(null);

  useEffect(() => {
    if (!target) return;
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;

    connectLiveSession(target.server, target.path)
      .then((session) => {
        if (cancelled) {
          session.close();
          return;
        }
        setData(session.initial);
        unsubscribe = session.subscribe(setData);
      })
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [target]);

  const project = data?.project;

  useEffect(() => {
    if (project && currentLocationId === null) setCurrentLocationId(project.defaultLocation);
  }, [project, currentLocationId]);

  if (!target) {
    return (
      <div className="status">
        Add <code>?server=http://localhost:4000&amp;path=/abs/project.hexen.yml</code> to the URL.
      </div>
    );
  }
  if (error) return <div className="status error">{error}</div>;
  if (!project || !data) return <div className="status">Connecting…</div>;

  const currentLocation = project.locations.find((l) => l.id === currentLocationId);
  if (!currentLocation) return <div className="status error">Unknown location "{currentLocationId}"</div>;

  const linkTitles = Object.fromEntries(
    currentLocation.links.map((link) => [link.target, data.resolvedContent[link.target]?.title ?? link.target]),
  );
  const locationTitle = data.resolvedContent[currentLocation.id]?.title ?? currentLocation.id;
  const imageUrl = currentLocation.image
    ? `${target.server}/image?dir=${encodeURIComponent(dirname(target.path))}&file=${encodeURIComponent(currentLocation.image.file)}`
    : null;

  function goToLink(linkId: string) {
    setSelectedLinkId(linkId);
    const link = currentLocation!.links.find((l) => l.id === linkId);
    const targetLocation = link && project!.locations.find((l) => l.id === link.target);
    if (targetLocation?.image) setCurrentLocationId(targetLocation.id);
  }

  return (
    <div className="presentation">
      <header className="presentation-header">
        <h2>{locationTitle}</h2>
        {currentLocation.id !== project.defaultLocation && (
          <button type="button" className="link-button" onClick={() => setCurrentLocationId(project.defaultLocation)}>
            ← {project.title}
          </button>
        )}
      </header>

      <main className="presentation-map">
        {currentLocation.image && imageUrl ? (
          <MapCanvas
            image={currentLocation.image}
            imageUrl={imageUrl}
            grid={currentLocation.grid}
            links={currentLocation.links}
            linkTitles={linkTitles}
            selectedLinkId={selectedLinkId ?? undefined}
            onSelectLink={goToLink}
            fog={currentLocation.fog}
          />
        ) : (
          <div className="status">"{currentLocation.id}" has no image — nothing to render.</div>
        )}
      </main>

      {currentLocation.links.some((l) => !l.hidden) && (
        <nav className="presentation-links">
          <ul>
            {currentLocation.links
              .filter((link) => !link.hidden)
              .map((link) => {
                const icon = findLinkIcon(link.icon);
                return (
                  <li key={link.id}>
                    <button
                      type="button"
                      className={link.id === selectedLinkId ? "selected" : ""}
                      onClick={() => goToLink(link.id)}
                    >
                      {icon && <span className="icon-swatch" dangerouslySetInnerHTML={{ __html: icon.svg }} />}
                      {linkTitles[link.target]}
                    </button>
                  </li>
                );
              })}
          </ul>
        </nav>
      )}
    </div>
  );
}
