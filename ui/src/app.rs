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

    // Initial size
    let win_startup = window.clone();
    use_hook(move || {
        // FIX: Removed `let _ =` (clippy::let_unit_value)
        win_startup.set_inner_size(LogicalSize::new(800.0, 600.0));
    });

    let current_mode = *state.mode.read();
    let drag_win = window.clone();

    // Determine state class for CSS
    let state_class = match current_mode {
        AppMode::Recording => "state-recording",
        AppMode::Review | AppMode::Exporting => "state-review",
        // FIX: Explicit match (clippy::match_wildcard_for_single_variants)
        AppMode::Idle => "state-idle",
    };

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            class: "app-frame {state_class}",

            // 1. Resize Handles (Only when not recording)
            if current_mode == AppMode::Idle {
                ResizeHandles {}
            }

            // 2. Drag Header (Invisible but usable area at top)
            if current_mode == AppMode::Idle {
                div {
                    class: "drag-header",
                    onmousedown: move |_| drag_win.drag(),
                    // Optional: Visual indicator for drag area if needed
                }
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
