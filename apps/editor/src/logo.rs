use leptos::prelude::*;

// Design rule: below 48px the full ink-wash mark (logo.svg) turns to mud —
// swap to the flat, single-weight glyph (logo-simple.svg) instead.
const SIMPLIFIED_BELOW: u32 = 48;

/// Rendered as a CSS mask, not an <img>, so the mark takes its color
/// from the surrounding theme (currentColor) — see .logo-mark in
/// styles.css.
#[component]
pub fn Logo(#[prop(default = 48)] size: u32) -> impl IntoView {
    let src = if size < SIMPLIFIED_BELOW {
        "/logo-simple.svg"
    } else {
        "/logo.svg"
    };
    let style = format!(
        "width: {size}px; height: {size}px; -webkit-mask-image: url({src}); mask-image: url({src});"
    );
    view! { <span role="img" aria-label="Hex Enductor" class="logo-mark" style=style></span> }
}

#[component]
pub fn BrandMark(#[prop(default = 48)] size: u32) -> impl IntoView {
    view! {
        <span class="brand-mark">
            <Logo size />
            <span class="wordmark">
                <span>"Hex"</span>
                <span>"Enductor"</span>
            </span>
        </span>
    }
}

const FADE_MS: i32 = 320;

/// A branded cover for page-level async waits: fades in as soon as it
/// mounts, stays mounted through its own fade-out after `active` goes
/// false so the transition can actually play, then unmounts.
#[component]
pub fn LoadingScreen(#[prop(into)] active: Signal<bool>) -> impl IntoView {
    let mounted = RwSignal::new(active.get_untracked());
    let visible = RwSignal::new(false);

    Effect::new(move |_| {
        if active.get() {
            mounted.set(true);
            request_animation_frame(move || visible.set(true));
        } else {
            visible.set(false);
            set_timeout(
                move || {
                    if !active.get_untracked() {
                        mounted.set(false);
                    }
                },
                std::time::Duration::from_millis(FADE_MS as u64),
            );
        }
    });

    move || {
        mounted.get().then(|| {
            view! {
                <div
                    class=move || if visible.get() { "loading-screen visible" } else { "loading-screen" }
                    role="status"
                    aria-live="polite"
                >
                    <div class="loading-grid">
                        <BrandMark size=56 />
                        <div class="throbber-row">
                            <span class="throbber" aria-hidden="true"></span>
                        </div>
                    </div>
                </div>
            }
        })
    }
}
