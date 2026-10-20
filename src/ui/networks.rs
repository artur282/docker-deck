use crate::app::App;
use crate::model::{ConfirmRequest, NetworkInfo, PendingAction, PruneKind};
use crate::ui::components;
use eframe::egui;

const BUILTIN_BADGE_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 165, 195);
const NAME_TRUNCATE_LENGTH: usize = 42;

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let busy = app.busy;
    let total = app.snapshot.networks.len();
    let networks = filtered_networks(app);

    header(app, ui, total, busy);
    ui.add_space(4.0);
    toolbar(app, ui, busy);
    ui.add_space(6.0);

    if total == 0 {
        components::empty_state(ui, "No Docker networks found.");
    } else if networks.is_empty() {
        components::empty_state(ui, "No networks match this filter.");
    } else {
        table(app, ui, &networks, busy);
    }
}

fn header(app: &mut App, ui: &mut egui::Ui, total: usize, busy: bool) {
    ui.horizontal(|ui| {
        ui.heading("Networks");
        components::muted(ui, &network_count(total));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(!busy, egui::Button::new("Refresh"))
                .clicked()
            {
                app.refresh();
            }
        });
    });
}

fn toolbar(app: &mut App, ui: &mut egui::Ui, busy: bool) {
    ui.horizontal(|ui| {
        components::search_box(
            ui,
            &mut app.network_search,
            "Filter by name, driver or scope…",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if danger_action(ui, !busy, "Prune unused").clicked() {
                app.ask(networks_prune_confirmation());
            }
        });
    });
}

fn table(app: &mut App, ui: &mut egui::Ui, networks: &[NetworkInfo], busy: bool) {
    egui::ScrollArea::vertical()
        .id_salt("networks_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("networks_grid")
                .num_columns(4)
                .striped(true)
                .min_col_width(110.0)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    table_header(ui);
                    for network in networks {
                        row(app, ui, network, busy);
                    }
                });
        });
}

fn table_header(ui: &mut egui::Ui) {
    for label in ["Name", "Driver", "Scope", "Actions"] {
        ui.label(egui::RichText::new(label).strong());
    }
    ui.end_row();
}

fn row(app: &mut App, ui: &mut egui::Ui, network: &NetworkInfo, busy: bool) {
    name_cell(ui, network);
    ui.label(network.driver.as_str());
    components::muted(ui, network.scope.as_str());
    actions_cell(app, ui, network, busy);
    ui.end_row();
}

fn name_cell(ui: &mut egui::Ui, network: &NetworkInfo) {
    ui.horizontal(|ui| {
        let display = components::truncate_middle(&network.name, NAME_TRUNCATE_LENGTH);
        let response = ui.label(egui::RichText::new(display.as_str()).monospace());
        let mut tooltip = network.name.clone();
        if !network.labels.trim().is_empty() {
            tooltip.push_str(&format!("\nlabels: {}", network.labels));
        }
        response.on_hover_text(tooltip);
        if network.builtin {
            components::badge(ui, "built-in", BUILTIN_BADGE_COLOR);
        }
    });
}

fn actions_cell(app: &mut App, ui: &mut egui::Ui, network: &NetworkInfo, busy: bool) {
    let can_remove = !busy && !network.builtin;
    let response = if can_remove {
        components::danger_button(ui, "Remove")
    } else {
        ui.add_enabled(false, egui::Button::new("Remove"))
    };

    let response = if network.builtin {
        response.on_disabled_hover_text("Built-in Docker networks cannot be removed.")
    } else if busy {
        response.on_disabled_hover_text("An operation is in progress.")
    } else {
        response
    };

    if response.clicked() {
        app.ask(network_remove_confirmation(network));
    }
}

fn danger_action(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    if enabled {
        components::danger_button(ui, text)
    } else {
        ui.add_enabled(false, egui::Button::new(text))
            .on_disabled_hover_text("An operation is in progress.")
    }
}

fn filtered_networks(app: &App) -> Vec<NetworkInfo> {
    let query = app.network_search.trim().to_lowercase();
    app.snapshot
        .networks
        .iter()
        .filter(|network| matches_query(network, &query))
        .cloned()
        .collect()
}

fn matches_query(network: &NetworkInfo, query: &str) -> bool {
    query.is_empty()
        || network.name.to_lowercase().contains(query)
        || network.driver.to_lowercase().contains(query)
        || network.scope.to_lowercase().contains(query)
}

fn network_count(total: usize) -> String {
    if total == 1 {
        "1 network".to_string()
    } else {
        format!("{total} networks")
    }
}

fn network_remove_confirmation(network: &NetworkInfo) -> ConfirmRequest {
    ConfirmRequest {
        title: format!("Remove network {}", network.name),
        message: format!(
            "Remove the network \"{}\"? Containers attached to it must be disconnected first. \
             This cannot be undone.",
            network.name
        ),
        action: PendingAction::NetworkRemove(network.id.clone()),
        confirm_label: "Remove network".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}

fn networks_prune_confirmation() -> ConfirmRequest {
    ConfirmRequest {
        title: "Prune networks".to_string(),
        message: "Remove all unused networks? Networks with attached containers are kept. \
                  This cannot be undone."
            .to_string(),
        action: PendingAction::Prune(PruneKind::Networks),
        confirm_label: "Prune networks".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}
