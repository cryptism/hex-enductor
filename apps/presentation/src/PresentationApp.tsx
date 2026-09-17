import { useEffect, useMemo, useRef, useState } from "react";
import { MapCanvas, findLinkIcon } from "@hex-enductor/map-core";
import { connectLiveSession, type LiveSession, type OpenedProjectData } from "@hex-enductor/live-session";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

interface Target {
  server: string;
  path: string;
  /** &gm=1 turns on fog-of-war controls — meant for the GM's own window, never the projected/player one. */
  gm: boolean;
}

// This app never edits and never browses the filesystem — it's handed
// exactly which project to watch via its own URL query string, the
// same idea as an embed link:
// ?server=http://localhost:4000&path=/abs/project.hexen.yml
function targetFromUrl(): Target | null {
  const params = new URLSearchParams(window.location.search);
  const server = params.get("server");
  const path = params.get("path");
  return server && path ? { server, path, gm: params.get("gm") === "1" } : null;
}

// A rider on hexend's live session: it renders whatever the editor
// (or anyone else connected to the same project) does, live, the same
// way for every viewer. The one exception is fog of war — with
// ?gm=1, this window also calls execute() to start/clear fog and
// toggle cells, since that control is meant to live right where the
// GM is already looking at the table, not in a separate editor UI.
export function PresentationApp() {
  const target = useMemo(targetFromUrl, []);
  const [data, setData] = useState<OpenedProjectData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [currentLocationId, setCurrentLocationId] = useState<string | null>(null);
  const [selectedLinkId, setSelectedLinkId] = useState<string | null>(null);
  const sessionRef = useRef<LiveSession | null>(null);

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
        sessionRef.current = session;
        setData(session.initial);
        unsubscribe = session.subscribe(setData);
      })
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));

    return () => {
      cancelled = true;
      unsubscribe?.();
      sessionRef.current = null;
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

  function startFog() {
    sessionRef.current?.execute({ type: "setFog", locationId: currentLocation!.id, fog: { revealedCells: [] } });
  }

  function clearFog() {
    sessionRef.current?.execute({ type: "setFog", locationId: currentLocation!.id, fog: null });
  }

  function toggleFogCell(cell: string) {
    sessionRef.current?.execute({ type: "toggleFogCell", locationId: currentLocation!.id, cell });
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
        {target.gm && currentLocation.image && (
          <button type="button" className="link-button fog-toggle" onClick={currentLocation.fog ? clearFog : startFog}>
            {currentLocation.fog ? "Clear fog" : "Start fog"}
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
            fogEditable={target.gm}
            onToggleFogCell={toggleFogCell}
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
