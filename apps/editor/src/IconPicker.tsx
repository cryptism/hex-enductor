import { useMemo, useState } from "react";
import { LINK_ICONS, findLinkIcon, type IconCategory } from "@hex-enductor/map-core";

const CATEGORY_LABELS: Record<IconCategory, string> = {
  settlement: "Settlement",
  landmark: "Landmark",
  ruin: "Ruin",
  hazard: "Hazard",
  waypoint: "Waypoint",
};
const CATEGORY_ORDER = Object.keys(CATEGORY_LABELS) as IconCategory[];

export interface IconPickerProps {
  value: string | undefined;
  onChange: (slug: string | undefined) => void;
}

export function IconPicker({ value, onChange }: IconPickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  const selected = findLinkIcon(value);

  const grouped = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = q
      ? LINK_ICONS.filter((icon) => icon.label.toLowerCase().includes(q) || icon.slug.includes(q))
      : LINK_ICONS;
    return CATEGORY_ORDER.map((category) => ({
      category,
      icons: filtered.filter((icon) => icon.category === category),
    })).filter((group) => group.icons.length > 0);
  }, [query]);

  return (
    <div className="icon-picker">
      <button type="button" className="icon-picker-toggle" onClick={() => setOpen(!open)}>
        {selected ? (
          <>
            <span className="icon-swatch" dangerouslySetInnerHTML={{ __html: selected.svg }} />
            {selected.label}
          </>
        ) : (
          "(none)"
        )}
        <span className="icon-picker-caret">{open ? "▲" : "▼"}</span>
      </button>

      {open && (
        <div className="icon-picker-panel">
          <input
            type="text"
            placeholder="Search icons…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoFocus
          />
          <button
            type="button"
            className="icon-picker-none"
            onClick={() => {
              onChange(undefined);
              setOpen(false);
            }}
          >
            None
          </button>
          {grouped.map(({ category, icons }) => (
            <div key={category} className="icon-picker-group">
              <h4>{CATEGORY_LABELS[category]}</h4>
              <div className="icon-picker-grid">
                {icons.map((icon) => (
                  <button
                    key={icon.slug}
                    type="button"
                    title={icon.label}
                    className={`icon-picker-item${icon.slug === value ? " selected" : ""}`}
                    onClick={() => {
                      onChange(icon.slug);
                      setOpen(false);
                    }}
                    dangerouslySetInnerHTML={{ __html: icon.svg }}
                  />
                ))}
              </div>
            </div>
          ))}
          {grouped.length === 0 && <p className="muted">No icons match "{query}".</p>}
        </div>
      )}
    </div>
  );
}
