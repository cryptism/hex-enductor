import { useState } from "react";

export interface AddLocationFormProps {
  onSave: (values: { locationId: string; type: string }) => void;
  onCancel: () => void;
  saving: boolean;
}

// Deliberately just two fields — a click already fixed x/y, and
// icon/color/hidden all have sensible defaults editable afterward via
// the ordinary LinkForm once the pin exists.
export function AddLocationForm({ onSave, onCancel, saving }: AddLocationFormProps) {
  const [locationId, setLocationId] = useState("");
  const [type, setType] = useState("");

  const canSave = locationId.trim().length > 0 && type.trim().length > 0;

  return (
    <form
      className="link-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (canSave) onSave({ locationId: locationId.trim(), type: type.trim() });
      }}
    >
      <h3>New location</h3>
      <label>
        Location id
        <input
          type="text"
          value={locationId}
          onChange={(e) => setLocationId(e.target.value)}
          placeholder="e.g. old-mill"
          autoFocus
        />
      </label>
      <label>
        Type
        <input
          type="text"
          value={type}
          onChange={(e) => setType(e.target.value)}
          placeholder="e.g. settlement"
        />
      </label>
      <div className="form-actions">
        <button type="submit" disabled={!canSave || saving}>
          {saving ? "Adding…" : "Add"}
        </button>
        <button type="button" className="link-button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}
