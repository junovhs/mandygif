//! Region selector - transparent overlay for capture area selection

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
    
    // Default to center of screen
    selector.set_region_x(100.0);
    selector.set_region_y(100.0);
    selector.set_region_width(640.0);
    selector.set_region_height(480.0);
    
    let selector_weak = selector.as_weak();
    selector.on_confirm(move || {
        if let Some(s) = selector_weak.upgrade() {
            let region = Region {
                x: s.get_region_x() as i32,
                y: s.get_region_y() as i32,
                width: s.get_region_width() as u32,
                height: s.get_region_height() as u32,
            };
            
            // Output as JSON
            if let Ok(json) = serde_json::to_string(&region) {
                println!("{}", json);
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