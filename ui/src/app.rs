#![allow(non_snake_case)]

use crate::components::control_bar::ControlBar;
use crate::components::resize_handle::ResizeHandles;
use crate::hooks::use_recorder;
use crate::state::{AppMode, AppState};
use dioxus::desktop::use_window;
use dioxus::prelude::*;

// FIX: Suppress the Rust 2024 compatibility warning regarding exit(0)
#[allow(dependency_on_unit_never_type_fallback)]
pub fn App() -> Element {
    use_context_provider(AppState::new);
    let state = use_context::<AppState>();
    let window = use_window();
    let recorder = use_recorder();

    let current_mode = *state.mode.read();
    let border_col = if current_mode == AppMode::Recording {
        "#ff0000"
    } else {
        "#00ff00"
    };

    let drag_win = window.clone();

    // We don't need explicit types here anymore due to the function-level allow
    let close_handler = move |_: MouseEvent| {
        std::process::exit(0);
    };

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            class: "app-frame",
            style: "border: 1px solid {border_col};",

            if current_mode == AppMode::Idle || current_mode == AppMode::Review {
                ResizeHandles {}
            }

            div {
                class: "header",
                style: "background: rgba(0,0,0,0.8); padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; border-bottom-right-radius: 6px; width: fit-content;",

                span {
                    onmousedown: move |_| drag_win.drag(),
                    style: "color: white; font-weight: 700; font-size: 14px; margin-right: 15px; cursor: move; user-select: none; letter-spacing: 0.5px;",
                    "MandyGIF"
                }

                button {
                    onclick: close_handler,
                    style: "background: #ff4444; color: white; border: none; border-radius: 3px; padding: 2px 6px; cursor: pointer; font-weight: 800; font-size: 12px;",
                    "✕"
                }
            }

            div {
                style: "position: absolute; bottom: 15px; left: 15px;",
                ControlBar {
                    on_record: recorder.start,
                    on_stop: recorder.stop,
                    on_export: recorder.export
                }
            }
        }
    }
}
