// Design rule: below 48px the full ink-wash mark (logo.svg) turns to mud —
// swap to the flat, single-weight glyph (logo-simple.svg) instead.
const SIMPLIFIED_BELOW = 48;

export function Logo({ size = 48, className }: { size?: number; className?: string }) {
  const src = size < SIMPLIFIED_BELOW ? "/logo-simple.svg" : "/logo.svg";
  return <img src={src} width={size} height={size} alt="" className={className} />;
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
