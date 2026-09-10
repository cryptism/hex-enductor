export function AboutModal({ onClose }: { onClose: () => void }) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-panel about-modal" onClick={(e) => e.stopPropagation()}>
        <button type="button" className="link-button picker-close" onClick={onClose}>
          Close
        </button>
        <img src="/hex-enductor.png" alt="Hex Enductor" className="about-banner" />

        <section>
          <h4>Credits</h4>
          <p>
            Map icons by{" "}
            <a href="https://delapouite.com" target="_blank" rel="noreferrer">
              Delapouite
            </a>{" "}
            and{" "}
            <a href="https://lorcblog.blogspot.com" target="_blank" rel="noreferrer">
              Lorc
            </a>{" "}
            via{" "}
            <a href="https://game-icons.net" target="_blank" rel="noreferrer">
              game-icons.net
            </a>
            , licensed{" "}
            <a href="https://creativecommons.org/licenses/by/3.0/" target="_blank" rel="noreferrer">
              CC BY 3.0
            </a>
            .
          </p>
        </section>

        <section>
          <h4>License</h4>
          <p>MIT License. Copyright © 2026 Joe Whittles.</p>
        </section>

        <p>
          <a href="https://github.com/cryptism/hex-enductor" target="_blank" rel="noreferrer">
            github.com/cryptism/hex-enductor
          </a>
        </p>
      </div>
    </div>
  );
}
