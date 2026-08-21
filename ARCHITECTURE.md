# Grand Strategy Rust Map Engine - Architecture & Data Flow Specification

## 🗺️ System Overview
The Grand Strategy Rust Engine is a high-performance vector-raster cartographic system rendering **22,711 provinces/locations** at 60+ FPS with GPU-accelerated 3D terrain, biomes, rivers, curved copperplate typography, and an interactive real-time scenario editor.

---

## 🏗️ Core Architectural Modules

```
src/
├── main.rs            # Entry point & window initialization
├── types.rs           # Core domain models (Province, MapGroup, SettlementInfo, BiomeType, MapMode)
├── parser.rs          # SVG path parser, MapChart JSON parser & serializer
├── rasterizer.rs      # High-speed parallel rasterizer generating 9600x5440 lossless ID maps
├── camera.rs          # Smooth 2D viewport camera (pan, smooth zoom, world-to-screen transforms)
├── gpu_renderer.rs    # Modern OpenGL (GLSL) terrain, biomes, rivers, borders & palette shaders
├── label_layout.rs    # Curved nation arc label generation (Poisson disk, skeleton spine, tangent rot)
├── settlement.rs      # Deterministic world-space city & capital registry with guaranteed group capitals
├── ui.rs              # egui interface (Responsive top bar, Scenario Editor, Search modal, city badges)
└── app.rs             # Main game loop, user interaction dispatch, real-time reactive sync
```

---

## 📊 Data Layer (`src/types.rs`)

### 1. `Province`
Represents an individual EU5 location / province polygon:
- `index: usize`: Continuous 0-based index in the global province array.
- `id: String`: Unique SVG path identifier (e.g. `FR_paris`, `GB_london`).
- `name: String`: Human-readable province name.
- `group_key: String`: Faction / country color key (e.g. `#08519c` for France).
- `group_label: String`: Faction display name (e.g. `"France"`).
- `color: [u8; 3]`: Active RGB color in political palette.
- `is_wasteland: bool`: Flag designating impassable high alpine crags, mountain ridges, and deep deserts.
- `centroid: [f32; 2]`: Geometric center of mass in SVG world coordinates.
- `settlement: Option<SettlementInfo>`: Urban data (capital / regional city info).
- `biome: BiomeType`: Geographical biome classification (Forest, Jungle, Steppe, Desert, Taiga, Tundra, Grassland).

### 2. `MapGroup`
Represents a sovereign country, empire, or regional tag:
- `key: String`: Unique faction identifier (e.g. `#08519c`).
- `label: String`: Imperial display name (e.g. `"France"`, `"Holy Roman Empire"`).
- `paths: HashSet<String>`: Live set of all province SVG path IDs belonging to this country.
- `color: [u8; 3]`: Sovereign flag / map color.
- `capital_province_id: Option<String>`: Guaranteed capital location ID.
- `capital_name: Option<String>`: Designated capital city name.
- `capital_pos: Option<[f32; 2]>`: World-space coordinates of the nation's capital.

---

## 🎨 GPU Shading Pipeline (`src/gpu_renderer.rs`)

The map is rendered via an OpenGL full-screen quad fragment shader utilizing 4 primary textures:
1. **Lossless Province ID Map (`u_id_texture`)**: 9600x5440 16-bit integer texture indexing all 22,711 provinces.
2. **Elevation DEM (`u_relief_texture`)**: 4K Natural Earth digital elevation model.
3. **Biomes & Landcover (`u_terrain_texture`)**: 4K satellite landcover capturing forests, deserts, and rivers.
4. **Palette Array (`u_palette_texture`)**: Dynamic 256x256 texture mapping province ID -> RGB country paint.

### Shading Techniques:
- **Wasteland-Aligned Harsh Terrain**: Impassable wastelands (Alps spine, Himalayas, Andes, Tibetan Plateau) render sharp, rugged 3D mountain crags and deep rock shadows.
- **Inhabited Soft Topography**: Settled plains, hills, and river valleys render soft, gentle, readable cartographic hillshading (maximum 26% shadow darkening) ensuring total readability on light colors.
- **Biomes & Forests**: Organic foliage micro-stippling over forested regions (Black Forest, Ardennes, Taiga, Congo, Amazon).
- **Global River Waterways**: Radiant freshwater channels reflecting blue water across major river basins (Rhine, Danube, Nile, Yangtze, Amazon, Mississippi).
- **Multi-Tier Borders**:
  - *National Borders*: Distinct ink contours between differing nations (0.72 opacity).
  - *Internal Province Borders*: Hairline incisions visible when zoomed in.

---

## 🔄 Real-Time Reactive Scenario Editor (`src/app.rs`)

When painting or erasing provinces in the Scenario Editor:
1. `apply_editor_action` immediately updates `province.group_key` and synchronizes `group.paths` in memory.
2. `update_palette` updates the GPU palette texture without expensive CPU re-rasterization.
3. `generate_nation_labels` re-calculates curved nation spines and font sizes in real time.
4. `SettlementRegistry::build` updates guaranteed nation capitals on the fly.
