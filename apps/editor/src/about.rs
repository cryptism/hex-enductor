use leptos::prelude::*;

// Lyrics, one per track, from The Fall's "Hex Enduction Hour" — linked
// to the official YouTube upload of each song.
const HEX_LYRICS: &[(&str, &str)] = &[
    (
        r#"Made with the highest British attention to the wrong detail!"#,
        "https://youtu.be/-aDYIvKLBT8?si=Bfl2ThKIsO8g4EB3",
    ),
    (
        r#""Explain, move into the light of the moon""#,
        "https://youtu.be/j6VrY66_UJ8?si=csimCFaAK_G5JwsC",
    ),
    (
        r#"Drink the long draught, Dan!"#,
        "https://youtu.be/dx10715hFKo?si=1snWjgklQtHZ0ds0",
    ),
    (
        r#"Have *you* been to the English deer park?"#,
        "https://youtu.be/pcEajscsyQ0?si=aT1R2mIqcTyU6v_R",
    ),
    (
        r#"Decadent sandwich quaff!"#,
        "https://youtu.be/DTWwNtW1RyA?si=jYVVcLi0x28g6NCe",
    ),
    (
        r#"All entrances delivered!"#,
        "https://youtu.be/ZYE6lqMGbLQ?si=SRIXm99vP4zaWgam",
    ),
    (
        r#""Wear the gold and put it on""#,
        "https://youtu.be/jFXNraX4uSg?si=z5eCRo7rHE9OWQ4g",
    ),
    (
        r#"Hit those lung wurm back rays!"#,
        "https://youtu.be/rzf_oDLLnzU?si=DzTKmzs5BoQ_AU65",
    ),
    (
        r#"Who makes the nazis?"#,
        "https://youtu.be/DAqrFhGT4tQ?si=UNRcScMPJdpK-4nt",
    ),
    (
        r#"Cast the runes against your own soul!"#,
        "https://youtu.be/2h5BGIKcMso?si=ZFliMcdMjjWliVPq",
    ),
    (
        r#"Blades make presence felt!"#,
        "https://youtu.be/qlG1yvoUVFM?si=eVlvdDkQrxQs32kT",
    ),
];

#[component]
pub fn AboutModal(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (text, url) =
        HEX_LYRICS[(js_sys::Math::random() * HEX_LYRICS.len() as f64) as usize % HEX_LYRICS.len()];
    view! {
        <div class="modal-backdrop" on:click=move |_| on_close.run(())>
            <div class="modal-panel about-modal" on:click=|e| e.stop_propagation()>
                <button type="button" class="link-button picker-close" on:click=move |_| on_close.run(())>
                    "Close"
                </button>
                <img src="/hex-enductor.png" alt="Hex Enductor" class="about-banner" />
                <a href=url target="_blank" rel="noreferrer" class="about-easter-egg">
                    {text}
                </a>

                <section>
                    <h4>"Credits"</h4>
                    <p>
                        "Map icons by "
                        <a href="https://delapouite.com" target="_blank" rel="noreferrer">"Delapouite"</a>
                        " and "
                        <a href="https://lorcblog.blogspot.com" target="_blank" rel="noreferrer">"Lorc"</a>
                        " via "
                        <a href="https://game-icons.net" target="_blank" rel="noreferrer">"game-icons.net"</a>
                        ", licensed "
                        <a href="https://creativecommons.org/licenses/by/3.0/" target="_blank" rel="noreferrer">
                            "CC BY 3.0"
                        </a>
                        "."
                    </p>
                </section>

                <section>
                    <h4>"License"</h4>
                    <p>"MIT License. Copyright © 2026 Joe Whittles."</p>
                </section>

                <p>
                    <a href="https://github.com/cryptism/hex-enductor" target="_blank" rel="noreferrer">
                        "github.com/cryptism/hex-enductor"
                    </a>
                </p>
            </div>
        </div>
    }
}
