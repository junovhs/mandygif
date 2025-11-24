//! `MandyGIF` - Dioxus UI

#![allow(non_snake_case)]

mod app;
mod hooks;
mod components {
    pub mod control_bar;
    pub mod resize_handle;
}
mod processes;
mod state;

use app::App;
use dioxus::desktop::tao::dpi::PhysicalSize;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");

    let cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("MandyGIF")
                .with_transparent(true)
                .with_decorations(false)
                .with_always_on_top(true)
                .with_maximized(false)
                .with_resizable(true)
                // FIX: Use PhysicalSize for exact pixel control on 4K
                .with_min_inner_size(PhysicalSize::new(300, 150))
                .with_inner_size(PhysicalSize::new(800, 600)),
        )
        .with_background_color((0, 0, 0, 0));

    LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}
