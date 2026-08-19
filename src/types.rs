use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Province {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub group_key: String,
    pub group_label: String,
    pub color: [u8; 3],
    pub is_wasteland: bool,
    pub centroid: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct MapGroup {
    pub key: String,
    pub label: String,
    pub paths: HashSet<String>,
    pub color: [u8; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MapMode {
    Political,
    Wastelands,
    Independent,
    Plain,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    Brush,
    Eraser,
    Eyedropper,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigGroup {
    pub label: String,
    pub paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapConfig {
    pub groups: HashMap<String, ConfigGroup>,
    pub title: Option<String>,
    pub background: Option<String>,
    pub borders: Option<String>,
    #[serde(rename = "defaultColor")]
    pub default_color: Option<String>,
    #[serde(rename = "areBordersShown")]
    pub are_borders_shown: Option<bool>,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct RenderStats {
    pub fps: u32,
    pub render_time_ms: f32,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}
