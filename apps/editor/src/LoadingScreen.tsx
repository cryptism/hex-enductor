import { useEffect, useState } from "react";
import { BrandMark } from "./Logo.tsx";

const FADE_MS = 320;

// A branded cover for page-level async waits: fades in as soon as it
// mounts, stays mounted through its own fade-out after `active` goes
// false so the transition can actually play, then unmounts.
export function LoadingScreen({ active }: { active: boolean }) {
  const [mounted, setMounted] = useState(active);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (active) {
      setMounted(true);
      const id = requestAnimationFrame(() => setVisible(true));
      return () => cancelAnimationFrame(id);
    }
    setVisible(false);
  }, [active]);

  useEffect(() => {
    if (!active && mounted) {
      const t = setTimeout(() => setMounted(false), FADE_MS);
      return () => clearTimeout(t);
    }
  }, [active, mounted]);

  if (!mounted) return null;

  return (
    <div className={`loading-screen${visible ? " visible" : ""}`} role="status" aria-live="polite">
      <div className="loading-grid">
        <BrandMark size={56} />
        <div className="throbber-row">
          <span className="throbber" aria-hidden="true" />
        </div>
      </div>
    </div>
  );
}
