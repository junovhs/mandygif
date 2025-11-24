// warden:ignore
use dioxus::desktop::tao::window::ResizeDirection;
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[component]
pub fn ResizeHandles() -> Element {
    let window = use_window();

    // Handles are 32px
    // Center offset = -16px (half of 32)
    // This places the center of the handle exactly on the window edge

    let make_handle = move |dir: ResizeDirection, style: &'static str, is_corner: bool| {
        let w = window.clone();

        let corner_vis = if is_corner {
            rsx! { div { class: "resize-corner-vis", style: match dir {
                ResizeDirection::NorthWest => "border-right: none; border-bottom: none;",
                ResizeDirection::NorthEast => "border-left: none; border-bottom: none;",
                ResizeDirection::SouthWest => "border-right: none; border-top: none;",
                ResizeDirection::SouthEast => "border-left: none; border-top: none;",
                _ => ""
            }}}
        } else {
            rsx! {}
        };

        rsx! {
            div {
                class: "resize-zone",
                style: "{style}",
                onmousedown: move |_| { let _ = w.drag_resize_window(dir); },
                {corner_vis}
            }
        }
    };

    rsx! {
        // Corners: Centered on the point (-16px)
        {make_handle(ResizeDirection::NorthWest, "top: -16px; left: -16px; width: 32px; height: 32px; cursor: nw-resize;", true)}
        {make_handle(ResizeDirection::NorthEast, "top: -16px; right: -16px; width: 32px; height: 32px; cursor: ne-resize;", true)}
        {make_handle(ResizeDirection::SouthWest, "bottom: -16px; left: -16px; width: 32px; height: 32px; cursor: sw-resize;", true)}
        {make_handle(ResizeDirection::SouthEast, "bottom: -16px; right: -16px; width: 32px; height: 32px; cursor: se-resize;", true)}

        // Sides: Overlap slightly, centered on edges
        {make_handle(ResizeDirection::North, "top: -8px; left: 16px; right: 16px; height: 16px; cursor: n-resize;", false)}
        {make_handle(ResizeDirection::South, "bottom: -8px; left: 16px; right: 16px; height: 16px; cursor: s-resize;", false)}
        {make_handle(ResizeDirection::West, "top: 16px; bottom: 16px; left: -8px; width: 16px; cursor: w-resize;", false)}
        {make_handle(ResizeDirection::East, "top: 16px; bottom: 16px; right: -8px; width: 16px; cursor: e-resize;", false)}
    }
}
