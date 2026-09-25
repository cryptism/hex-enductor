// Experimental: a from-scratch Leptos rewrite of apps/presentation
// (currently React/Vite, see apps/presentation/src/PresentationApp.tsx).
// This is a skeleton only — it doesn't connect to hexend's live
// session yet, just proves out SSR + hydration wiring. See
// apps/presentation-rs/src/main.rs for the axum server side and
// flake.nix for the wasm32 toolchain note.
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/presentation-rs.css"/>
        <Title text="hex-enductor presentation (skeleton)"/>
        <main>
            <h1>"hex-enductor — presentation (Rust/Leptos skeleton)"</h1>
            <p>"Placeholder only — no live session connection yet."</p>
        </main>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
