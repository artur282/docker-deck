use crate::app::App;
use crate::model::{ConfirmRequest, ImageInfo, PendingAction, PruneKind};
use crate::ui::components;
use eframe::egui;

const NAME_CHARS: usize = 46;
const CREATED_CHARS: usize = 24;

const DANGLING_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 180, 80);

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let images = app.snapshot.images.clone();

    draw_header(ui, &images);
    draw_toolbar(app, ui);

    let query = app.image_search.trim().to_ascii_lowercase();
    let filtered: Vec<&ImageInfo> = images
        .iter()
        .filter(|image| matches_query(image, &query))
        .collect();

    if filtered.is_empty() {
        components::empty_state(ui, empty_message(&query));
        return;
    }

    draw_table(app, ui, &filtered);
}

fn draw_header(ui: &mut egui::Ui, images: &[ImageInfo]) {
    let dangling = images.iter().filter(|image| image.dangling).count();
    components::page_header(
        ui,
        "Images",
        &format!("{} total · {} dangling", images.len(), dangling),
    );
}

fn draw_toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        components::search_box(ui, &mut app.image_search, "Search repository, tag, id…");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(!app.busy, |ui| {
                if components::danger_button(ui, "Prune dangling").clicked() {
                    app.ask(prune_request());
                }
            });
        });
    });
    ui.add_space(6.0);
}

fn draw_table(app: &mut App, ui: &mut egui::Ui, images: &[&ImageInfo]) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("images_table")
                .num_columns(5)
                .spacing([16.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for title in ["Repository:Tag", "ID", "Size", "Created", "Actions"] {
                        header_label(ui, title);
                    }
                    ui.end_row();

                    for image in images {
                        name_cell(ui, image);
                        id_cell(ui, image);
                        ui.label(image.size.as_str());
                        created_cell(ui, image);
                        actions_cell(app, ui, image);
                        ui.end_row();
                    }
                });
        });
}

fn header_label(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .small()
            .strong()
            .color(ui.visuals().weak_text_color()),
    );
}

fn name_cell(ui: &mut egui::Ui, image: &ImageInfo) {
    let name = image.display_name();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(components::truncate_middle(
            &name, NAME_CHARS,
        )))
        .on_hover_text(&name);
        if image.dangling {
            components::badge(ui, "dangling", DANGLING_COLOR);
        }
    });
}

fn id_cell(ui: &mut egui::Ui, image: &ImageInfo) {
    ui.label(egui::RichText::new(image.short_id()).monospace())
        .on_hover_text(image.id.as_str());
}

fn created_cell(ui: &mut egui::Ui, image: &ImageInfo) {
    ui.label(egui::RichText::new(components::truncate_middle(
        &image.created,
        CREATED_CHARS,
    )))
    .on_hover_text(image.created.as_str());
}

fn actions_cell(app: &mut App, ui: &mut egui::Ui, image: &ImageInfo) {
    ui.add_enabled_ui(!app.busy, |ui| {
        if components::danger_button(ui, "Remove").clicked() {
            app.ask(remove_request(image));
        }
    });
}

fn matches_query(image: &ImageInfo, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    [
        image.repository.as_str(),
        image.tag.as_str(),
        image.id.as_str(),
    ]
    .iter()
    .any(|field| field.to_ascii_lowercase().contains(query))
}

fn empty_message(query: &str) -> &'static str {
    if query.is_empty() {
        "No images found."
    } else {
        "No images match your search."
    }
}

fn remove_request(image: &ImageInfo) -> ConfirmRequest {
    ConfirmRequest {
        title: "Remove image".to_string(),
        message: format!(
            "Remove image \"{}\"? It is deleted from local storage; containers using it keep running until recreated.",
            image.display_name()
        ),
        action: PendingAction::ImageRemove(image.id.clone()),
        confirm_label: "Remove image".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}

fn prune_request() -> ConfirmRequest {
    ConfirmRequest {
        title: "Prune images".to_string(),
        message: format!(
            "Remove all {}? Dangling images have no tag and are not used by any container. This cannot be undone.",
            PruneKind::Images.label()
        ),
        action: PendingAction::Prune(PruneKind::Images),
        confirm_label: "Prune images".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}
