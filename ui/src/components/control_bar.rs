// warden:ignore
use crate::components::icons::{IconExport, IconMic, IconRecord, IconSettings, IconStop};
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

    // Calculate formatted time
    let d = *duration;
    let seconds = (d / 1000) % 60;
    let minutes = (d / 1000) / 60;
    let time_str = format!("{minutes:02}:{seconds:02}");

    rsx! {
        div {
            class: "control-pill",

            // Settings / Config (Left side)
            if *mode != AppMode::Recording {
                button { class: "icon-btn", title: "Microphone", IconMic {} }
                button { class: "icon-btn", title: "Settings", IconSettings {} }
                div { class: "pill-divider" }
            }

            // Main Action Button (Center)
            if *mode == AppMode::Recording {
                div { class: "timer", "{time_str}" }
                button {
                    class: "icon-btn btn-stop",
                    onclick: move |_| on_stop.call(()),
                    IconStop {}
                }
            } else if *mode == AppMode::Idle {
                button {
                    class: "icon-btn btn-record",
                    onclick: move |_| on_record.call(()),
                    IconRecord {}
                }
            } else if *mode == AppMode::Review {
                 // Export Section
                select {
                    class: "icon-btn",
                    style: "font-size: 12px; width: auto; padding: 4px 8px; border-radius: 4px;",
                    onchange: move |evt| state.export_format.set(evt.value()),
                    option { value: "gif", "GIF" }
                    option { value: "mp4", "MP4" }
                    option { value: "webp", "WebP" }
                }
                button {
                    class: "icon-btn",
                    style: "color: white; gap: 6px; padding-right: 12px; width: auto; border-radius: 20px; background: #007AFF;",
                    onclick: move |_| on_export.call(()),
                    IconExport {}
                    span { "Export" }
                }
                button {
                    class: "icon-btn",
                    title: "Discard",
                    onclick: move |_| state.mode.set(AppMode::Idle),
                    "✕"
                }
            } else if *mode == AppMode::Exporting {
                div {
                    style: "color: var(--text-secondary); font-size: 13px;",
                    "Rendering..."
                }
            }
        }
    }
}
