mod app;
mod camera;
mod gpu_renderer;
mod label_layout;
mod parser;
mod rasterizer;
mod types;
mod ui;

use app::GrandStratApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    println!("⚔ Launching Grand Strategy Rust Map Engine (22,711 Provinces)...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("⚔ Grand Strategy Rust Map Engine • 22,711 Locations (1450)")
            .with_inner_size([1600.0, 950.0])
            .with_min_inner_size([900.0, 600.0])
            .with_maximized(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "GrandStrategyEngine",
        options,
        Box::new(|cc| Ok(Box::new(GrandStratApp::new(cc)))),
    )
}
