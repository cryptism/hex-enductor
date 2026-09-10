import { useEffect, useState, type ReactNode } from "react";
import { MapCanvas, findLinkIcon } from "@hex-enductor/map-core";
import { useAppStore } from "./store.ts";
import { LinkForm } from "./LinkForm.tsx";
import { LocationContentForm } from "./LocationContentForm.tsx";
import { AddLocationForm } from "./AddLocationForm.tsx";
import { ConfigureGridForm } from "./ConfigureGridForm.tsx";
import { ImageUpload } from "./ImageUpload.tsx";
import { ProjectPicker } from "./ProjectPicker.tsx";
import { LocationBrowser } from "./LocationBrowser.tsx";
import { AboutModal } from "./AboutModal.tsx";
import { BrandMark, Logo } from "./Logo.tsx";
import { LoadingScreen } from "./LoadingScreen.tsx";
import type { OpenedProjectData } from "./storage/index.ts";
import type { Grid, Link, Point } from "@hex-enductor/hexen-schema";

function OpenProjectForm() {
  return (
    <div className="open-project">
      <h1 className="brand-heading">
        <BrandMark size={64} />
      </h1>
      <ProjectPicker />
    </div>
  );
}

function App() {
  const storage = useAppStore((s) => s.storage);
  const currentLocationId = useAppStore((s) => s.currentLocationId);
  const selectedLinkId = useAppStore((s) => s.selectedLinkId);
  const editMode = useAppStore((s) => s.editMode);
  const gridVisible = useAppStore((s) => s.gridVisible);
  const placingLocation = useAppStore((s) => s.placingLocation);
  const setCurrentLocation = useAppStore((s) => s.setCurrentLocation);
  const selectLink = useAppStore((s) => s.selectLink);
  const setEditMode = useAppStore((s) => s.setEditMode);
  const setGridVisible = useAppStore((s) => s.setGridVisible);
  const setPlacingLocation = useAppStore((s) => s.setPlacingLocation);

  // The active backend's data — no more react-query: every storage
  // method already hands back the freshly reopened project, so a
  // mutation's .then(setData) is the entire "refetch" step.
  const [data, setData] = useState<OpenedProjectData | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    setData(null);
    setLoadError(null);
    if (!storage) return;
    let cancelled = false;
    storage
      .open()
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((err) => {
        if (!cancelled) setLoadError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [storage]);

  // Where the Add Location tool's pending click landed, in image pixel
  // space — local, not store state, since it's meaningless the moment
  // placingLocation goes false (see the effect below).
  const [pendingPoint, setPendingPoint] = useState<Point | null>(null);
  useEffect(() => {
    if (!placingLocation) setPendingPoint(null);
  }, [placingLocation]);

  // Configure Grid is a command, not a tool: opening it hands MapCanvas
  // a draft grid to render live, and only saveGrid on Apply commits
  // it. Also local, not store state, for the same reason as
  // pendingPoint above.
  const [configuringGrid, setConfiguringGrid] = useState(false);
  const [draftGrid, setDraftGrid] = useState<Grid | null>(null);
  useEffect(() => {
    setConfiguringGrid(false);
    setDraftGrid(null);
  }, [currentLocationId, storage, editMode]);

  // The landing screen's picker, reopened as a panel over the editor —
  // switching projects, not editing this one, so it's available
  // regardless of editMode and isn't reset by any of the effects above.
  const [pickerOpen, setPickerOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);

  // Every Location in the project, filterable, independent of which
  // map you're on — navigation, not a mutation, so no editMode gate.
  const [browserOpen, setBrowserOpen] = useState(false);

  const [savingLink, setSavingLink] = useState(false);
  const [savingContent, setSavingContent] = useState(false);
  const [addingLocation, setAddingLocation] = useState(false);
  const [savingGrid, setSavingGrid] = useState(false);

  const project = data?.project;

  // Default to the project's defaultLocation once it loads.
  useEffect(() => {
    if (project && currentLocationId === null) {
      setCurrentLocation(project.defaultLocation);
    }
  }, [project, currentLocationId, setCurrentLocation]);

  // The current location's image, resolved to a displayable URL —
  // async because local (File System Access) storage reads the file
  // via the folder handle and hands back a blob: URL that has to be
  // revoked once we're done with it.
  const currentLocationImageFile = project?.locations.find((l) => l.id === currentLocationId)?.image?.file;
  const [resolvedImageUrl, setResolvedImageUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!storage || !currentLocationImageFile) {
      setResolvedImageUrl(null);
      return;
    }
    let cancelled = false;
    let objectUrl: string | null = null;
    storage.getImageUrl(currentLocationImageFile).then((url) => {
      if (cancelled) {
        URL.revokeObjectURL(url);
        return;
      }
      objectUrl = url;
      setResolvedImageUrl(url);
    });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [storage, currentLocationImageFile]);

  if (storage === null) return <OpenProjectForm />;
  if (loadError) return <div className="status error">{loadError}</div>;

  let mainContent: ReactNode = null;
  if (project) {
    const currentLocation = project.locations.find((l) => l.id === currentLocationId);
    if (!currentLocation) {
      mainContent = <div className="status error">Unknown location "{currentLocationId}"</div>;
    } else {
      const linkTitles = Object.fromEntries(
        currentLocation.links.map((link) => [link.target, data?.resolvedContent[link.target]?.title ?? link.target]),
      );

      const selectedLink = currentLocation.links.find((l) => l.id === selectedLinkId);
      // A selected link's own target location, if it has an image of its
      // own — i.e. it's not just a pin, it's a map you can click into.
      const selectedTargetLocation = selectedLink
        ? project.locations.find((l) => l.id === selectedLink.target)
        : undefined;

      const warnings = data?.warnings ?? [];
      const resolveErrors = data?.resolveErrors ?? {};

      const locationTitle = data?.resolvedContent[currentLocation.id]?.title ?? "";
      const locationBody = data?.resolvedContent[currentLocation.id]?.body ?? "";
      const isInlineContent = currentLocation.content === null || currentLocation.content.type === "inline";

      mainContent = (
        <div className="app">
          <aside className="sidebar">
            <div className="sidebar-header">
              <button type="button" className="logo-button" onClick={() => setAboutOpen(true)} aria-label="About Hex Enductor">
                <Logo size={28} />
              </button>
              <button type="button" className="project-switcher" onClick={() => setPickerOpen(true)}>
                {project.title}
              </button>
            </div>
            <button type="button" className="link-button sidebar-spaced" onClick={() => setBrowserOpen(true)}>
              Browse all locations…
            </button>

            <label className="mode-toggle">
              <input type="checkbox" checked={editMode} onChange={(e) => setEditMode(e.target.checked)} />
              <span className="mode-toggle-track" aria-hidden="true" />
              <span className="mode-toggle-label">{editMode ? "Editing" : "Viewing"}</span>
            </label>

            {editMode && (
              <button
                type="button"
                className={`tool-button${placingLocation ? " active" : ""}`}
                onClick={() => {
                  setConfiguringGrid(false);
                  setDraftGrid(null);
                  setPlacingLocation(!placingLocation);
                }}
              >
                {placingLocation ? "Click the map…" : "Add Location"}
              </button>
            )}

            {editMode && (
              <ImageUpload
                key={currentLocation.id}
                storage={storage}
                locationId={currentLocation.id}
                hasImage={currentLocation.image !== null}
                onDone={setData}
              />
            )}

            {editMode && currentLocation.image && (
              <button
                type="button"
                className="tool-button"
                onClick={() => {
                  setPlacingLocation(false);
                  setDraftGrid(currentLocation.grid);
                  setConfiguringGrid(true);
                }}
              >
                Configure grid…
              </button>
            )}

            {editMode && isInlineContent ? (
              <LocationContentForm
                key={currentLocation.id}
                locationId={currentLocation.id}
                title={locationTitle}
                body={locationBody}
                saving={savingContent}
                onSave={(patch) => {
                  setSavingContent(true);
                  storage
                    .saveLocationContent(currentLocation.id, patch)
                    .then(setData)
                    .finally(() => setSavingContent(false));
                }}
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
                <summary>{warnings.length + Object.keys(resolveErrors).length} warning(s)</summary>
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
              {currentLocation.links.map((link) => {
                const icon = findLinkIcon(link.icon);
                return (
                  <li key={link.id}>
                    <button
                      className={link.id === selectedLinkId ? "selected" : ""}
                      onClick={() => selectLink(link.id)}
                    >
                      {icon && <span className="icon-swatch" dangerouslySetInnerHTML={{ __html: icon.svg }} />}
                      {linkTitles[link.target]}
                      {link.hidden ? " (hidden)" : ""}
                    </button>
                  </li>
                );
              })}
              {currentLocation.links.length === 0 && <li className="muted">No locations pinned here yet.</li>}
            </ul>

            <p className="icon-credit">
              Map icons by{" "}
              <a href="https://delapouite.com" target="_blank" rel="noreferrer">
                Delapouite
              </a>{" "}
              and{" "}
              <a href="https://lorcblog.blogspot.com" target="_blank" rel="noreferrer">
                Lorc
              </a>{" "}
              via{" "}
              <a href="https://game-icons.net" target="_blank" rel="noreferrer">
                game-icons.net
              </a>
              , CC BY 3.0.
            </p>
          </aside>

          <main className="map-area">
            {currentLocation.image && resolvedImageUrl ? (
              <>
                <MapCanvas
                  image={currentLocation.image}
                  imageUrl={resolvedImageUrl}
                  grid={configuringGrid ? draftGrid : currentLocation.grid}
                  gridVisible={gridVisible || configuringGrid}
                  links={currentLocation.links}
                  linkTitles={linkTitles}
                  selectedLinkId={selectedLinkId ?? undefined}
                  onSelectLink={selectLink}
                  placing={placingLocation && !configuringGrid}
                  onPlaceLocation={setPendingPoint}
                />
                <label className="grid-toggle">
                  <input type="checkbox" checked={gridVisible} onChange={(e) => setGridVisible(e.target.checked)} />
                  Show grid
                </label>
              </>
            ) : currentLocation.image ? (
              <div className="status">Loading image…</div>
            ) : (
              <div className="status">"{currentLocation.id}" has no image — nothing to render.</div>
            )}
          </main>

          {configuringGrid && currentLocation.image ? (
            <aside className="edit-panel">
              <ConfigureGridForm
                key={currentLocation.id}
                initialGrid={draftGrid}
                image={currentLocation.image}
                onPreview={setDraftGrid}
                saving={savingGrid}
                onApply={(grid) => {
                  setSavingGrid(true);
                  storage
                    .saveGrid(currentLocation.id, grid)
                    .then((d) => {
                      setData(d);
                      setConfiguringGrid(false);
                      setDraftGrid(null);
                    })
                    .finally(() => setSavingGrid(false));
                }}
                onCancel={() => {
                  setConfiguringGrid(false);
                  setDraftGrid(null);
                }}
              />
            </aside>
          ) : placingLocation && pendingPoint ? (
            <aside className="edit-panel">
              <AddLocationForm
                saving={addingLocation}
                onSave={(values) => {
                  setAddingLocation(true);
                  storage
                    .addLocationLink(currentLocation.id, values.locationId, pendingPoint.x, pendingPoint.y, values.type)
                    .then((d) => {
                      setData(d);
                      setPlacingLocation(false);
                    })
                    .finally(() => setAddingLocation(false));
                }}
                onCancel={() => setPendingPoint(null)}
              />
            </aside>
          ) : (
            selectedLink && (
              <aside className="edit-panel">
                {editMode ? (
                  <LinkForm
                    key={selectedLink.id}
                    link={selectedLink}
                    title={linkTitles[selectedLink.target] ?? selectedLink.target}
                    saving={savingLink}
                    onSave={(patch) => {
                      setSavingLink(true);
                      storage
                        .saveLink(currentLocation.id, selectedLink.id, patch as Partial<Link>)
                        .then(setData)
                        .finally(() => setSavingLink(false));
                    }}
                  />
                ) : (
                  <div className="location-heading">
                    <h2>{linkTitles[selectedLink.target] ?? selectedLink.target}</h2>
                    <p className="location-body">{selectedLink.type}</p>
                  </div>
                )}
                {/* Navigation, not a mutation — available in view mode too. */}
                {selectedTargetLocation?.image && (
                  <button className="link-button" onClick={() => setCurrentLocation(selectedTargetLocation.id)}>
                    View map →
                  </button>
                )}
              </aside>
            )
          )}
        </div>
      );
    }
  }

  return (
    <>
      <LoadingScreen active={!project} />
      {mainContent}
      {aboutOpen && <AboutModal onClose={() => setAboutOpen(false)} />}
      {pickerOpen && (
        <div className="modal-backdrop">
          <div className="modal-panel">
            <ProjectPicker onClose={() => setPickerOpen(false)} />
          </div>
        </div>
      )}
      {browserOpen && project && (
        <div className="modal-backdrop">
          <div className="modal-panel">
            <LocationBrowser
              project={project}
              resolvedContent={data?.resolvedContent ?? {}}
              currentLocationId={currentLocationId}
              onSelect={setCurrentLocation}
              onClose={() => setBrowserOpen(false)}
            />
          </div>
        </div>
      )}
    </>
  );
}

export default App;
