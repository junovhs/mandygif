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
    let duration = state.duration_ms.read();

    rsx! {
        div {
            class: "control-bar",
            style: "background: #1a1a1a; padding: 10px; border-radius: 8px; display: flex; gap: 10px; align-items: center; color: white;",

            if *mode == AppMode::Recording {
                div {
                    style: "color: #ff4444; font-weight: bold;",
                    "REC {*duration / 1000}s"
                }
                button {
                    onclick: move |_| {
                        tracing::info!("STOP button clicked");
                        on_stop.call(());
                    },
                    style: "background: #ff4444; color: white; border: none; padding: 5px 15px; border-radius: 4px; cursor: pointer;",
                    "STOP"
                }
            } else if *mode == AppMode::Idle {
                button {
                    onclick: move |_| {
                        tracing::info!("RECORD button clicked");
                        on_record.call(());
                    },
                    style: "background: #44ff44; color: black; border: none; padding: 5px 15px; border-radius: 4px; cursor: pointer;",
                    "RECORD"
                }
            } else if *mode == AppMode::Review {
                // Show Export Controls
                select {
                    onchange: move |evt| {
                        tracing::info!("Format changed: {}", evt.value());
                        state.export_format.set(evt.value());
                    },
                    style: "padding: 5px; border-radius: 4px; background: #333; color: white; border: 1px solid #555; cursor: pointer;",
                    option { value: "gif", "GIF" }
                    option { value: "mp4", "MP4" }
                    option { value: "webp", "WebP" }
                }
                button {
                    onclick: move |_| {
                        tracing::info!("EXPORT button clicked");
                        on_export.call(());
                    },
                    style: "background: #4488ff; color: white; border: none; padding: 5px 15px; border-radius: 4px; cursor: pointer;",
                    "EXPORT"
                }
                // Option to discard and re-record
                button {
                    onclick: move |_| {
                        state.mode.set(AppMode::Idle);
                    },
                    style: "background: transparent; color: #aaa; border: 1px solid #555; padding: 5px 10px; border-radius: 4px; cursor: pointer;",
                    "Discard"
                }
            } else if *mode == AppMode::Exporting {
                div { "Exporting..." }
            }
        }
    }
}
