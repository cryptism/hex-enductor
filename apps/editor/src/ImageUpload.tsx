import { useRef, useState } from "react";
import type { ProjectStorage } from "./storage/index.ts";

const ACCEPT = "image/png,image/jpeg";

export interface ImageUploadProps {
  storage: ProjectStorage;
  locationId: string;
  hasImage: boolean;
}

// A command, not a tool: one file dialog, one call to the active
// storage backend. Which backend that is (server or local folder)
// doesn't matter here — that's exactly what ProjectStorage is for. The
// resulting project data isn't returned from here — it arrives through
// the storage's own subscribe(), same as every other mutation. Re-
// uploading for the same location always overwrites the same
// underlying file, so "replace" needs nothing extra beyond calling
// this again.
export function ImageUpload({ storage, locationId, hasImage }: ImageUploadProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);

  async function handleFile(file: File) {
    setUploading(true);
    setError(undefined);
    try {
      await storage.uploadImage(locationId, file);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploading(false);
    }
  }

  function handleRemove() {
    setError(undefined);
    storage.removeImage(locationId);
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
        <button type="button" className="link-button" disabled={uploading} onClick={handleRemove}>
          Remove image
        </button>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
