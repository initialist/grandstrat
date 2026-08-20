use crate::parser::{parse_svg_commands, RawPath};
use rayon::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub const MAP_WIDTH: usize = 9600;
pub const MAP_HEIGHT: usize = 5440;
pub const WORLD_WIDTH: f32 = 1200.0;
pub const WORLD_HEIGHT: f32 = 680.0;

#[derive(Clone, Copy, Debug)]
struct LineSegment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

pub struct RasterizedMap {
    pub width: usize,
    pub height: usize,
    pub id_buffer: Vec<u32>, // 1-based province IDs (0 = ocean)
    pub rgba_texture_data: Vec<u8>, // RGBA texture bytes for OpenGL
}

pub fn get_or_build_id_map(raw_paths: &[RawPath]) -> RasterizedMap {
    let cache_path = Path::new("provinces_cache.bin");
    let expected_len = MAP_WIDTH * MAP_HEIGHT * 4;

    if cache_path.exists() {
        if let Ok(mut file) = File::open(cache_path) {
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() && buffer.len() == expected_len {
                let id_buffer: Vec<u32> = buffer
                    .chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();

                let mut non_zero_count = 0;
                for &id in &id_buffer {
                    if id > 0 {
                        non_zero_count += 1;
                    }
                }

                if non_zero_count > 1_000_000 {
                    println!("Loaded lossless province ID map from cache (9600x5440, {} land pixels)", non_zero_count);
                    let mut rgba_texture_data = vec![0u8; MAP_WIDTH * MAP_HEIGHT * 4];
                    for i in 0..id_buffer.len() {
                        let id = id_buffer[i];
                        let r = ((id >> 16) & 0xFF) as u8;
                        let g = ((id >> 8) & 0xFF) as u8;
                        let b = (id & 0xFF) as u8;
                        let idx = i * 4;
                        rgba_texture_data[idx] = r;
                        rgba_texture_data[idx + 1] = g;
                        rgba_texture_data[idx + 2] = b;
                        rgba_texture_data[idx + 3] = if id > 0 { 255 } else { 0 };
                    }

                    return RasterizedMap {
                        width: MAP_WIDTH,
                        height: MAP_HEIGHT,
                        id_buffer,
                        rgba_texture_data,
                    };
                }
            }
        }
    }

    println!("Rasterizing 22,711 provinces at 9600x5440 (8x lossless density) in parallel using Rayon...");
    let scale_x = MAP_WIDTH as f32 / WORLD_WIDTH;
    let scale_y = MAP_HEIGHT as f32 / WORLD_HEIGHT;

    let province_spans: Vec<(u32, Vec<(usize, usize, usize)>)> = raw_paths
        .par_iter()
        .enumerate()
        .map(|(index, p)| {
            let id = (index + 1) as u32;
            let segments = parse_path_to_segments(&p.d, scale_x, scale_y);
            let spans = rasterize_segments_to_spans(&segments, MAP_WIDTH, MAP_HEIGHT);
            (id, spans)
        })
        .collect();

    let mut id_buffer = vec![0u32; MAP_WIDTH * MAP_HEIGHT];
    let mut rgba_texture_data = vec![0u8; MAP_WIDTH * MAP_HEIGHT * 4];
    let mut total_filled = 0;

    for (id, spans) in province_spans {
        let r = ((id >> 16) & 0xFF) as u8;
        let g = ((id >> 8) & 0xFF) as u8;
        let b = (id & 0xFF) as u8;

        for (y, x_start, x_end) in spans {
            if y < MAP_HEIGHT {
                let row_offset = y * MAP_WIDTH;
                for x in x_start..=x_end.min(MAP_WIDTH - 1) {
                    let idx = row_offset + x;
                    id_buffer[idx] = id;
                    let tex_idx = idx * 4;
                    rgba_texture_data[tex_idx] = r;
                    rgba_texture_data[tex_idx + 1] = g;
                    rgba_texture_data[tex_idx + 2] = b;
                    rgba_texture_data[tex_idx + 3] = 255;
                    total_filled += 1;
                }
            }
        }
    }

    println!("9600x5440 Rasterization complete: filled {} land pixels", total_filled);

    // Save to disk cache
    if let Ok(mut file) = File::create(cache_path) {
        let mut byte_buffer = Vec::with_capacity(id_buffer.len() * 4);
        for &id in &id_buffer {
            byte_buffer.extend_from_slice(&id.to_le_bytes());
        }
        let _ = file.write_all(&byte_buffer);
        println!("Saved 9600x5440 province ID map cache (provinces_cache.bin)");
    }

    RasterizedMap {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        id_buffer,
        rgba_texture_data,
    }
}

