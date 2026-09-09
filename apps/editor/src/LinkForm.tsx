import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import type { Link } from "@hex-enductor/hexen-schema";

const LinkFormSchema = z.object({
  x: z.coerce.number(),
  y: z.coerce.number(),
  type: z.string().min(1, "required"),
  icon: z.string(),
  color: z.string(),
  hidden: z.boolean(),
});
type LinkFormValues = z.infer<typeof LinkFormSchema>;

function linkToFormValues(link: Link): LinkFormValues {
  return {
    x: link.x,
    y: link.y,
    type: link.type,
    icon: link.icon ?? "",
    color: link.color ?? "",
    hidden: link.hidden,
  };
}

export interface LinkFormProps {
  link: Link;
  title: string;
  onSave: (patch: Partial<Link>) => void;
  saving: boolean;
}

export function LinkForm({ link, title, onSave, saving }: LinkFormProps) {
  const {
    register,
    handleSubmit,
    reset,
    formState: { isDirty, errors },
  } = useForm<LinkFormValues>({
    resolver: zodResolver(LinkFormSchema),
    values: linkToFormValues(link),
  });

  // react-hook-form's `values` option keeps the form in sync when a
  // different link is selected, but doesn't reset the dirty flag on
  // its own — do that explicitly so "Save" doesn't stay enabled after
  // switching selection.
  useEffect(() => {
    reset(linkToFormValues(link));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [link.id]);

  const submit = handleSubmit((values) => {
    onSave({
      x: values.x,
      y: values.y,
      type: values.type,
      icon: values.icon || undefined,
      color: values.color || null,
      hidden: values.hidden,
    });
  });

  return (
    <form onSubmit={submit} className="link-form">
      <h3>{title}</h3>
      <p className="link-form-id">{link.id}</p>

      <label>
        X
        <input type="number" step="any" {...register("x")} />
      </label>
      <label>
        Y
        <input type="number" step="any" {...register("y")} />
      </label>
      <label>
        Type
        <input type="text" {...register("type")} />
        {errors.type && <span className="field-error">{errors.type.message}</span>}
      </label>
      <label>
        Icon
        <input type="text" placeholder="(none)" {...register("icon")} />
      </label>
      <label>
        Color
        <input type="text" placeholder="(default)" {...register("color")} />
      </label>
      <label className="checkbox-label">
        <input type="checkbox" {...register("hidden")} />
        Hidden
      </label>

      <button type="submit" disabled={!isDirty || saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
