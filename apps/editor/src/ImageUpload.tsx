import { useRef, useState } from "react";
import { trpc, serverUrl } from "./trpc.ts";

const ACCEPT = "image/png,image/jpeg";

function dirname(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "." : path.slice(0, i);
}

function extensionFor(file: File): string | null {
  if (file.type === "image/png") return "png";
  if (file.type === "image/jpeg") return "jpeg";
  const m = /\.(png|jpe?g)$/i.exec(file.name);
  return m ? m[1]!.toLowerCase() : null;
}

export interface ImageUploadProps {
  projectPath: string;
  locationId: string;
  hasImage: boolean;
  /** Called after either a successful upload or a successful remove — invalidate the project query. */
  onDone: () => void;
}

// A command, not a tool: one file dialog, one upload request, one
// mutation to point the location's `image` at the result. Re-uploading
// for the same location always overwrites the same server-side file
// (see apps/server's POST /image), so "replace" needs nothing extra
// here beyond calling this again.
export function ImageUpload({ projectPath, locationId, hasImage, onDone }: ImageUploadProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);
  const saveImage = trpc.saveImage.useMutation();

  async function handleFile(file: File) {
    const ext = extensionFor(file);
    if (!ext) {
      setError("Only PNG or JPEG images are supported.");
      return;
    }

    setUploading(true);
    setError(undefined);
    try {
      const dir = dirname(projectPath);
      const res = await fetch(
        `${serverUrl()}/image?dir=${encodeURIComponent(dir)}&locationId=${encodeURIComponent(locationId)}&ext=${ext}`,
        { method: "POST", body: file },
      );
      if (!res.ok) throw new Error(await res.text());
      const image = await res.json();

      await saveImage.mutateAsync({ path: projectPath, locationId, image });
      onDone();
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
        <button
          type="button"
          className="link-button"
          disabled={uploading || saveImage.isPending}
          onClick={() => saveImage.mutate({ path: projectPath, locationId, image: null }, { onSuccess: onDone })}
        >
          Remove image
        </button>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
