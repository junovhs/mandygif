use dioxus::desktop::tao::window::ResizeDirection;
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[component]
pub fn ResizeHandles() -> Element {
    let window = use_window();

    // Helper to generate a handle div
    let make_handle = move |dir: ResizeDirection, class: &'static str| {
        let w = window.clone();
        rsx! {
            div {
                class: "{class}",
                // FIX: Explicitly ignore result to return unit type ()
                onmousedown: move |_| { let _ = w.drag_resize_window(dir); },
            }
        }
    };

    rsx! {
        // Corners
        {make_handle(ResizeDirection::NorthWest, "handle-nw")}
        {make_handle(ResizeDirection::NorthEast, "handle-ne")}
        {make_handle(ResizeDirection::SouthWest, "handle-sw")}
        {make_handle(ResizeDirection::SouthEast, "handle-se")}

        // Sides
        {make_handle(ResizeDirection::North, "handle-n")}
        {make_handle(ResizeDirection::South, "handle-s")}
        {make_handle(ResizeDirection::West, "handle-w")}
        {make_handle(ResizeDirection::East, "handle-e")}
    }
}
