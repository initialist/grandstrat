use crate::camera::Camera;
use crate::types::{EditorTool, MapGroup, MapMode, Province, RenderStats};
use eframe::egui;
use std::collections::HashSet;

pub struct UiState {
    pub show_search: bool,
    pub show_editor: bool,
    pub search_query: String,
    pub faction_filter: String,
    pub active_tool: EditorTool,
    pub active_group_key: String,

    // New Custom Faction State
    pub new_faction_label: String,
    pub new_faction_color: [u8; 3],
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_search: false,
            show_editor: false,
            search_query: String::new(),
            faction_filter: String::new(),
            active_tool: EditorTool::Brush,
            active_group_key: "#810f7c".to_string(), // Ottoman default
            new_faction_label: "New Faction".to_string(),
            new_faction_color: [180, 50, 80],
        }
    }
}

pub fn draw_ui(
    ctx: &egui::Context,
    ui_state: &mut UiState,
    camera: &mut Camera,
    provinces: &mut [Province],
    groups: &mut std::collections::HashMap<String, MapGroup>,
    hovered_idx: Option<usize>,
    selected_idx: &mut Option<usize>,
    map_mode: &mut MapMode,
    show_borders: &mut bool,
    stats: &RenderStats,
    export_requested: &mut bool,
    palette_dirty: &mut bool,
) {
    // 1. Top Panel Navigation Bar
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("⚔ Grand Strategy Rust Engine");
            ui.label(egui::RichText::new("22,711 Provinces • 1450 Scenario").color(egui::Color32::from_rgb(0, 210, 255)).strong());

            ui.separator();

            // Performance Badge
            let fps_color = if stats.fps >= 55 {
                egui::Color32::from_rgb(46, 204, 113)
            } else {
                egui::Color32::from_rgb(231, 76, 60)
            };
            ui.colored_label(fps_color, format!("{} FPS", stats.fps));
            ui.label(format!("{:.1}ms", stats.render_time_ms));
            ui.label(format!("{:.2}x", camera.zoom));

            ui.separator();

            // Map Mode Selectors
            let prev_mode = *map_mode;
            ui.selectable_value(map_mode, MapMode::Political, "Political");
            ui.selectable_value(map_mode, MapMode::Wastelands, "Wastelands");
            ui.selectable_value(map_mode, MapMode::Independent, "Independent");
            ui.selectable_value(map_mode, MapMode::Plain, "Plain");

            if prev_mode != *map_mode {
                *palette_dirty = true;
            }

            ui.separator();

            ui.checkbox(show_borders, "Borders");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let editor_btn = if ui_state.show_editor { "🎨 Editor [ON]" } else { "🎨 Scenario Editor" };
                if ui.button(editor_btn).clicked() {
                    ui_state.show_editor = !ui_state.show_editor;
                }
                if ui.button("🔍 Search (/)").clicked() {
                    ui_state.show_search = !ui_state.show_search;
                }
                if ui.button("Reset View").clicked() {
                    camera.fit_to_screen(camera.screen_width, camera.screen_height);
                }
            });
        });
    });

    // 2. Search Dialog
    if ui_state.show_search {
        let mut close_search = false;
        egui::Window::new("🔍 Search Provinces & Factions")
            .open(&mut ui_state.show_search)
            .collapsible(false)
            .resizable(true)
            .default_size([460.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    let resp = ui.text_edit_singleline(&mut ui_state.search_query);
                    if ui_state.search_query.is_empty() {
                        resp.request_focus();
                    }
                });

                ui.separator();

                let q = ui_state.search_query.trim().to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut count = 0;
                    for p in provinces.iter() {
                        if q.is_empty() || p.name.to_lowercase().contains(&q) || p.id.to_lowercase().contains(&q) || p.group_label.to_lowercase().contains(&q) {
                            ui.horizontal(|ui| {
                                let c = p.color;
                                ui.painter().circle_filled(
                                    ui.cursor().min + egui::vec2(6.0, 8.0),
                                    5.0,
                                    egui::Color32::from_rgb(c[0], c[1], c[2]),
                                );
                                ui.add_space(14.0);

                                if ui.selectable_label(*selected_idx == Some(p.index), &p.name).clicked() {
                                    *selected_idx = Some(p.index);
                                    camera.jump_to(p.centroid[0], p.centroid[1], 8.0);
                                    close_search = true;
                                }
                                ui.label(egui::RichText::new(&p.group_label).color(egui::Color32::GRAY).small());

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("Focus").clicked() {
                                        *selected_idx = Some(p.index);
                                        camera.jump_to(p.centroid[0], p.centroid[1], 8.0);
                                        close_search = true;
                                    }
                                });
                            });
                            count += 1;
                            if count >= 40 {
                                break;
                            }
                        }
                    }
                });
            });

        if close_search {
            ui_state.show_search = false;
        }
    }

    // 3. Selected Province Inspector
    if let Some(idx) = *selected_idx {
        if idx < provinces.len() {
            let (p_name, p_id, p_color, p_group_label, p_group_key, p_is_wasteland, p_centroid) = {
                let p = &provinces[idx];
                (p.name.clone(), p.id.clone(), p.color, p.group_label.clone(), p.group_key.clone(), p.is_wasteland, p.centroid)
            };

            egui::Window::new("📍 Province Details")
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
                .collapsible(true)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading(&p_name);
                    ui.label(egui::RichText::new(format!("ID: {}", p_id)).color(egui::Color32::GRAY).monospace().small());

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Owner:");
                        ui.painter().circle_filled(
                            ui.cursor().min + egui::vec2(6.0, 8.0),
                            5.0,
                            egui::Color32::from_rgb(p_color[0], p_color[1], p_color[2]),
                        );
                        ui.add_space(14.0);
                        ui.strong(&p_group_label);
                    });

                    ui.label(format!("Class: {}", if p_is_wasteland { "Wasteland" } else { "Settled Land" }));
                    ui.label(format!("Location: X: {:.1}, Y: {:.1}", p_centroid[0], p_centroid[1]));

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("🎯 Focus Camera").clicked() {
                            camera.jump_to(p_centroid[0], p_centroid[1], 8.0);
                        }
                        if !p_group_key.is_empty() && ui.button("💧 Pick Faction").clicked() {
                            ui_state.active_group_key = p_group_key.clone();
                            ui_state.active_tool = EditorTool::Brush;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("🖌 Paint Selected").clicked() {
                            let active_key = ui_state.active_group_key.clone();
                            if let Some(g) = groups.get(&active_key) {
                                provinces[idx].group_key = active_key;
                                provinces[idx].group_label = g.label.clone();
                                provinces[idx].color = g.color;
                                *palette_dirty = true;
                            }
                        }
                        if ui.button("🧹 Unassign").clicked() {
                            provinces[idx].group_key = String::new();
                            provinces[idx].group_label = "Unassigned".to_string();
                            provinces[idx].color = [209, 219, 221];
                            *palette_dirty = true;
                        }
                        if ui.button("Close").clicked() {
                            *selected_idx = None;
                        }
                    });
                });
        }
    }

    // 4. Hover Tooltip (Floating overlay at cursor)
    if let Some(h_idx) = hovered_idx {
        if h_idx < provinces.len() && *selected_idx != Some(h_idx) {
            let p = &provinces[h_idx];
            if let Some(mouse_pos) = ctx.pointer_hover_pos() {
                egui::Area::new(egui::Id::new("hover_tooltip_area"))
                    .fixed_pos(mouse_pos + egui::vec2(15.0, 15.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let c = p.color;
                                ui.painter().circle_filled(
                                    ui.cursor().min + egui::vec2(6.0, 8.0),
                                    4.0,
                                    egui::Color32::from_rgb(c[0], c[1], c[2]),
                                );
                                ui.add_space(12.0);
                                ui.strong(&p.name);
                            });
                            ui.label(egui::RichText::new(&p.group_label).color(egui::Color32::GRAY).small());
                        });
                    });
            }
        }
    }

    // 5. Scenario Editor Window
    if ui_state.show_editor {
        egui::Window::new("🎨 Scenario Editor & Painter")
            .open(&mut ui_state.show_editor)
            .default_size([340.0, 520.0])
            .show(ctx, |ui| {
                // Tool Selectors
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut ui_state.active_tool, EditorTool::Brush, "🖌 Paint");
                    ui.selectable_value(&mut ui_state.active_tool, EditorTool::Eraser, "🧹 Eraser");
                    ui.selectable_value(&mut ui_state.active_tool, EditorTool::Eyedropper, "💧 Eyedropper");
                });

                ui.separator();

                // Active Tool Banner
                let (active_color, active_label) = if let Some(g) = groups.get(&ui_state.active_group_key) {
                    (g.color, g.label.clone())
                } else {
                    ([200, 200, 200], "None Selected".to_string())
                };

                ui.horizontal(|ui| {
                    ui.label("Current Brush:");
                    ui.painter().circle_filled(
                        ui.cursor().min + egui::vec2(6.0, 8.0),
                        6.0,
                        egui::Color32::from_rgb(active_color[0], active_color[1], active_color[2]),
                    );
                    ui.add_space(16.0);
                    ui.strong(&active_label);
                });

                ui.separator();

                // Quick Empire Select
                ui.label(egui::RichText::new("Quick Select Empires:").small().strong());
                ui.horizontal_wrapped(|ui| {
                    let quick_keys = [
                        ("#810f7c", "Ottomans"),
                        ("#df65b0", "Ming Dynasty"),
                        ("#88419d", "Timurids"),
                        ("#02818a", "France"),
                        ("#67a9cf", "Castile"),
                        ("#bdc9e1", "England"),
                        ("#016c59", "Muscovy"),
                        ("#6baed6", "Mamluks"),
                    ];
                    for (k, name) in quick_keys {
                        if ui.small_button(name).clicked() {
                            ui_state.active_group_key = k.to_string();
                            ui_state.active_tool = EditorTool::Brush;
                        }
                    }
                });

                ui.separator();

                // Faction Search Filter
                ui.horizontal(|ui| {
                    ui.label("Search Factions:");
                    ui.text_edit_singleline(&mut ui_state.faction_filter);
                });

                let filter = ui_state.faction_filter.trim().to_lowercase();

                // Scrollable Faction List
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    let mut sorted_groups: Vec<_> = groups.values().collect();
                    sorted_groups.sort_by(|a, b| b.paths.len().cmp(&a.paths.len()));

                    for g in sorted_groups {
                        if filter.is_empty() || g.label.to_lowercase().contains(&filter) {
                            let is_active = ui_state.active_group_key == g.key;
                            ui.horizontal(|ui| {
                                let c = g.color;
                                ui.painter().circle_filled(
                                    ui.cursor().min + egui::vec2(6.0, 8.0),
                                    5.0,
                                    egui::Color32::from_rgb(c[0], c[1], c[2]),
                                );
                                ui.add_space(14.0);

                                if ui.selectable_label(is_active, format!("{} ({})", g.label, g.paths.len())).clicked() {
                                    ui_state.active_group_key = g.key.clone();
                                    ui_state.active_tool = EditorTool::Brush;
                                }
                            });
                        }
                    }
                });

                ui.separator();

                // Create Custom Faction Section
                ui.collapsing("➕ Add New Custom Faction", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut ui_state.new_faction_label);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut rgb = [
                            ui_state.new_faction_color[0] as f32 / 255.0,
                            ui_state.new_faction_color[1] as f32 / 255.0,
                            ui_state.new_faction_color[2] as f32 / 255.0,
                        ];
                        if ui.color_edit_button_rgb(&mut rgb).changed() {
                            ui_state.new_faction_color = [
                                (rgb[0] * 255.0) as u8,
                                (rgb[1] * 255.0) as u8,
                                (rgb[2] * 255.0) as u8,
                            ];
                        }
                    });

                    if ui.button("Create Faction").clicked() {
                        let c = ui_state.new_faction_color;
                        let hex_key = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
                        let label = ui_state.new_faction_label.trim().to_string();
                        if !label.is_empty() {
                            groups.insert(
                                hex_key.clone(),
                                MapGroup {
                                    key: hex_key.clone(),
                                    label,
                                    paths: HashSet::new(),
                                    color: c,
                                },
                            );
                            ui_state.active_group_key = hex_key;
                            ui_state.active_tool = EditorTool::Brush;
                        }
                    }
                });

                ui.separator();

                if ui.button("💾 Export MapChart Config (.txt)").clicked() {
                    *export_requested = true;
                }
            });
    }
}
