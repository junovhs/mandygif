//! Caption rendering module
//! 
//! Phase 1: Generates ffmpeg drawtext filter strings
//! Phase 2: Direct rendering with skia-safe (SkParagraph)

use mandygif_protocol::Caption;
use anyhow::Result;

/// Generate ffmpeg drawtext filter for a caption (Phase 1)
pub fn to_ffmpeg_drawtext(caption: &Caption, video_width: u32, video_height: u32) -> String {
    let x = (caption.rect.x * video_width as f32) as u32;
    let y = (caption.rect.y * video_height as f32) as u32;
    
    // Basic drawtext filter with timing
    format!(
        "drawtext=text='{}':fontfile=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf:fontsize={}:fontcolor={}:borderw=2:bordercolor={}:x={}:y={}:enable='between(t,{},{})'",
        caption.text.replace('\'', "\\'"),
        caption.style.size,
        caption.style.color,
        caption.style.stroke,
        x,
        y,
        caption.start_ms as f64 / 1000.0,
        caption.end_ms as f64 / 1000.0
    )
}

/// Combine multiple captions into a filter chain
pub fn build_filter_chain(captions: &[Caption], video_width: u32, video_height: u32) -> String {
    captions
        .iter()
        .map(|c| to_ffmpeg_drawtext(c, video_width, video_height))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mandygif_protocol::{CaptionStyle, CaptionRect, CaptionAnimation};

    #[test]
    fn test_drawtext_generation() {
        let caption = Caption {
            text: "Hello World".into(),
            font: "System".into(),
            style: CaptionStyle {
                color: "#ffffff".into(),
                stroke: "#000000".into(),
                size: 24,
            },
            rect: CaptionRect { x: 0.1, y: 0.8, w: 0.8, h: 0.1 },
            start_ms: 1000,
            end_ms: 3000,
            animation: CaptionAnimation::None,
        };
        
        let filter = to_ffmpeg_drawtext(&caption, 1920, 1080);
        assert!(filter.contains("Hello World"));
        assert!(filter.contains("fontsize=24"));
        assert!(filter.contains("enable='between(t,1,3)'"));
    }
}