use dioxus::desktop::use_window;
use dioxus::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct Region {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

pub fn App() -> Element {
    let window = use_window();
    let region = use_signal(|| Region {
        x: 100,
        y: 100,
        width: 800,
        height: 600,
    });

    // FIX: Clone window for confirm callback
    let window_confirm = window.clone();
    let confirm = move |_| {
        let r = region.read();
        if let Ok(json) = serde_json::to_string(&*r) {
            println!("{json}");
        }
        window_confirm.close();
    };

    // FIX: Clone window for drag callback
    let window_drag = window.clone();

    rsx! {
        div {
            style: "width: 100vw; height: 100vh; background: transparent;",
            div {
                style: "position: absolute; left: {region.read().x}px; top: {region.read().y}px; width: {region.read().width}px; height: {region.read().height}px; border: 2px dashed #00ff00; background: rgba(0, 255, 0, 0.1);",

                // Drag Handle
                div {
                    // FIX: Use cloned handle
                    onmousedown: move |_| window_drag.drag(),
                    style: "width: 100%; height: 30px; background: rgba(0,0,0,0.5); cursor: move; color: white;",
                    "Select Region"
                }

                // Confirm Button
                button {
                    onclick: confirm,
                    style: "position: absolute; bottom: -40px; left: 0; background: #00ff00; padding: 5px 10px;",
                    "Confirm"
                }
            }
        }
    }
}
