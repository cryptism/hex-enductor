import { useEffect, useState, type ReactNode } from "react";
import { MapCanvas } from "@hex-enductor/map-core";
import { useAppStore } from "./store.ts";
import { trpc, serverUrl } from "./trpc.ts";
import { LinkForm } from "./LinkForm.tsx";
import { LocationContentForm } from "./LocationContentForm.tsx";
import { AddLocationForm } from "./AddLocationForm.tsx";
import { ConfigureGridForm } from "./ConfigureGridForm.tsx";
import { ImageUpload } from "./ImageUpload.tsx";
import { ProjectPicker } from "./ProjectPicker.tsx";
import { BrandMark } from "./Logo.tsx";
import { LoadingScreen } from "./LoadingScreen.tsx";
import type { Grid, Link, Point } from "@hex-enductor/hexen-schema";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

function imageUrl(projectPath: string, file: string): string {
  const dir = dirname(projectPath);
  return `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&file=${encodeURIComponent(file)}`;
}

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
  const projectPath = useAppStore((s) => s.projectPath);
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

  // Where the Add Location tool's pending click landed, in image pixel
  // space — local, not store state, since it's meaningless the moment
  // placingLocation goes false (see the effect below).
  const [pendingPoint, setPendingPoint] = useState<Point | null>(null);
  useEffect(() => {
    if (!placingLocation) setPendingPoint(null);
  }, [placingLocation]);

  // Configure Grid is a command, not a tool: opening it hands MapCanvas
  // a draft grid to render live, and only saveGrid.mutate on Apply
  // commits it. Also local, not store state, for the same reason as
  // pendingPoint above.
  const [configuringGrid, setConfiguringGrid] = useState(false);
  const [draftGrid, setDraftGrid] = useState<Grid | null>(null);
  useEffect(() => {
    setConfiguringGrid(false);
    setDraftGrid(null);
  }, [currentLocationId, projectPath, editMode]);

  // The landing screen's picker, reopened as a panel over the editor —
  // switching projects, not editing this one, so it's available
  // regardless of editMode and isn't reset by any of the effects above.
  const [pickerOpen, setPickerOpen] = useState(false);

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
  const addLocationLink = trpc.addLocationLink.useMutation({
    onSuccess: () => utils.openProject.invalidate({ path: projectPath! }),
  });
  const saveGrid = trpc.saveGrid.useMutation({
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
            <button type="button" className="project-switcher" onClick={() => setPickerOpen(true)}>
              {project.title}
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
                projectPath={projectPath}
                locationId={currentLocation.id}
                hasImage={currentLocation.image !== null}
                onDone={() => utils.openProject.invalidate({ path: projectPath })}
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
            {currentLocation.image ? (
              <>
                <MapCanvas
                  image={currentLocation.image}
                  imageUrl={imageUrl(projectPath, currentLocation.image.file)}
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
                  <input
                    type="checkbox"
                    checked={gridVisible}
                    onChange={(e) => setGridVisible(e.target.checked)}
                  />
                  Show grid
                </label>
              </>
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
                saving={saveGrid.isPending}
                onApply={(grid) =>
                  saveGrid.mutate(
                    { path: projectPath, locationId: currentLocation.id, grid },
                    {
                      onSuccess: () => {
                        setConfiguringGrid(false);
                        setDraftGrid(null);
                      },
                    },
                  )
                }
                onCancel={() => {
                  setConfiguringGrid(false);
                  setDraftGrid(null);
                }}
              />
            </aside>
          ) : placingLocation && pendingPoint ? (
            <aside className="edit-panel">
              <AddLocationForm
                saving={addLocationLink.isPending}
                onSave={(values) =>
                  addLocationLink.mutate(
                    {
                      path: projectPath,
                      parentLocationId: currentLocation.id,
                      locationId: values.locationId,
                      x: pendingPoint.x,
                      y: pendingPoint.y,
                      type: values.type,
                    },
                    { onSuccess: () => setPlacingLocation(false) },
                  )
                }
                onCancel={() => setPendingPoint(null)}
              />
            </aside>
          ) : (
            editMode &&
            selectedLink && (
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
      {pickerOpen && (
        <div className="modal-backdrop">
          <div className="modal-panel">
            <ProjectPicker onClose={() => setPickerOpen(false)} />
          </div>
        </div>
      )}
    </>
  );
}

export default App;
