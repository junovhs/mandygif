//! Region Selector - Dioxus Implementation

#![allow(non_snake_case)]

mod app;

use app::App;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    // Transparent fullscreen window for overlay
    let cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("MandyGIF Region Selector")
                .with_transparent(true)
                .with_decorations(false)
                .with_always_on_top(true)
                .with_maximized(true),
        )
        .with_background_color((0, 0, 0, 0));

    LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}
