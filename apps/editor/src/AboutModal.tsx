import { useState } from "react";

// Lyrics, one per track, from The Fall's "Hex Enduction Hour" — linked to the
// official YouTube upload of each song.
const HEX_LYRICS: { text: string; url: string }[] = [
  {
    text: "Made with the highest British attention to the wrong detail!",
    url: "https://youtu.be/-aDYIvKLBT8?si=Bfl2ThKIsO8g4EB3",
  },
  {
    text: '"Explain, move into the light of the moon"',
    url: "https://youtu.be/j6VrY66_UJ8?si=csimCFaAK_G5JwsC",
  },
  {
    text: "Drink the long draught, Dan!",
    url: "https://youtu.be/dx10715hFKo?si=1snWjgklQtHZ0ds0",
  },
  {
    text: "Have *you* been to the English deer park?",
    url: "https://youtu.be/pcEajscsyQ0?si=aT1R2mIqcTyU6v_R",
  },
  {
    text: "Decadent sandwich quaff!",
    url: "https://youtu.be/DTWwNtW1RyA?si=jYVVcLi0x28g6NCe",
  },
  {
    text: "All entrances delivered!",
    url: "https://youtu.be/ZYE6lqMGbLQ?si=SRIXm99vP4zaWgam",
  },
  {
    text: '"Wear the gold and put it on"',
    url: "https://youtu.be/jFXNraX4uSg?si=z5eCRo7rHE9OWQ4g",
  },
  {
    text: "Hit those lung wurm back rays!",
    url: "https://youtu.be/rzf_oDLLnzU?si=DzTKmzs5BoQ_AU65",
  },
  {
    text: "Who makes the nazis?",
    url: "https://youtu.be/DAqrFhGT4tQ?si=UNRcScMPJdpK-4nt",
  },
  {
    text: "Cast the runes against your own soul!",
    url: "https://youtu.be/2h5BGIKcMso?si=ZFliMcdMjjWliVPq",
  },
  {
    text: "Blades make presence felt!",
    url: "https://youtu.be/qlG1yvoUVFM?si=eVlvdDkQrxQs32kT",
  },
];

function pickHexLyric() {
  return HEX_LYRICS[Math.floor(Math.random() * HEX_LYRICS.length)];
}

export function AboutModal({ onClose }: { onClose: () => void }) {
  const [lyric] = useState(pickHexLyric);
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-panel about-modal" onClick={(e) => e.stopPropagation()}>
        <button type="button" className="link-button picker-close" onClick={onClose}>
          Close
        </button>
        <img src="/hex-enductor.png" alt="Hex Enductor" className="about-banner" />
        <a
          href={lyric.url}
          target="_blank"
          rel="noreferrer"
          className="about-easter-egg"
        >
          {lyric.text}
        </a>

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
