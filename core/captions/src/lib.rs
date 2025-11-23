//! Caption rendering module
//!
//! Phase 1: Generates ffmpeg drawtext filter strings.

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::many_single_char_names)]

use anyhow::Result;
use mandygif_protocol::Caption;

/// Generate ffmpeg drawtext filter for a caption (Phase 1)
///
/// # Errors
/// Returns error if color parsing fails.
pub fn ffmpeg_text(caption: &Caption, w: u32, h: u32) -> Result<String> {
    let x = (caption.rect.x * w as f32) as u32;
    let y = (caption.rect.y * h as f32) as u32;

    // Validate colors before generating string
    let font_color = ff_color(&caption.style.color, 1.0)?;
    let border_color = ff_color(&caption.style.stroke, 1.0)?;

    Ok(format!(
        "drawtext=text='{}':fontfile=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf:fontsize={}:fontcolor={}:borderw=2:bordercolor={}:x={}:y={}:enable='between(t,{},{})'",
        caption.text.replace('\'', "\\'"),
        caption.style.size,
        font_color,
        border_color,
        x,
        y,
        caption.start_ms as f64 / 1000.0,
        caption.end_ms as f64 / 1000.0
    ))
}

/// Combine multiple captions into a filter chain
///
/// # Errors
/// Returns error if any caption fails to generate (e.g. bad color).
pub fn chain_filters(captions: &[Caption], w: u32, h: u32) -> Result<String> {
    let mut filters = Vec::new();
    for c in captions {
        filters.push(ffmpeg_text(c, w, h)?);
    }
    Ok(filters.join(","))
}

/// Generate ffmpeg drawtext filter using expressions (`main_w/main_h`)
/// Works correctly after scaling operations.
///
/// # Errors
/// Returns error if color parsing fails.
pub fn ffmpeg_text_expr(caption: &Caption) -> Result<String> {
    let x_expr = format!("(main_w*{:.6})", caption.rect.x);
    let y_expr = format!("(main_h*{:.6})", caption.rect.y);
    let start_s = (caption.start_ms as f64) / 1000.0;
    let end_s = (caption.end_ms as f64) / 1000.0;

    let fontcolor = ff_color(&caption.style.color, 1.0)?;
    let bordercolor = ff_color(&caption.style.stroke, 1.0)?;

    Ok(format!(
        "drawtext=text='{}':fontfile=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf:fontsize={}:fontcolor={}:borderw=2:bordercolor={}:x={}:y={}:enable='between(t,{:.3},{:.3})'",
        caption.text.replace('\'', "\\'"),
        caption.style.size,
        fontcolor,
        bordercolor,
        x_expr,
        y_expr,
        start_s,
        end_s
    ))
}

/// Combine multiple captions using expression-based positioning.
///
/// # Errors
/// Returns error if any caption fails to generate.
pub fn chain_filters_expr(captions: &[Caption]) -> Result<String> {
    let mut filters = Vec::new();
    for c in captions {
        filters.push(ffmpeg_text_expr(c)?);
    }
    Ok(filters.join(","))
}

/// Normalize CSS-like hex colors to ffmpeg syntax.
/// #RGB/#RRGGBB/#RGBA/#RRGGBBAA -> 0xRRGGBB or 0xRRGGBB@A.A
#[allow(clippy::cast_lossless)]
fn ff_color(input: &str, default_alpha: f32) -> Result<String> {
    let s = input.trim();
    if s.starts_with("0x") || s.contains('@') {
        return Ok(s.to_string());
    }

    let hex = s.strip_prefix('#').unwrap_or(s);
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
            (r, g, b, (default_alpha * 255.0).round() as u8)
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)?;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16)?;
            (r, g, b, (f32::from(a) / 15.0 * 255.0).round() as u8)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            (r, g, b, (default_alpha * 255.0).round() as u8)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            let a = u8::from_str_radix(&hex[6..8], 16)?;
            (r, g, b, a)
        }
        _ => return Err(anyhow::anyhow!("Invalid hex color format: {s}")),
    };

    if a == 255 {
        Ok(format!("0x{r:02X}{g:02X}{b:02X}"))
    } else {
        let alpha = f32::from(a) / 255.0;
        Ok(format!("0x{r:02X}{g:02X}{b:02X}@{alpha:.3}"))
    }
}
