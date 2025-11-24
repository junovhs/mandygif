#![allow(non_snake_case)]

use crate::components::control_bar::ControlBar;
use crate::components::resize_handle::ResizeHandles;
use crate::hooks::use_recorder;
use crate::state::{AppMode, AppState};
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[allow(dependency_on_unit_never_type_fallback)]
pub fn App() -> Element {
    use_context_provider(AppState::new);
    let state = use_context::<AppState>();
    let window = use_window();
    let recorder = use_recorder();

    let current_mode = *state.mode.read();

    // Determine CSS class based on mode
    let mode_class = match current_mode {
        AppMode::Recording => "mode-recording",
        AppMode::Review | AppMode::Exporting => "mode-review",
        _ => "mode-idle",
    };

    let drag_win = window.clone();
    let close_handler = move |_: MouseEvent| {
        std::process::exit(0);
    };

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            // Apply the dynamic class here
            class: "app-frame {mode_class}",

            if current_mode == AppMode::Idle || current_mode == AppMode::Review {
                ResizeHandles {}
            }

            div {
                class: "header",
                style: "background: rgba(20,20,20,0.9); padding: 6px 12px; display: flex; justify-content: space-between; align-items: center; border-bottom-right-radius: 8px; width: fit-content; border: 1px solid rgba(255,255,255,0.1); border-top: none; border-left: none;",

                span {
                    onmousedown: move |_| drag_win.drag(),
                    style: "color: white; font-weight: 700; font-size: 13px; margin-right: 15px; cursor: move; user-select: none; letter-spacing: 0.5px; font-family: sans-serif;",
                    "MandyGIF"
                }

                button {
                    onclick: close_handler,
                    style: "background: #ff4444; color: white; border: none; border-radius: 50%; width: 16px; height: 16px; display: flex; align-items: center; justify-content: center; cursor: pointer; font-size: 10px; line-height: 1;",
                    "✕"
                }
            }

            div {
                style: "position: absolute; bottom: 20px; left: 20px;",
                ControlBar {
                    on_record: recorder.start,
                    on_stop: recorder.stop,
                    on_export: recorder.export
                }
            }
        }
    }
}
