use crate::camera::Camera;
use crate::gpu_renderer::GpuRenderer;
use crate::parser::{apply_config, build_provinces, parse_svg_paths, serialize_to_mapchart_json};
use crate::rasterizer::{get_or_build_id_map, RasterizedMap, MAP_HEIGHT, MAP_WIDTH, WORLD_HEIGHT, WORLD_WIDTH};
use crate::types::{EditorTool, MapConfig, MapGroup, MapMode, Province, RenderStats};
use crate::ui::{draw_ui, UiState};
use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

pub struct GrandStratApp {
    camera: Camera,
    gpu_renderer: Option<GpuRenderer>,
    raster_map: RasterizedMap,
    provinces: Vec<Province>,
    groups: HashMap<String, MapGroup>,
    ui_state: UiState,

    hovered_idx: Option<usize>,
    selected_idx: Option<usize>,
    map_mode: MapMode,
    show_borders: bool,

    has_moved_while_down: bool,
    is_first_frame: bool,

    stats: RenderStats,
    frame_count: u32,
    fps_timer: Instant,
}

impl GrandStratApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        println!("Reading Blank_Map.svg...");
        let svg_content = fs::read_to_string("Blank_Map.svg").expect("Failed to read Blank_Map.svg");
        let raw_paths = parse_svg_paths(&svg_content);
        println!("Parsed {} SVG paths", raw_paths.len());

        let (mut provinces, id_to_index) = build_provinces(&raw_paths);

        println!("Reading mapchart-config-world-1450.txt...");
        let config_str = fs::read_to_string("mapchart-config-world-1450.txt").unwrap_or_default();
        let config: MapConfig = serde_json::from_str(&config_str).unwrap_or(MapConfig {
            groups: HashMap::new(),
            title: None,
            background: None,
            borders: None,
            default_color: None,
            are_borders_shown: None,
        });

        let groups = apply_config(&config, &mut provinces, &id_to_index);
        println!("Applied {} groups across provinces", groups.len());

        let raster_map = get_or_build_id_map(&raw_paths);

        let gl = cc.gl.clone().expect("OpenGL context required");
        let mut gpu_renderer = GpuRenderer::new(gl, &raster_map);
        gpu_renderer.update_palette(&provinces, MapMode::Political, [1, 63, 63], [209, 219, 221]);

        let mut camera = Camera::default();
        camera.fit_to_screen(1280.0, 720.0);

        Self {
            camera,
            gpu_renderer: Some(gpu_renderer),
            raster_map,
            provinces,
            groups,
            ui_state: UiState::default(),
            hovered_idx: None,
            selected_idx: None,
            map_mode: MapMode::Political,
            show_borders: true,
            has_moved_while_down: false,
            is_first_frame: true,
            stats: RenderStats::default(),
            frame_count: 0,
            fps_timer: Instant::now(),
        }
    }

    fn get_province_at_world(&self, world_x: f32, world_y: f32) -> Option<usize> {
        let px = ((world_x / WORLD_WIDTH) * MAP_WIDTH as f32).floor() as isize;
        let py = ((world_y / WORLD_HEIGHT) * MAP_HEIGHT as f32).floor() as isize;

        if px < 0 || px >= MAP_WIDTH as isize || py < 0 || py >= MAP_HEIGHT as isize {
            return None;
        }

        let idx = (py as usize) * MAP_WIDTH + (px as usize);
        if idx >= self.raster_map.id_buffer.len() {
            return None;
        }

        let id = self.raster_map.id_buffer[idx];
        if id == 0 {
            None
        } else {
            let p_idx = (id - 1) as usize;
            if p_idx < self.provinces.len() {
                Some(p_idx)
            } else {
                None
            }
        }
    }

    fn apply_editor_action(&mut self, p_idx: usize) -> bool {
        let mut dirty = false;
        match self.ui_state.active_tool {
            EditorTool::Brush => {
                let group_key = self.ui_state.active_group_key.clone();
                if let Some(g) = self.groups.get(&group_key) {
                    let p = &mut self.provinces[p_idx];
                    p.group_key = group_key;
                    p.group_label = g.label.clone();
                    p.color = g.color;
                    dirty = true;
                }
            }
            EditorTool::Eraser => {
                let p = &mut self.provinces[p_idx];
                p.group_key = String::new();
                p.group_label = "Unassigned".to_string();
                p.color = [209, 219, 221];
                dirty = true;
            }
            EditorTool::Eyedropper => {
                let p = &self.provinces[p_idx];
                if !p.group_key.is_empty() {
                    self.ui_state.active_group_key = p.group_key.clone();
                    self.ui_state.active_tool = EditorTool::Brush;
                }
            }
        }
        dirty
    }
}

