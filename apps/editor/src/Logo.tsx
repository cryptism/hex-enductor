// Design rule: below 48px the full ink-wash mark (logo.svg) turns to mud —
// swap to the flat, single-weight glyph (logo-simple.svg) instead.
const SIMPLIFIED_BELOW = 48;

// Rendered as a CSS mask, not an <img>, so the mark takes its color from
// the surrounding theme (currentColor) instead of being baked into the
// SVG file — see .logo-mark in styles.css.
export function Logo({ size = 48, className }: { size?: number; className?: string }) {
  const src = size < SIMPLIFIED_BELOW ? "/logo-simple.svg" : "/logo.svg";
  return (
    <span
      role="img"
      aria-label="Hex Enductor"
      className={["logo-mark", className].filter(Boolean).join(" ")}
      style={{
        width: size,
        height: size,
        WebkitMaskImage: `url(${src})`,
        maskImage: `url(${src})`,
      }}
    />
  );
}

export function BrandMark({ size = 48 }: { size?: number }) {
  return (
    <span className="brand-mark">
      <Logo size={size} />
      <span className="wordmark">
        <span>Hex</span>
        <span>Enductor</span>
      </span>
    </span>
  );
}