fn parse_path_to_segments(d: &str, scale_x: f32, scale_y: f32) -> Vec<LineSegment> {
    let commands = parse_svg_commands(d);
    let mut segments = Vec::new();
    let mut cur_x = 0.0f32;
    let mut cur_y = 0.0f32;
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;
    let mut has_moved = false;

    for cmd in commands {
        let is_rel = cmd.cmd.is_ascii_lowercase();
        let upper = cmd.cmd.to_ascii_uppercase();
        let nums = &cmd.params;
        let mut idx = 0;

        match upper {
            'M' => {
                // Auto-close previous sub-path if open
                if has_moved && ((cur_x - start_x).abs() > 0.001 || (cur_y - start_y).abs() > 0.001) {
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: start_x * scale_x,
                        y2: start_y * scale_y,
                    });
                }

                if idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    cur_x = x;
                    cur_y = y;
                    start_x = x;
                    start_y = y;
                    has_moved = true;
                    idx += 2;
                }
                while idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: x * scale_x,
                        y2: y * scale_y,
                    });
                    cur_x = x;
                    cur_y = y;
                    idx += 2;
                }
            }
            'L' => {
                while idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: x * scale_x,
                        y2: y * scale_y,
                    });
                    cur_x = x;
                    cur_y = y;
                    idx += 2;
                }
            }
            'H' => {
                while idx < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: x * scale_x,
                        y2: cur_y * scale_y,
                    });
                    cur_x = x;
                    idx += 1;
                }
            }
            'V' => {
                while idx < nums.len() {
                    let y = if is_rel { cur_y + nums[idx] } else { nums[idx] };
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: cur_x * scale_x,
                        y2: y * scale_y,
                    });
                    cur_y = y;
                    idx += 1;
                }
            }
            'C' => {
                while idx + 5 < nums.len() {
                    let p0 = (cur_x, cur_y);
                    let p1 = if is_rel { (cur_x + nums[idx], cur_y + nums[idx + 1]) } else { (nums[idx], nums[idx + 1]) };
                    let p2 = if is_rel { (cur_x + nums[idx + 2], cur_y + nums[idx + 3]) } else { (nums[idx + 2], nums[idx + 3]) };
                    let p3 = if is_rel { (cur_x + nums[idx + 4], cur_y + nums[idx + 5]) } else { (nums[idx + 4], nums[idx + 5]) };

                    let steps = 5;
                    let mut prev_pt = p0;
                    for s in 1..=steps {
                        let t = s as f32 / steps as f32;
                        let it = 1.0 - t;
                        let qx = it * it * it * p0.0 + 3.0 * it * it * t * p1.0 + 3.0 * it * t * t * p2.0 + t * t * t * p3.0;
                        let qy = it * it * it * p0.1 + 3.0 * it * it * t * p1.1 + 3.0 * it * t * t * p2.1 + t * t * t * p3.1;
                        segments.push(LineSegment {
                            x1: prev_pt.0 * scale_x,
                            y1: prev_pt.1 * scale_y,
                            x2: qx * scale_x,
                            y2: qy * scale_y,
                        });
                        prev_pt = (qx, qy);
                    }

                    cur_x = p3.0;
                    cur_y = p3.1;
                    idx += 6;
                }
            }
            'S' | 'Q' => {
                while idx + 3 < nums.len() {
                    let p0 = (cur_x, cur_y);
                    let p1 = if is_rel { (cur_x + nums[idx], cur_y + nums[idx + 1]) } else { (nums[idx], nums[idx + 1]) };
                    let p2 = if is_rel { (cur_x + nums[idx + 2], cur_y + nums[idx + 3]) } else { (nums[idx + 2], nums[idx + 3]) };

                    let steps = 4;
                    let mut prev_pt = p0;
                    for s in 1..=steps {
                        let t = s as f32 / steps as f32;
                        let it = 1.0 - t;
                        let qx = it * it * p0.0 + 2.0 * it * t * p1.0 + t * t * p2.0;
                        let qy = it * it * p0.1 + 2.0 * it * t * p1.1 + t * t * p2.1;
                        segments.push(LineSegment {
                            x1: prev_pt.0 * scale_x,
                            y1: prev_pt.1 * scale_y,
                            x2: qx * scale_x,
                            y2: qy * scale_y,
                        });
                        prev_pt = (qx, qy);
                    }

                    cur_x = p2.0;
                    cur_y = p2.1;
                    idx += 4;
                }
            }
            'A' => {
                while idx + 6 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx + 5] } else { nums[idx + 5] };
                    let y = if is_rel { cur_y + nums[idx + 6] } else { nums[idx + 6] };
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: x * scale_x,
                        y2: y * scale_y,
                    });
                    cur_x = x;
                    cur_y = y;
                    idx += 7;
                }
            }
            'Z' => {
                if (cur_x - start_x).abs() > 0.001 || (cur_y - start_y).abs() > 0.001 {
                    segments.push(LineSegment {
                        x1: cur_x * scale_x,
                        y1: cur_y * scale_y,
                        x2: start_x * scale_x,
                        y2: start_y * scale_y,
                    });
                }
                cur_x = start_x;
                cur_y = start_y;
            }
            _ => {}
        }
    }

    // Auto-close final subpath if open
    if has_moved && ((cur_x - start_x).abs() > 0.001 || (cur_y - start_y).abs() > 0.001) {
        segments.push(LineSegment {
            x1: cur_x * scale_x,
            y1: cur_y * scale_y,
            x2: start_x * scale_x,
            y2: start_y * scale_y,
        });
    }

    segments
}

