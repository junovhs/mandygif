// warden:ignore
use dioxus::desktop::tao::window::ResizeDirection;
use dioxus::desktop::use_window;
use dioxus::prelude::*;

#[component]
pub fn ResizeHandles() -> Element {
    let window = use_window();

    let make_handle = move |dir: ResizeDirection, style: &'static str, is_corner: bool| {
        let w = window.clone();

        // CSS classes for visuals
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
                // CRITICAL: Ensure we can actually click them
                onmousedown: move |_| { let _ = w.drag_resize_window(dir); },
                {corner_vis}
            }
        }
    };

    // Hit zones are now 16px thick for easier grabbing
    rsx! {
        // Corners (Visible Brackets)
        {make_handle(ResizeDirection::NorthWest, "top: 0; left: 0; width: 24px; height: 24px; cursor: nw-resize;", true)}
        {make_handle(ResizeDirection::NorthEast, "top: 0; right: 0; width: 24px; height: 24px; cursor: ne-resize;", true)}
        {make_handle(ResizeDirection::SouthWest, "bottom: 0; left: 0; width: 24px; height: 24px; cursor: sw-resize;", true)}
        {make_handle(ResizeDirection::SouthEast, "bottom: 0; right: 0; width: 24px; height: 24px; cursor: se-resize;", true)}

        // Sides (Invisible but thick)
        {make_handle(ResizeDirection::North, "top: 0; left: 24px; right: 24px; height: 12px; cursor: n-resize;", false)}
        {make_handle(ResizeDirection::South, "bottom: 0; left: 24px; right: 24px; height: 12px; cursor: s-resize;", false)}
        {make_handle(ResizeDirection::West, "top: 24px; bottom: 24px; left: 0; width: 12px; cursor: w-resize;", false)}
        {make_handle(ResizeDirection::East, "top: 24px; bottom: 24px; right: 0; width: 12px; cursor: e-resize;", false)}
    }
}
