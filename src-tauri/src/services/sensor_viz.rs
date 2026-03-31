use crate::models::manifest::Component;
use crate::services::sensor_state::SensorStateStore;
use image::{Rgba, RgbaImage};
use std::io::Cursor;

const BG: Rgba<u8> = Rgba([20, 20, 25, 255]);
const CYAN: Rgba<u8> = Rgba([100, 220, 255, 255]);
const AMBER: Rgba<u8> = Rgba([239, 159, 39, 255]);
const GREEN: Rgba<u8> = Rgba([93, 202, 165, 255]);
const RED: Rgba<u8> = Rgba([224, 75, 74, 255]);
const DIM: Rgba<u8> = Rgba([40, 40, 50, 255]);
const GRID: Rgba<u8> = Rgba([50, 50, 60, 255]);

pub struct SensorVizRenderer;

impl SensorVizRenderer {
    /// Render all spatial sensors into a single composite PNG, or None if no spatial sensors.
    pub fn render(state: &SensorStateStore) -> Option<Vec<u8>> {
        let spatial: Vec<&Component> = state
            .components()
            .iter()
            .filter(|c| is_spatial_sensor(&c.component_type))
            .collect();

        if spatial.is_empty() {
            return None;
        }

        let mut images: Vec<RgbaImage> = Vec::new();

        for comp in &spatial {
            match comp.component_type.as_str() {
                "line_sensor_array" => {
                    if let Some(reading) = state.get_latest(&comp.id) {
                        images.push(render_line_sensor(&reading.raw_value));
                    }
                }
                "encoder" => {
                    // Collect all encoder readings for drift comparison
                }
                "imu" => {
                    if let Some(history) = state.get_history(&comp.id) {
                        let vals: Vec<&[f64]> =
                            history.iter().map(|r| r.values.as_slice()).collect();
                        if !vals.is_empty() {
                            images.push(render_imu_timeseries(&vals));
                        }
                    }
                }
                _ => {}
            }
        }

        // Render encoder drift if we have encoder pairs
        let encoders: Vec<&Component> = spatial
            .iter()
            .filter(|c| c.component_type == "encoder")
            .copied()
            .collect();
        if encoders.len() >= 2 {
            let values: Vec<(String, f64)> = encoders
                .iter()
                .filter_map(|c| {
                    state
                        .get_latest(&c.id)
                        .map(|r| (c.label.clone(), *r.values.first().unwrap_or(&0.0)))
                })
                .collect();
            if !values.is_empty() {
                images.push(render_encoder_drift(&values));
            }
        }

        if images.is_empty() {
            return None;
        }

        // Stack images vertically with padding
        let padding = 8u32;
        let total_width = images.iter().map(|img| img.width()).max().unwrap_or(200);
        let total_height: u32 =
            images.iter().map(|img| img.height()).sum::<u32>() + padding * (images.len() as u32);

        let mut composite =
            RgbaImage::from_pixel(total_width, total_height.max(1), BG);
        let mut y_offset = 0u32;

        for img in &images {
            let x_offset = (total_width.saturating_sub(img.width())) / 2;
            for y in 0..img.height() {
                for x in 0..img.width() {
                    if x_offset + x < total_width && y_offset + y < total_height {
                        composite.put_pixel(x_offset + x, y_offset + y, *img.get_pixel(x, y));
                    }
                }
            }
            y_offset += img.height() + padding;
        }

        encode_png(&composite)
    }
}

fn is_spatial_sensor(component_type: &str) -> bool {
    matches!(
        component_type,
        "line_sensor_array" | "imu" | "encoder"
    )
}

/// Line sensor array: row of colored rectangles.
fn render_line_sensor(raw_value: &str) -> RgbaImage {
    let bits: Vec<bool> = raw_value.chars().map(|c| c == '1').collect();
    let count = bits.len().max(1) as u32;
    let cell_w = 24u32;
    let cell_h = 24u32;
    let gap = 3u32;
    let pad = 6u32;
    let width = count * cell_w + (count - 1) * gap + 2 * pad;
    let height = cell_h + 2 * pad;

    let mut img = RgbaImage::from_pixel(width, height, BG);

    for (i, &active) in bits.iter().enumerate() {
        let x = pad + i as u32 * (cell_w + gap);
        let color = if active { CYAN } else { DIM };
        fill_rect(&mut img, x, pad, cell_w, cell_h, color);
    }

    img
}

/// IMU time-series: line chart of X/Y/Z over recent readings.
fn render_imu_timeseries(history: &[&[f64]]) -> RgbaImage {
    let w = 300u32;
    let h = 150u32;
    let pad = 10u32;
    let mut img = RgbaImage::from_pixel(w, h, BG);

    // Draw grid lines
    for y in (pad..h - pad).step_by(20) {
        for x in pad..w - pad {
            img.put_pixel(x, y, GRID);
        }
    }

    let plot_w = (w - 2 * pad) as f64;
    let plot_h = (h - 2 * pad) as f64;
    let n = history.len();
    if n < 2 {
        return img;
    }

    // Find value range across all axes
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;
    for vals in history {
        for &v in *vals {
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }
    }
    let range = (max_val - min_val).max(0.01);

    let colors = [CYAN, AMBER, GREEN];

    // Draw each axis as a line
    let max_axes = history.iter().map(|v| v.len()).max().unwrap_or(0).min(3);
    for axis in 0..max_axes {
        let color = colors[axis % colors.len()];
        let mut prev: Option<(u32, u32)> = None;

        for (i, vals) in history.iter().enumerate() {
            if let Some(&v) = vals.get(axis) {
                let px = pad + ((i as f64 / (n - 1) as f64) * plot_w) as u32;
                let py = pad + (((max_val - v) / range) * plot_h) as u32;
                let px = px.min(w - 1);
                let py = py.min(h - 1);

                if let Some((px0, py0)) = prev {
                    draw_line(&mut img, px0, py0, px, py, color);
                }
                prev = Some((px, py));
            }
        }
    }

    img
}

/// Encoder drift: two vertical bars side by side.
fn render_encoder_drift(encoders: &[(String, f64)]) -> RgbaImage {
    let bar_w = 40u32;
    let gap = 20u32;
    let pad = 10u32;
    let max_h = 120u32;
    let count = encoders.len() as u32;
    let w = count * bar_w + (count - 1) * gap + 2 * pad;
    let h = max_h + 2 * pad;

    let mut img = RgbaImage::from_pixel(w, h, BG);

    let max_val = encoders
        .iter()
        .map(|(_, v)| v.abs())
        .fold(1.0f64, f64::max);

    let bar_colors = [CYAN, AMBER, GREEN, RED];

    for (i, (_, val)) in encoders.iter().enumerate() {
        let bar_h = ((val.abs() / max_val) * max_h as f64) as u32;
        let bar_h = bar_h.max(2);
        let x = pad + i as u32 * (bar_w + gap);
        let y = pad + max_h - bar_h;
        let color = bar_colors[i % bar_colors.len()];
        fill_rect(&mut img, x, y, bar_w, bar_h, color);
    }

    img
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, color);
            }
        }
    }
}

/// Bresenham's line drawing.
fn draw_line(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: Rgba<u8>) {
    let (mut x0, mut y0) = (x0 as i32, y0 as i32);
    let (x1, y1) = (x1 as i32, y1 as i32);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && y0 >= 0 && (x0 as u32) < img.width() && (y0 as u32) < img.height() {
            img.put_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn encode_png(img: &RgbaImage) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .ok()?;
    Some(buf)
}
