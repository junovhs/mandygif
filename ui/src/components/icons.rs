// warden:ignore
use dioxus::prelude::*;

pub fn IconStop() -> Element {
    rsx! {
        div {
            style: "width: 12px; height: 12px; background: currentColor; border-radius: 2px;"
        }
    }
}

pub fn IconExport() -> Element {
    rsx! {
        svg {
            width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { points: "17 8 12 3 7 8" }
            line { x1: "12", y1: "3", x2: "12", y2: "15" }
        }
    }
}