fn rasterize_segments_to_spans(
    segments: &[LineSegment],
    width: usize,
    height: usize,
) -> Vec<(usize, usize, usize)> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for seg in segments {
        let sx_min = seg.x1.min(seg.x2);
        let sx_max = seg.x1.max(seg.x2);
        let sy_min = seg.y1.min(seg.y2);
        let sy_max = seg.y1.max(seg.y2);

        if sx_min < min_x { min_x = sx_min; }
        if sx_max > max_x { max_x = sx_max; }
        if sy_min < min_y { min_y = sy_min; }
        if sy_max > max_y { max_y = sy_max; }
    }

    let start_y = (min_y.floor() as isize).max(0) as usize;
    let end_y = (max_y.ceil() as isize).min(height as isize - 1) as usize;
    let max_span_width = (max_x - min_x).ceil() as usize + 2;

    let mut spans = Vec::new();
    let mut intersections = Vec::with_capacity(32);

    for y in start_y..=end_y {
        let scan_y = y as f32 + 0.5;
        intersections.clear();

        for seg in segments {
            let y1 = seg.y1;
            let y2 = seg.y2;

            if (y1 <= scan_y && y2 > scan_y) || (y2 <= scan_y && y1 > scan_y) {
                let t = (scan_y - y1) / (y2 - y1);
                let x = seg.x1 + t * (seg.x2 - seg.x1);
                let seg_min_x = seg.x1.min(seg.x2);
                let seg_max_x = seg.x1.max(seg.x2);
                let clamped_x = x.clamp(seg_min_x - 0.5, seg_max_x + 0.5);
                intersections.push(clamped_x);
            }
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut i = 0;
        while i + 1 < intersections.len() {
            let x1 = (intersections[i].round() as isize).max(min_x.floor() as isize).max(0) as usize;
            let x2 = (intersections[i + 1].round() as isize).min(max_x.ceil() as isize).min(width as isize - 1) as usize;
            if x1 <= x2 && (x2 - x1) <= max_span_width {
                spans.push((y, x1, x2));
            }
            i += 2;
        }
    }

    spans
}
