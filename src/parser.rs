use crate::rasterizer::{WORLD_HEIGHT, WORLD_WIDTH};
use crate::types::{ConfigGroup, MapConfig, MapGroup, Province};
use std::collections::{HashMap, HashSet};

pub struct RawPath {
    pub id: String,
    pub d: String,
}

#[derive(Clone, Debug)]
pub struct SvgCommand {
    pub cmd: char,
    pub params: Vec<f32>,
}

pub fn parse_svg_paths(svg_content: &str) -> Vec<RawPath> {
    let mut paths = Vec::with_capacity(23000);
    let bytes = svg_content.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i + 5 < len {
        if bytes[i] == b'<' && &bytes[i..i + 5] == b"<path" {
            let start = i;
            let mut end = start + 5;
            while end < len && bytes[end] != b'>' {
                end += 1;
            }
            if end < len {
                let tag_bytes = &bytes[start..=end];
                if let Ok(tag_str) = std::str::from_utf8(tag_bytes) {
                    if let (Some(id), Some(d)) = (extract_attr(tag_str, "id"), extract_attr(tag_str, "d")) {
                        if !id.starts_with("pattern") {
                            paths.push(RawPath { id, d });
                        }
                    }
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    paths
}

fn extract_attr(tag_str: &str, attr_name: &str) -> Option<String> {
    let patterns = [
        format!(" {}=\"", attr_name),
        format!("\n{}=\"", attr_name),
        format!("\t{}=\"", attr_name),
        format!(" {}='", attr_name),
        format!("\n{}='", attr_name),
        format!("\t{}='", attr_name),
    ];

    for pattern in &patterns {
        if let Some(start_idx) = tag_str.find(pattern) {
            let val_start = start_idx + pattern.len();
            let quote_char = if pattern.ends_with('"') { '"' } else { '\'' };
            if let Some(end_idx) = tag_str[val_start..].find(quote_char) {
                return Some(tag_str[val_start..val_start + end_idx].to_string());
            }
        }
    }
    None
}

pub fn parse_svg_commands(d: &str) -> Vec<SvgCommand> {
    let mut commands = Vec::with_capacity(128);
    let mut current_cmd = None;
    let mut current_params = Vec::with_capacity(16);

    let bytes = d.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        let b = bytes[i];
        if b.is_ascii_alphabetic() {
            if let Some(cmd) = current_cmd {
                commands.push(SvgCommand {
                    cmd,
                    params: std::mem::take(&mut current_params),
                });
            }
            current_cmd = Some(b as char);
            i += 1;
        } else if b == b' ' || b == b',' || b == b'\n' || b == b'\r' || b == b'\t' {
            i += 1;
        } else if b == b'-' || b == b'+' || b.is_ascii_digit() || b == b'.' {
            let start = i;
            let mut has_dot = b == b'.';
            i += 1;

            while i < len {
                let nb = bytes[i];
                if nb.is_ascii_digit() {
                    i += 1;
                } else if nb == b'.' {
                    if has_dot {
                        break;
                    }
                    has_dot = true;
                    i += 1;
                } else if nb == b'-' || nb == b'+' {
                    break;
                } else if nb == b'e' || nb == b'E' {
                    i += 1;
                    if i < len && (bytes[i] == b'-' || bytes[i] == b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }

            if let Ok(s) = std::str::from_utf8(&bytes[start..i]) {
                if let Ok(val) = s.parse::<f32>() {
                    current_params.push(val);
                }
            }
        } else {
            i += 1;
        }
    }

    if let Some(cmd) = current_cmd {
        commands.push(SvgCommand {
            cmd,
            params: current_params,
        });
    }

    commands
}

pub fn parse_hex_color(hex: &str) -> [u8; 3] {
    if hex.starts_with('#') && hex.len() == 7 {
        if let Ok(val) = u32::from_str_radix(&hex[1..7], 16) {
            return [
                ((val >> 16) & 0xFF) as u8,
                ((val >> 8) & 0xFF) as u8,
                (val & 0xFF) as u8,
            ];
        }
    } else if hex.starts_with("diagonal") || hex.starts_with("horizontal") || hex.starts_with("vertical") {
        let parts: Vec<&str> = hex.split('_').collect();
        if parts.len() >= 2 {
            let bg_hex = parts[1];
            if bg_hex.len() == 6 {
                if let Ok(val) = u32::from_str_radix(bg_hex, 16) {
                    return [
                        ((val >> 16) & 0xFF) as u8,
                        ((val >> 8) & 0xFF) as u8,
                        (val & 0xFF) as u8,
                    ];
                }
            }
        }
    }
    [209, 219, 221] // Default #d1dbdd
}

pub fn build_provinces(raw_paths: &[RawPath]) -> (Vec<Province>, HashMap<String, usize>) {
    let mut provinces = Vec::with_capacity(raw_paths.len());
    let mut id_to_index = HashMap::with_capacity(raw_paths.len());

    for (index, p) in raw_paths.iter().enumerate() {
        let (cx, cy) = compute_path_centroid(&p.d);
        let name = p.id.replace('_', " ");

        provinces.push(Province {
            index,
            id: p.id.clone(),
            name,
            group_key: String::new(),
            group_label: "Unassigned".to_string(),
            color: [209, 219, 221],
            is_wasteland: false,
            centroid: [cx, cy],
            settlement: None,
            biome: crate::types::BiomeType::Grassland,
        });

        id_to_index.insert(p.id.clone(), index);
    }

    (provinces, id_to_index)
}

pub fn apply_config(
    config: &MapConfig,
    provinces: &mut [Province],
    id_to_index: &HashMap<String, usize>,
) -> HashMap<String, MapGroup> {
    let mut groups = HashMap::new();

    for (key, grp) in &config.groups {
        let color = parse_hex_color(key);
        let is_wasteland = grp.label.to_lowercase().contains("wasteland");
        let paths: HashSet<String> = grp.paths.iter().cloned().collect();

        for path_id in &grp.paths {
            if let Some(&idx) = id_to_index.get(path_id) {
                if idx < provinces.len() {
                    provinces[idx].group_key = key.clone();
                    provinces[idx].group_label = grp.label.clone();
                    provinces[idx].color = color;
                    provinces[idx].is_wasteland = is_wasteland;
                }
            }
        }

        groups.insert(
            key.clone(),
            MapGroup {
                key: key.clone(),
                label: grp.label.clone(),
                paths,
                color,
                capital_province_id: None,
                capital_name: None,
                capital_pos: None,
            },
        );
    }

    groups
}

pub fn serialize_to_mapchart_json(
    provinces: &[Province],
    groups: &HashMap<String, MapGroup>,
) -> String {
    let mut config_groups: HashMap<String, ConfigGroup> = HashMap::new();

    let mut group_paths: HashMap<String, Vec<String>> = HashMap::new();
    for p in provinces {
        if !p.group_key.is_empty() {
            group_paths
                .entry(p.group_key.clone())
                .or_default()
                .push(p.id.clone());
        }
    }

    for (key, paths) in group_paths {
        let label = groups
            .get(&key)
            .map(|g| g.label.clone())
            .unwrap_or_else(|| "Faction".to_string());
        config_groups.insert(key, ConfigGroup { label, paths });
    }

    let config = MapConfig {
        groups: config_groups,
        title: Some("World 1450 Grand Strategy".to_string()),
        background: Some("#013f3f".to_string()),
        borders: Some("#000".to_string()),
        default_color: Some("#d1dbdd".to_string()),
        are_borders_shown: Some(true),
    };

    serde_json::to_string_pretty(&config).unwrap_or_default()
}

pub fn compute_path_centroid(d: &str) -> (f32, f32) {
    let commands = parse_svg_commands(d);
    let mut cur_x = 0.0f32;
    let mut cur_y = 0.0f32;
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for cmd in commands {
        let is_rel = cmd.cmd.is_ascii_lowercase();
        let upper = cmd.cmd.to_ascii_uppercase();
        let nums = &cmd.params;
        let mut idx = 0;

        match upper {
            'M' => {
                if idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    cur_x = x;
                    cur_y = y;
                    start_x = x;
                    start_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 2;
                }
                while idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    cur_x = x;
                    cur_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 2;
                }
            }
            'L' => {
                while idx + 1 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    let y = if is_rel { cur_y + nums[idx + 1] } else { nums[idx + 1] };
                    cur_x = x;
                    cur_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 2;
                }
            }
            'H' => {
                while idx < nums.len() {
                    let x = if is_rel { cur_x + nums[idx] } else { nums[idx] };
                    cur_x = x;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    idx += 1;
                }
            }
            'V' => {
                while idx < nums.len() {
                    let y = if is_rel { cur_y + nums[idx] } else { nums[idx] };
                    cur_y = y;
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 1;
                }
            }
            'C' => {
                while idx + 5 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx + 4] } else { nums[idx + 4] };
                    let y = if is_rel { cur_y + nums[idx + 5] } else { nums[idx + 5] };
                    cur_x = x;
                    cur_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 6;
                }
            }
            'S' | 'Q' => {
                while idx + 3 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx + 2] } else { nums[idx + 2] };
                    let y = if is_rel { cur_y + nums[idx + 3] } else { nums[idx + 3] };
                    cur_x = x;
                    cur_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 4;
                }
            }
            'A' => {
                while idx + 6 < nums.len() {
                    let x = if is_rel { cur_x + nums[idx + 5] } else { nums[idx + 5] };
                    let y = if is_rel { cur_y + nums[idx + 6] } else { nums[idx + 6] };
                    cur_x = x;
                    cur_y = y;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    idx += 7;
                }
            }
            'Z' => {
                cur_x = start_x;
                cur_y = start_y;
            }
            _ => {}
        }
    }

    if min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite() {
        ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
    } else {
        (WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5)
    }
}
