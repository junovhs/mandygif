//! `MandyGIF` - Dioxus UI

#![allow(non_snake_case)]

mod app;
mod components; // Loads ui/src/components.rs
mod hooks;
mod processes;
mod state;

use app::App;
use dioxus::desktop::tao::dpi::LogicalSize;
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
                .with_inner_size(LogicalSize::new(800.0, 600.0)),
        )
        .with_background_color((0, 0, 0, 0));

    LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}
