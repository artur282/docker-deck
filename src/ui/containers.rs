use crate::app::App;
use crate::model::{
    ConfirmRequest, ContainerAction, ContainerInfo, DockerCommand, LogTarget, PendingAction,
};
use crate::ui::components;
use eframe::egui;

const NAME_CHARS: usize = 26;
const IMAGE_CHARS: usize = 36;
const STATUS_CHARS: usize = 38;
const PORTS_CHARS: usize = 32;

const HEALTHY_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 200, 130);
const UNHEALTHY_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 110, 110);
const PENDING_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 180, 80);

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let containers = app.snapshot.containers.clone();

    draw_header(ui, &containers);
    draw_toolbar(app, ui);

    let query = app.container_search.trim().to_ascii_lowercase();
    let filtered: Vec<&ContainerInfo> = containers
        .iter()
        .filter(|container| matches_query(container, &query))
        .collect();

    if filtered.is_empty() {
        components::empty_state(ui, empty_message(&query));
        return;
    }

    draw_table(app, ui, &filtered);
}

fn draw_header(ui: &mut egui::Ui, containers: &[ContainerInfo]) {
    let running = containers
        .iter()
        .filter(|container| container.is_running())
        .count();
    components::page_header(
        ui,
        "Containers",
        &format!("{} total · {} running", containers.len(), running),
    );
}

fn draw_toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        components::search_box(
            ui,
            &mut app.container_search,
            "Search name, image, state, project, ports…",
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh").clicked() {
                app.refresh();
            }
        });
    });
    ui.add_space(6.0);
}

fn draw_table(app: &mut App, ui: &mut egui::Ui, containers: &[&ContainerInfo]) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("containers_table")
                .num_columns(5)
                .spacing([16.0, 8.0])
                .min_col_width(90.0)
                .striped(true)
                .show(ui, |ui| {
                    for title in ["Name", "Image", "State", "Status / Ports", "Actions"] {
                        header_label(ui, title);
                    }
                    ui.end_row();

                    for container in containers {
                        name_cell(ui, container);
                        image_cell(ui, container);
                        state_cell(ui, container);
                        status_cell(ui, container);
                        actions_cell(app, ui, container);
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

fn name_cell(ui: &mut egui::Ui, container: &ContainerInfo) {
    ui.vertical(|ui| {
        ui.set_min_width(230.0);
        ui.label(
            egui::RichText::new(components::truncate_middle(&container.name, NAME_CHARS))
                .monospace(),
        )
        .on_hover_text(format!(
            "{} · {}\ncreated {}",
            container.name,
            container.short_id(),
            container.created
        ));

        if let Some(project) = &container.compose_project {
            components::muted(ui, &format!("compose: {project}"));
        }
    });
}

fn image_cell(ui: &mut egui::Ui, container: &ContainerInfo) {
    ui.vertical(|ui| {
        ui.set_min_width(170.0);
        ui.label(egui::RichText::new(components::truncate_middle(
            &container.image,
            IMAGE_CHARS,
        )))
        .on_hover_text(container.image.as_str());
    });
}

fn state_cell(ui: &mut egui::Ui, container: &ContainerInfo) {
    ui.horizontal(|ui| {
        ui.set_min_width(110.0);
        components::badge(
            ui,
            &container.state,
            components::status_color(&container.state),
        );
        let health = container.health.trim();
        if !health.is_empty() && !health.eq_ignore_ascii_case("none") {
            ui.label(
                egui::RichText::new(health)
                    .small()
                    .color(health_color(health)),
            );
        }
    });
}

fn status_cell(ui: &mut egui::Ui, container: &ContainerInfo) {
    ui.vertical(|ui| {
        ui.set_min_width(260.0);
        ui.label(
            egui::RichText::new(components::truncate_middle(&container.status, STATUS_CHARS))
                .small(),
        )
        .on_hover_text(container.status.as_str());

        if container.ports.trim().is_empty() {
            components::muted(ui, "no published ports");
        } else {
            ui.label(
                egui::RichText::new(components::truncate_middle(&container.ports, PORTS_CHARS))
                    .small()
                    .monospace()
                    .color(ui.visuals().weak_text_color()),
            )
            .on_hover_text(container.ports.as_str());
        }
    });
}

fn actions_cell(app: &mut App, ui: &mut egui::Ui, container: &ContainerInfo) {
    ui.horizontal(|ui| {
        ui.set_min_width(280.0);
        ui.add_enabled_ui(!app.busy, |ui| {
            let toggle = if container.is_running() {
                ContainerAction::Stop
            } else {
                ContainerAction::Start
            };
            if ui.button(action_text(toggle)).clicked() {
                app.dispatch(DockerCommand::Container {
                    id: container.id.clone(),
                    name: container.name.clone(),
                    action: toggle,
                });
            }
            if ui.button(action_text(ContainerAction::Restart)).clicked() {
                app.dispatch(DockerCommand::Container {
                    id: container.id.clone(),
                    name: container.name.clone(),
                    action: ContainerAction::Restart,
                });
            }
        });

        if ui.button("Logs").clicked() {
            app.open_logs(LogTarget::Container {
                id: container.id.clone(),
                name: container.name.clone(),
            });
        }

        ui.add_enabled_ui(!app.busy, |ui| {
            if components::danger_button(ui, "Remove").clicked() {
                app.ask(remove_request(container));
            }
        });
    });
}

fn matches_query(container: &ContainerInfo, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let project = container.compose_project.as_deref().unwrap_or_default();
    [
        container.name.as_str(),
        container.image.as_str(),
        container.state.as_str(),
        project,
        container.ports.as_str(),
    ]
    .iter()
    .any(|field| field.to_ascii_lowercase().contains(query))
}

fn empty_message(query: &str) -> &'static str {
    if query.is_empty() {
        "No containers found."
    } else {
        "No containers match your search."
    }
}

fn health_color(health: &str) -> egui::Color32 {
    match health.to_ascii_lowercase().as_str() {
        "healthy" => HEALTHY_COLOR,
        "unhealthy" => UNHEALTHY_COLOR,
        _ => PENDING_COLOR,
    }
}

fn action_text(action: ContainerAction) -> String {
    capitalize(action.label())
}

fn capitalize(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
        None => String::new(),
    }
}

fn remove_request(container: &ContainerInfo) -> ConfirmRequest {
    ConfirmRequest {
        title: "Remove container".to_string(),
        message: format!(
            "Force-remove container \"{}\"? It stops immediately and its writable layer is deleted.",
            container.name
        ),
        action: PendingAction::Container {
            id: container.id.clone(),
            name: container.name.clone(),
            action: ContainerAction::Remove,
        },
        confirm_label: "Remove container".to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}
