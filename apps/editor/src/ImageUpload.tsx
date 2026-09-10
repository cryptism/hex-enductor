import { useRef, useState } from "react";
import type { OpenedProjectData, ProjectStorage } from "./storage/index.ts";

const ACCEPT = "image/png,image/jpeg";

export interface ImageUploadProps {
  storage: ProjectStorage;
  locationId: string;
  hasImage: boolean;
  /** Called after either a successful upload or a successful remove — the fresh project data. */
  onDone: (data: OpenedProjectData) => void;
}

// A command, not a tool: one file dialog, one call to the active
// storage backend. Which backend that is (server or local folder)
// doesn't matter here — that's exactly what ProjectStorage is for.
// Re-uploading for the same location always overwrites the same
// underlying file, so "replace" needs nothing extra beyond calling
// this again.
export function ImageUpload({ storage, locationId, hasImage, onDone }: ImageUploadProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);

  async function handleFile(file: File) {
    setUploading(true);
    setError(undefined);
    try {
      onDone(await storage.uploadImage(locationId, file));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  }

  async function handleRemove() {
    setUploading(true);
    setError(undefined);
    try {
      onDone(await storage.removeImage(locationId));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  }

  return (
    <div className="image-upload">
      <input
        ref={inputRef}
        type="file"
        accept={ACCEPT}
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (file) void handleFile(file);
        }}
      />
      <button type="button" className="tool-button" disabled={uploading} onClick={() => inputRef.current?.click()}>
        {uploading ? "Uploading…" : hasImage ? "Replace image…" : "Upload image…"}
      </button>
      {hasImage && (
        <button type="button" className="link-button" disabled={uploading} onClick={() => void handleRemove()}>
          Remove image
        </button>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
