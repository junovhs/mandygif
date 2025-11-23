//! Region selector - transparent overlay for capture area selection

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use slint::ComponentHandle;
use std::io::Write;

slint::include_modules!();

#[derive(Debug, Serialize, Deserialize)]
struct Region {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn main() -> Result<()> {
    let selector = RegionSelector::new()?;

    // Start at reasonable centered size
    selector.set_sel_x(320.0);
    selector.set_sel_y(180.0);
    selector.set_sel_width(1280.0);
    selector.set_sel_height(720.0);

    let selector_weak = selector.as_weak();
    selector.on_confirm(move || {
        if let Some(s) = selector_weak.upgrade() {
            let region = Region {
                x: s.get_sel_x() as i32,
                y: s.get_sel_y() as i32,
                width: s.get_sel_width() as u32,
                height: s.get_sel_height() as u32,
            };

            // Output as JSON
            if let Ok(json) = serde_json::to_string(&region) {
                println!("{json}");
                let _ = std::io::stdout().flush();
            }

            slint::quit_event_loop().ok();
        }
    });

    selector.on_cancel(|| {
        std::process::exit(1);
    });

    selector.run()?;

    Ok(())
}
