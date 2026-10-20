mod app;
mod docker;
mod icon;
mod model;
mod tray;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    std::env::set_var("DOCKER_HOST", "unix:///var/run/docker.sock");
    std::env::set_var("DOCKER_CONTEXT", "default");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Docker Deck")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([1000.0, 650.0])
            .with_icon(std::sync::Arc::new(icon::app_icon()))
            .with_app_id("docker-deck"),
        ..Default::default()
    };

    eframe::run_native(
        "Docker Deck",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
