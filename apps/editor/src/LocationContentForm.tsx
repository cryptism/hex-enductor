import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const LocationContentFormSchema = z.object({
  title: z.string(),
  body: z.string(),
});
type LocationContentFormValues = z.infer<typeof LocationContentFormSchema>;

export interface LocationContentFormProps {
  locationId: string;
  title: string;
  body: string;
  onSave: (patch: LocationContentFormValues) => void;
  saving: boolean;
}

// A location's own title/body — editable directly here only because its
// content is (or will become) inline. Obsidian-backed locations are
// read-only in this app; that prose lives in the vault instead.
export function LocationContentForm({ locationId, title, body, onSave, saving }: LocationContentFormProps) {
  const {
    register,
    handleSubmit,
    reset,
    formState: { isDirty },
  } = useForm<LocationContentFormValues>({
    resolver: zodResolver(LocationContentFormSchema),
    values: { title, body },
  });

  useEffect(() => {
    reset({ title, body });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [locationId]);

  const submit = handleSubmit((values) => onSave(values));

  return (
    <form onSubmit={submit} className="link-form location-content-form">
      <label>
        Title
        <input type="text" placeholder="Untitled" {...register("title")} />
      </label>
      <label>
        Body
        <textarea rows={6} placeholder="Write something about this place…" {...register("body")} />
      </label>
      <button type="submit" disabled={!isDirty || saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
