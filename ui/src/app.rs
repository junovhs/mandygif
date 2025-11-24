#![allow(non_snake_case)]

use crate::components::control_bar::ControlBar;
use crate::components::resize_handle::ResizeHandles;
use crate::hooks::use_recorder;
use crate::state::{AppMode, AppState};
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[allow(dependency_on_unit_never_type_fallback)]
pub fn App() -> Element {
    use_context_provider(AppState::new);
    let state = use_context::<AppState>();
    let window = use_window();
    let recorder = use_recorder();

    // Initial size setup
    let win_startup = window.clone();
    use_hook(move || {
        win_startup.set_inner_size(LogicalSize::new(800.0, 600.0));
    });

    let current_mode = *state.mode.read();
    let drag_win = window.clone();

    // Class determines border color & background tint
    let state_class = match current_mode {
        AppMode::Recording => "state-recording",
        AppMode::Review | AppMode::Exporting => "state-review",
        AppMode::Idle => "state-idle",
    };

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            class: "app-frame {state_class}",

            // 1. Resize Handles (Only interactive in Idle)
            if current_mode == AppMode::Idle {
                ResizeHandles {}
            }

            // 2. Visible Drag Bar (Top) - "Obviously grabbable"
            div {
                class: "drag-bar",
                onmousedown: move |_| drag_win.drag(),
                span { class: "drag-bar-label", "MandyGIF" }
            }

            // 3. Floating Control Bar
            ControlBar {
                on_record: recorder.start,
                on_stop: recorder.stop,
                on_export: recorder.export
            }
        }
    }
}