impl eframe::App for GrandStratApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_start = Instant::now();
        self.camera.update();

        // Handle Keyboard Shortcuts
        ctx.input(|i| {
            let step = 50.0;
            if i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp) {
                self.camera.pan_by(0.0, step);
            }
            if i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown) {
                self.camera.pan_by(0.0, -step);
            }
            if i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft) {
                self.camera.pan_by(step, 0.0);
            }
            if i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight) {
                self.camera.pan_by(-step, 0.0);
            }
            if i.key_pressed(egui::Key::Slash) {
                self.ui_state.show_search = !self.ui_state.show_search;
            }
            if i.key_pressed(egui::Key::B) {
                self.ui_state.active_tool = EditorTool::Brush;
            }
            if i.key_pressed(egui::Key::E) {
                self.ui_state.active_tool = EditorTool::Eraser;
            }
            if i.key_pressed(egui::Key::I) {
                self.ui_state.active_tool = EditorTool::Eyedropper;
            }
        });

        let mut palette_dirty = false;

        // Main Map Canvas Central Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let screen_w = rect.width();
                let screen_h = rect.height();

                if self.is_first_frame || (self.camera.screen_width - screen_w).abs() > 1.0 || (self.camera.screen_height - screen_h).abs() > 1.0 {
                    self.camera.fit_to_screen(screen_w, screen_h);
                    self.is_first_frame = false;
                }

                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                // Mouse Wheel Zoom
                let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 {
                    if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        let zoom_factor = if scroll_delta > 0.0 { 1.15 } else { 0.87 };
                        self.camera.zoom_at(mouse_pos.x, mouse_pos.y, zoom_factor);
                    }
                }

                // Mouse Move & Hover
                if let Some(pos) = response.hover_pos() {
                    let world_pt = self.camera.screen_to_world(pos.x, pos.y);
                    self.hovered_idx = self.get_province_at_world(world_pt[0], world_pt[1]);

                    // Drag Interaction (Panning or Painting/Erasing)
                    if response.dragged_by(egui::PointerButton::Primary) || response.dragged_by(egui::PointerButton::Middle) {
                        let delta = response.drag_delta();
                        if delta.length() > 0.5 {
                            self.has_moved_while_down = true;

                            if self.ui_state.show_editor && (self.ui_state.active_tool == EditorTool::Brush || self.ui_state.active_tool == EditorTool::Eraser) && response.dragged_by(egui::PointerButton::Primary) {
                                if let Some(p_idx) = self.hovered_idx {
                                    if self.apply_editor_action(p_idx) {
                                        palette_dirty = true;
                                    }
                                }
                            } else {
                                self.camera.pan_by(delta.x, delta.y);
                            }
                        }
                    }

                    // Click Selection / Editing
                    if response.clicked() && !self.has_moved_while_down {
                        if self.ui_state.show_editor {
                            if let Some(p_idx) = self.hovered_idx {
                                if self.apply_editor_action(p_idx) {
                                    palette_dirty = true;
                                }
                            }
                            self.selected_idx = self.hovered_idx;
                        } else {
                            self.selected_idx = self.hovered_idx;
                        }
                    }
                }

                if response.drag_stopped() {
                    self.has_moved_while_down = false;
                }

                // Render GPU Map Quad
                if let Some(renderer) = &self.gpu_renderer {
                    let dpr = ctx.pixels_per_point();
                    let total_h = ctx.screen_rect().height();
                    let physical_x = rect.min.x * dpr;
                    let physical_y = (total_h - rect.max.y) * dpr;
                    let physical_w = rect.width() * dpr;
                    let physical_h = rect.height() * dpr;
                    let viewport_rect = [physical_x, physical_y, physical_w, physical_h];

                    renderer.render(
                        &self.camera,
                        viewport_rect,
                        dpr,
                        self.hovered_idx,
                        self.selected_idx,
                        self.show_borders,
                    );
                }
            });

        // Draw UI Windows & Overlays
        let mut export_requested = false;
        draw_ui(
            ctx,
            &mut self.ui_state,
            &mut self.camera,
            &mut self.provinces,
            &mut self.groups,
            self.hovered_idx,
            &mut self.selected_idx,
            &mut self.map_mode,
            &mut self.show_borders,
            &self.stats,
            &mut export_requested,
            &mut palette_dirty,
        );

        // Refresh GPU palette if dirty
        if palette_dirty {
            if let Some(renderer) = &mut self.gpu_renderer {
                renderer.update_palette(&self.provinces, self.map_mode, [1, 63, 63], [209, 219, 221]);
            }
        }

        if export_requested {
            let json = serialize_to_mapchart_json(&self.provinces, &self.groups);
            let _ = fs::write("mapchart-config-custom-rust.txt", json);
            println!("Exported mapchart-config-custom-rust.txt");
        }

        // Calculate Frame Stats
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.fps_timer).as_millis() >= 500 {
            let elapsed_secs = now.duration_since(self.fps_timer).as_secs_f32();
            self.stats.fps = (self.frame_count as f32 / elapsed_secs).round() as u32;
            self.stats.render_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            self.stats.zoom = self.camera.zoom;
            self.stats.pan_x = self.camera.pan_x;
            self.stats.pan_y = self.camera.pan_y;
            self.frame_count = 0;
            self.fps_timer = now;
        }

        ctx.request_repaint();
    }
}
