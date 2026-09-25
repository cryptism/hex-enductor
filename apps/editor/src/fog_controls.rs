use hexen_proto::hexen::v1::{FogOfWar, ImageRef};
use leptos::prelude::*;
use map_core::fog::{fog_grid_dims, hidden_fog_cells};

#[derive(Clone, Copy, PartialEq)]
enum BlanketAction {
    Cover,
    Reveal,
}

/// GM mode's fog panel — a Krita-style layer toggle (peek under the fog
/// without changing it) plus a status readout that stays truthful about
/// what players see regardless of that peek, always available in GM
/// mode. The paint tool and blanket apply/remove only appear once edit
/// mode is also on, since they mutate the project.
#[component]
pub fn FogControls(
    #[prop(into)] image: Signal<ImageRef>,
    #[prop(into)] fog: Signal<Option<FogOfWar>>,
    #[prop(into)] edit_mode: Signal<bool>,
    layer_visible: RwSignal<bool>,
    #[prop(into)] painting_fog: Signal<bool>,
    on_set_painting_fog: Callback<bool>,
    on_set_fog: Callback<Option<FogOfWar>>,
) -> impl IntoView {
    let pending = RwSignal::new(None::<BlanketAction>);

    let status = move || {
        let (w, h) = image.with(|i| (i.width as f64, i.height as f64));
        let dims = fog_grid_dims(w, h);
        fog.with(|fog| match fog {
            Some(fog) => format!(
                "Fog is live for players — {} of {} cells hidden.",
                hidden_fog_cells(w, h, &fog.revealed_cells).len(),
                dims.cols * dims.rows
            ),
            None => "Fog is off — players see the full map.".into(),
        })
    };

    view! {
        <div class="fog-controls">
            <p class="fog-status">{status}</p>

            <label class="grid-toggle fog-layer-toggle">
                <input type="checkbox" bind:checked=layer_visible />
                "Show fog layer"
            </label>

            <Show when=move || edit_mode.get()>
                <button
                    type="button"
                    class=move || if painting_fog.get() { "tool-button active" } else { "tool-button" }
                    disabled=move || fog.with(Option::is_none)
                    on:click=move |_| on_set_painting_fog.run(!painting_fog.get_untracked())
                >
                    {move || if painting_fog.get() { "Click the map…" } else { "Paint fog" }}
                </button>

                <div class="tool-row">
                    <button type="button" class="tool-button" on:click=move |_| pending.set(Some(BlanketAction::Cover))>
                        "Fog entire map"
                    </button>
                    <button type="button" class="tool-button" on:click=move |_| pending.set(Some(BlanketAction::Reveal))>
                        "Reveal entire map"
                    </button>
                </div>
            </Show>

            {move || {
                pending
                    .get()
                    .map(|action| {
                        let cover = action == BlanketAction::Cover;
                        view! {
                            <div class="modal-backdrop">
                                <div class="modal-panel">
                                    <p>
                                        {if cover {
                                            "Cover the entire map in fog? Any cells already revealed to players will be hidden again."
                                        } else {
                                            "Reveal the entire map? Fog will be turned off and players will see everything."
                                        }}
                                    </p>
                                    <div class="form-actions">
                                        <button
                                            type="button"
                                            class="tool-button danger"
                                            on:click=move |_| {
                                                on_set_fog.run(cover.then(FogOfWar::default));
                                                pending.set(None);
                                            }
                                        >
                                            {if cover { "Yes, fog it" } else { "Yes, reveal it" }}
                                        </button>
                                        <button type="button" class="link-button" on:click=move |_| pending.set(None)>
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}
