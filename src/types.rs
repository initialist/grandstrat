use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum BiomeType {
    Grassland,
    Woods,
    Forest,
    Jungle,
    Taiga,
    Steppe,
    Desert,
    Savanna,
    Tundra,
    Marsh,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SettlementInfo {
    pub name: String,
    pub tier: SettlementTier,
    pub is_capital: bool,
}

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
    pub settlement: Option<SettlementInfo>,
    pub biome: BiomeType,
}

#[derive(Clone, Debug)]
pub struct MapGroup {
    pub key: String,
    pub label: String,
    pub paths: HashSet<String>,
    pub color: [u8; 3],
    pub capital_province_id: Option<String>,
    pub capital_name: Option<String>,
    pub capital_pos: Option<[f32; 2]>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MapMode {
    Political,
    Terrain,
    Wastelands,
    Independent,
    Plain,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LabelAlgorithm {
    Standard,
    Curved,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum SettlementTier {
    Capital, // Tier 1: Global Imperial Metropolises & Great Capitals (Paris, Constantinople, Beijing, Kyoto, etc.)
    City,    // Tier 2: Regional Hubs & Major Cities (Lyon, Toledo, Milan, Alexandria, Agra, etc.)
    Town,    // Tier 3: Local Provincial Towns (EU5 locations)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Settlement {
    pub name: String,
    pub province_id: String,
    pub province_index: usize,
    pub world_pos: [f32; 2],
    pub tier: SettlementTier,
    pub group_key: String,
    pub is_coastal: bool,
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
