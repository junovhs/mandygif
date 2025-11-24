// warden:ignore
use crate::components::icons::{IconExport, IconStop};
use crate::state::{use_app_state, AppMode};
use dioxus::prelude::*;

#[component]
pub fn ControlBar(
    on_record: EventHandler<()>,
    on_stop: EventHandler<()>,
    on_export: EventHandler<()>,
) -> Element {
    let mut state = use_app_state();
    let mode = state.mode.read();
    let duration = *state.duration_ms.read();

    let sec = (duration / 1000) % 60;
    let min = (duration / 1000) / 60;
    let time_str = format!("{min:02}:{sec:02}");

    rsx! {
        div {
            class: "control-shell",

            // ZONE 1: Left (Empty for now, cleaner look)
            if *mode != AppMode::Exporting {
                div { class: "zone-left" }
            }

            // ZONE 2: Center (Record/Timer)
            div {
                class: "zone-center",
                if *mode == AppMode::Idle {
                    button {
                        class: "record-trigger",
                        onclick: move |_| on_record.call(()),
                        div { class: "record-dot" }
                    }
                } else if *mode == AppMode::Recording {
                    div {
                        class: "timer-badge",
                        div { class: "pulse-dot" }
                        span { "{time_str}" }
                    }
                } else if *mode == AppMode::Review {
                     span { class: "review-text", "Review" }
                }
            }

            // ZONE 3: Right (Stop/Export)
            div {
                class: "zone-right",
                if *mode == AppMode::Recording {
                    button {
                        class: "stop-btn",
                        onclick: move |_| on_stop.call(()),
                        IconStop {}
                    }
                } else if *mode == AppMode::Review {
                    select {
                        class: "fmt-select",
                        onchange: move |evt| state.export_format.set(evt.value()),
                        option { value: "gif", "GIF" }
                        option { value: "mp4", "MP4" }
                        option { value: "webp", "WebP" }
                    }
                    button {
                        class: "action-btn",
                        onclick: move |_| on_export.call(()),
                        span { "Export" }
                        IconExport {}
                    }
                }
            }
        }
    }
}
