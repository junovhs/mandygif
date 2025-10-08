//! Caption rendering module
//! 
//! Phase 1: Generates ffmpeg drawtext filter strings
//! Phase 2: Direct rendering with skia-safe (SkParagraph)

use mandygif_protocol::Caption;

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

/// Generate ffmpeg drawtext filter using expressions (main_w/main_h)
/// Works correctly after scaling operations
pub fn to_ffmpeg_drawtext_expr(caption: &Caption) -> String {
    let x_expr = format!("(main_w*{:.6})", caption.rect.x);
    let y_expr = format!("(main_h*{:.6})", caption.rect.y);
    let start_s = (caption.start_ms as f64) / 1000.0;
    let end_s = (caption.end_ms as f64) / 1000.0;
    let fontcolor = ff_color(&caption.style.color, 1.0);
    let bordercolor = ff_color(&caption.style.stroke, 1.0);
    
    format!(
        "drawtext=text='{}':fontfile=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf:fontsize={}:fontcolor={}:borderw=2:bordercolor={}:x={}:y={}:enable='between(t,{:.3},{:.3})'",
        caption.text.replace('\'', "\\'"),
        caption.style.size,
        fontcolor,
        bordercolor,
        x_expr,
        y_expr,
        start_s,
        end_s
    )
}

/// Combine multiple captions using expression-based positioning
pub fn build_filter_chain_expr(captions: &[Caption]) -> String {
    captions
        .iter()
        .map(to_ffmpeg_drawtext_expr)
        .collect::<Vec<_>>()
        .join(",")
}

/// Normalize CSS-like hex colors to ffmpeg syntax
/// #RGB/#RRGGBB/#RGBA/#RRGGBBAA → 0xRRGGBB or 0xRRGGBB@A.A
fn ff_color(input: &str, default_alpha: f32) -> String {
    let s = input.trim();
    if s.starts_with("0x") || s.contains('@') {
        return s.to_string();
    }
    
    let (r, g, b, a) = if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
                (r, g, b, (default_alpha * 255.0).round() as u8)
            }
            4 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).unwrap_or(255);
                (r, g, b, a)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b, (default_alpha * 255.0).round() as u8)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
                (r, g, b, a)
            }
            _ => (255, 255, 255, (default_alpha * 255.0).round() as u8),
        }
    } else {
        return s.to_string();
    };
    
    if a == 255 {
        format!("0x{:02X}{:02X}{:02X}", r, g, b)
    } else {
        let alpha = (a as f32) / 255.0;
        format!("0x{:02X}{:02X}{:02X}@{:.3}", r, g, b, alpha)
    }
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
    
    #[test]
    fn test_expr_generation() {
        let caption = Caption {
            text: "Hi".into(),
            font: "System".into(),
            style: CaptionStyle {
                color: "#fff".into(),
                stroke: "#0008".into(),
                size: 20,
            },
            rect: CaptionRect { x: 0.25, y: 0.75, w: 0.0, h: 0.0 },
            start_ms: 0,
            end_ms: 1500,
            animation: CaptionAnimation::None,
        };
        
        let f = to_ffmpeg_drawtext_expr(&caption);
        assert!(f.contains("x=(main_w*0.250000)"));
        assert!(f.contains("y=(main_h*0.750000)"));
        assert!(f.contains("fontsize=20"));
        assert!(f.contains("enable='between(t,0.000,1.500)'"));
    }
}