use crate::app::App;
use crate::model::{ConfirmRequest, PendingAction, VolumeInfo};
use crate::ui::components;
use eframe::egui;

const NAME_PREVIEW_LIMIT: usize = 40;
const MAX_NAMED_IN_CONFIRM: usize = 8;
const AMBER: egui::Color32 = egui::Color32::from_rgb(230, 180, 80);
const GREEN: egui::Color32 = egui::Color32::from_rgb(110, 200, 130);

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let volumes = app.snapshot.volumes.clone();
    let in_use_count = volumes.iter().filter(|volume| volume.in_use()).count();
    let subtitle = format!(
        "{} {} · {} in use",
        volumes.len(),
        volume_word(volumes.len()),
        in_use_count
    );

    components::page_header(ui, "Volumes", &subtitle);
    toolbar(app, ui);
    ui.add_space(6.0);

    if volumes.is_empty() {
        components::empty_state(ui, "No volumes yet. Create one with docker volume create.");
        return;
    }

    let filtered = filtered_volumes(&volumes, &app.volume_search);
    if filtered.is_empty() {
        components::empty_state(ui, "No volumes match your search.");
        return;
    }

    volume_table(app, ui, &filtered);
}

fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        components::search_box(
            ui,
            &mut app.volume_search,
            "Filter by name, driver or label",
        );

        if ui.button("Select unused").clicked() {
            select_unused(app);
        }
        if ui.button("Clear").clicked() {
            app.selected_volumes.clear();
        }

        ui.separator();

        let selected = app.selected_volumes.len();
        let enabled = selected > 0 && !app.busy;
        let label = format!("Delete selected ({selected})");
        let button = ui
            .add_enabled_ui(enabled, |ui| components::danger_button(ui, &label))
            .inner;

        let button = if selected == 0 {
            button.on_disabled_hover_text("Select at least one unused volume first")
        } else if app.busy {
            button.on_disabled_hover_text("Wait for the current operation to finish")
        } else {
            button
        };

        if button.clicked() {
            request_bulk_delete(app);
        }
    });
}

fn select_unused(app: &mut App) {
    let unused: Vec<String> = app
        .snapshot
        .volumes
        .iter()
        .filter(|volume| volume.links == 0)
        .map(|volume| volume.name.clone())
        .collect();
    app.selected_volumes.extend(unused);
}

fn filtered_volumes(volumes: &[VolumeInfo], search: &str) -> Vec<VolumeInfo> {
    let query = search.trim().to_lowercase();
    let mut filtered: Vec<VolumeInfo> = volumes
        .iter()
        .filter(|volume| matches_query(volume, &query))
        .cloned()
        .collect();
    filtered.sort_by_cached_key(|volume| volume.name.to_lowercase());
    filtered
}

fn matches_query(volume: &VolumeInfo, query: &str) -> bool {
    query.is_empty()
        || volume.name.to_lowercase().contains(query)
        || volume.driver.to_lowercase().contains(query)
        || volume.labels.to_lowercase().contains(query)
}

fn volume_table(app: &mut App, ui: &mut egui::Ui, volumes: &[VolumeInfo]) {
    egui::ScrollArea::vertical()
        .id_salt("volumes_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("volumes_table")
                .num_columns(6)
                .striped(true)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    heading_cell(ui, "");
                    heading_cell(ui, "NAME");
                    heading_cell(ui, "SIZE");
                    heading_cell(ui, "IN USE");
                    heading_cell(ui, "DRIVER");
                    heading_cell(ui, "ACTIONS");
                    ui.end_row();

                    for volume in volumes {
                        selection_cell(app, ui, volume);
                        name_cell(ui, volume);
                        ui.label(size_label(&volume.size));
                        usage_cell(ui, volume);
                        ui.label(or_dash(&volume.driver));
                        delete_cell(app, ui, volume);
                        ui.end_row();
                    }
                });
        });
}

fn heading_cell(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .small()
            .strong()
            .color(ui.visuals().weak_text_color()),
    );
}

fn selection_cell(app: &mut App, ui: &mut egui::Ui, volume: &VolumeInfo) {
    let mut selected = app.selected_volumes.contains(&volume.name);
    let response = ui.add_enabled(
        !volume.in_use(),
        egui::Checkbox::without_text(&mut selected),
    );

    if response.changed() {
        if selected {
            app.selected_volumes.insert(volume.name.clone());
        } else {
            app.selected_volumes.remove(&volume.name);
        }
    }
}

fn name_cell(ui: &mut egui::Ui, volume: &VolumeInfo) {
    let preview = components::truncate_middle(&volume.name, NAME_PREVIEW_LIMIT);
    ui.label(egui::RichText::new(preview).monospace())
        .on_hover_ui(|ui| volume_tooltip(ui, volume));
}

fn volume_tooltip(ui: &mut egui::Ui, volume: &VolumeInfo) {
    ui.set_max_width(420.0);
    ui.label(egui::RichText::new(&volume.name).monospace().strong());
    ui.add_space(4.0);
    ui.label(format!("Mountpoint: {}", or_dash(&volume.mountpoint)));
    ui.label(format!("Labels: {}", or_dash(&volume.labels)));
}

fn usage_cell(ui: &mut egui::Ui, volume: &VolumeInfo) {
    if volume.in_use() {
        components::badge(ui, &format!("in use ({})", volume.links), AMBER);
    } else {
        components::badge(ui, "free", GREEN);
    }
}

fn delete_cell(app: &mut App, ui: &mut egui::Ui, volume: &VolumeInfo) {
    let response = ui
        .add_enabled(!volume.in_use(), egui::Button::new("Delete"))
        .on_disabled_hover_text("Attached to a container. Remove the container first.");

    if response.clicked() {
        app.ask(volume_delete_confirm(&volume.name));
    }
}

fn volume_delete_confirm(name: &str) -> ConfirmRequest {
    ConfirmRequest {
        title: "Delete volume".to_string(),
        message: format!(
            "Delete volume \"{name}\"?\n\nThis permanently deletes the volume and all its data. It cannot be undone."
        ),
        action: PendingAction::VolumeRemove(vec![name.to_string()]),
        confirm_label: "Delete volume".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}

fn request_bulk_delete(app: &mut App) {
    let names: Vec<String> = app.selected_volumes.iter().cloned().collect();
    if names.is_empty() {
        return;
    }

    let label = format!("Delete {} {}", names.len(), volume_word(names.len()));
    let message = bulk_delete_message(&names);
    app.ask(ConfirmRequest {
        title: label.clone(),
        message,
        action: PendingAction::VolumeRemove(names),
        confirm_label: label,
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    });
}

fn bulk_delete_message(names: &[String]) -> String {
    let count = names.len();
    let listed = names
        .iter()
        .take(MAX_NAMED_IN_CONFIRM)
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let summary = if count > MAX_NAMED_IN_CONFIRM {
        format!("{listed} and {} more", count - MAX_NAMED_IN_CONFIRM)
    } else {
        listed
    };
    let warning = if count == 1 {
        "This permanently deletes the volume and all its data. It cannot be undone."
    } else {
        "This permanently deletes the volumes and all their data. It cannot be undone."
    };

    format!(
        "Delete {count} {}?\n\n{summary}\n\n{warning}",
        volume_word(count)
    )
}

fn size_label(size: &str) -> &str {
    let trimmed = size.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("n/a") {
        "—"
    } else {
        trimmed
    }
}

fn or_dash(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "—"
    } else {
        trimmed
    }
}

fn volume_word(count: usize) -> &'static str {
    if count == 1 {
        "volume"
    } else {
        "volumes"
    }
}
