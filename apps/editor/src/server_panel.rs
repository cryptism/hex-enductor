//! The desktop app's Server panel: where hexend is, the presentation
//! window, and opt-in LAN sharing for player-facing screens.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::desktop::{self, ServerInfo};

#[component]
pub fn ServerPanel(
    /// The open project's path on the server, if it's a server project.
    project_path: Option<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let info = RwSignal::new(desktop::desktop_info());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let copied = RwSignal::new(false);

    spawn_local(async move {
        match desktop::server_info().await {
            Ok(i) => info.set(Some(i)),
            Err(e) => error.set(Some(e)),
        }
    });

    let set_lan = move |enabled: bool| {
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match desktop::set_lan_sharing(enabled).await {
                Ok(i) => info.set(Some(i)),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let open_window = {
        let path = project_path.clone();
        move |_| {
            let Some(path) = path.clone() else { return };
            spawn_local(async move {
                if let Err(e) = desktop::open_presentation_window(&path).await {
                    error.set(Some(e));
                }
            });
        }
    };

    let lan_link = {
        let path = project_path.clone();
        move || {
            let lan = info.with(|i| i.as_ref().and_then(|i| i.lan_url.clone()))?;
            Some(
                path.as_deref()
                    .map(|p| desktop::presentation_url(&lan, p))
                    .unwrap_or(lan),
            )
        }
    };
    let copy = move |text: String| {
        if let Some(window) = web_sys::window() {
            let _ = window.navigator().clipboard().write_text(&text);
            copied.set(true);
        }
    };

    view! {
        <div class="modal-backdrop" on:click=move |_| on_close.run(())>
            <div class="modal-panel server-panel" on:click=|e| e.stop_propagation()>
                <button type="button" class="link-button picker-close" on:click=move |_| on_close.run(())>
                    "Close"
                </button>
                <h3>"Server"</h3>
                {move || {
                    info.get()
                        .map(|ServerInfo { server_url, version, .. }| {
                            view! {
                                <p class="muted">
                                    "Hex Enductor " {version} " is running its own server at "
                                    <code>{server_url}</code>
                                    " — reachable from this computer only."
                                </p>
                            }
                        })
                }}

                <section>
                    <h4>"Presentation"</h4>
                    {if project_path.is_some() {
                        view! {
                            <button type="button" class="tool-button" on:click=open_window>
                                "Open presentation window"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! { <p class="muted">"Open a project to present it."</p> }.into_any()
                    }}
                </section>

                <section>
                    <h4>"Share on this network"</h4>
                    <label class="checkbox-label">
                        <input
                            type="checkbox"
                            prop:checked=move || info.with(|i| i.as_ref().is_some_and(|i| i.lan_url.is_some()))
                            disabled=move || busy.get()
                            on:change=move |ev| set_lan(event_target_checked(&ev))
                        />
                        "Let other devices on this network watch the presentation"
                    </label>
                    <p class="muted">
                        "Read-only: they see projects open here, live, and can't change anything or browse this computer. "
                        "There's no password — anyone on the network can connect while this is on."
                    </p>
                    {move || {
                        lan_link()
                            .map(|link| {
                                let for_copy = link.clone();
                                view! {
                                    <div class="server-link">
                                        <input type="text" readonly prop:value=link />
                                        <button type="button" on:click=move |_| copy(for_copy.clone())>
                                            {move || if copied.get() { "Copied" } else { "Copy" }}
                                        </button>
                                    </div>
                                }
                            })
                    }}
                </section>

                {move || error.get().map(|e| view! { <span class="field-error">{e}</span> })}
            </div>
        </div>
    }
}
