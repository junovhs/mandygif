// warden:ignore
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
                class: "resize-zone {class}",
                style: match dir {
                    ResizeDirection::North => "top: 0; left: 10px; right: 10px; height: 6px; cursor: n-resize;",
                    ResizeDirection::South => "bottom: 0; left: 10px; right: 10px; height: 6px; cursor: s-resize;",
                    ResizeDirection::East => "top: 10px; bottom: 10px; right: 0; width: 6px; cursor: e-resize;",
                    ResizeDirection::West => "top: 10px; bottom: 10px; left: 0; width: 6px; cursor: w-resize;",
                    ResizeDirection::NorthWest => "top: 0; left: 0; width: 10px; height: 10px; cursor: nw-resize;",
                    ResizeDirection::NorthEast => "top: 0; right: 0; width: 10px; height: 10px; cursor: ne-resize;",
                    ResizeDirection::SouthWest => "bottom: 0; left: 0; width: 10px; height: 10px; cursor: sw-resize;",
                    ResizeDirection::SouthEast => "bottom: 0; right: 0; width: 10px; height: 10px; cursor: se-resize;",
                },
                onmousedown: move |_| { let _ = w.drag_resize_window(dir); },
            }
        }
    };

    rsx! {
        // Corners
        {make_handle(ResizeDirection::NorthWest, "")}
        {make_handle(ResizeDirection::NorthEast, "")}
        {make_handle(ResizeDirection::SouthWest, "")}
        {make_handle(ResizeDirection::SouthEast, "")}

        // Sides
        {make_handle(ResizeDirection::North, "")}
        {make_handle(ResizeDirection::South, "")}
        {make_handle(ResizeDirection::West, "")}
        {make_handle(ResizeDirection::East, "")}
    }
}
