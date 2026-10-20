use crate::app::App;
use crate::model::{
    ComposeAction, ComposeProject, ConfirmRequest, DockerCommand, LogTarget, PendingAction,
};
use crate::ui::components;
use eframe::egui;

enum RowAction {
    Dispatch {
        name: String,
        files: Vec<String>,
        action: ComposeAction,
    },
    Logs {
        name: String,
        files: Vec<String>,
    },
    ConfirmDown {
        name: String,
        files: Vec<String>,
    },
}

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let projects = app.snapshot.compose.clone();
    components::page_header(ui, "Compose", &project_count_label(projects.len()));
    ui.add_space(2.0);
    toolbar(app, ui);

    if projects.is_empty() {
        components::empty_state(ui, "No compose projects found.");
        return;
    }

    let query = app.compose_search.trim().to_lowercase();
    let matching: Vec<ComposeProject> = projects
        .into_iter()
        .filter(|project| matches_query(project, &query))
        .collect();

    if matching.is_empty() {
        components::empty_state(
            ui,
            &format!("No projects match \"{}\".", app.compose_search.trim()),
        );
        return;
    }

    let busy = app.busy;
    let mut clicked_action = None;

    egui::ScrollArea::both()
        .id_salt("compose_projects")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("compose_grid")
                .num_columns(5)
                .striped(true)
                .spacing([16.0, 8.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    header_row(ui);
                    for project in &matching {
                        project_row(ui, project, busy, &mut clicked_action);
                        ui.end_row();
                    }
                });
        });

    if let Some(action) = clicked_action {
        apply(app, action);
    }
}

fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        components::search_box(
            ui,
            &mut app.compose_search,
            "Search projects or config files",
        );
        if ui.button("Refresh").clicked() {
            app.refresh();
        }
    });
    ui.add_space(6.0);
}

fn header_row(ui: &mut egui::Ui) {
    let color = ui.visuals().weak_text_color();
    for title in ["PROJECT", "STATUS", "CONFIG", "OPEN", "ACTIONS"] {
        ui.label(egui::RichText::new(title).small().strong().color(color));
    }
    ui.end_row();
}

fn project_row(
    ui: &mut egui::Ui,
    project: &ComposeProject,
    busy: bool,
    clicked_action: &mut Option<RowAction>,
) {
    ui.label(
        egui::RichText::new(project.name.as_str())
            .monospace()
            .strong(),
    );
    status_badge(ui, project);
    config_cell(ui, project);
    open_cell(ui, project);
    actions_cell(ui, project, busy, clicked_action);
}

fn status_badge(ui: &mut egui::Ui, project: &ComposeProject) {
    let status = project.status.trim();
    let text = if status.is_empty() { "unknown" } else { status };
    let color = if project.is_running() {
        components::status_color("running")
    } else if status.starts_with("exited") || status.starts_with("dead") {
        components::status_color("exited")
    } else {
        ui.visuals().weak_text_color()
    };
    components::badge(ui, text, color);
}

fn config_cell(ui: &mut egui::Ui, project: &ComposeProject) {
    if project.config_files.is_empty() {
        components::muted(ui, "—");
        return;
    }

    let names: Vec<&str> = project
        .config_files
        .iter()
        .filter_map(|path| path.rsplit('/').next())
        .collect();
    let summary = match names.as_slice() {
        [] => "—".to_string(),
        [only] => only.to_string(),
        [first, rest @ ..] => format!("{first} +{}", rest.len()),
    };
    ui.label(egui::RichText::new(summary).monospace().size(12.0))
        .on_hover_text(project.config_files.join("\n"));
}

fn open_cell(ui: &mut egui::Ui, project: &ComposeProject) {
    ui.vertical(|ui| {
        ui.set_min_width(130.0);
        if project.ports.is_empty() {
            components::muted(ui, "—");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for port in &project.ports {
                let tooltip = format!("container {}/{}", port.container_port, port.protocol);
                if let Some(url) = port.url() {
                    ui.hyperlink_to(port.label(), url.as_str())
                        .on_hover_text(format!("Open {url} · {tooltip}"));
                } else {
                    ui.label(egui::RichText::new(port.label()).small().monospace())
                        .on_hover_text(tooltip);
                }
            }
        });
    });
}

fn actions_cell(
    ui: &mut egui::Ui,
    project: &ComposeProject,
    busy: bool,
    clicked_action: &mut Option<RowAction>,
) {
    let running = project.is_running();
    let enabled = !busy;

    ui.horizontal(|ui| {
        action_button(
            ui,
            "Up",
            enabled,
            project,
            ComposeAction::Up,
            clicked_action,
        );
        action_button(
            ui,
            "Stop",
            enabled && running,
            project,
            ComposeAction::Stop,
            clicked_action,
        );
        action_button(
            ui,
            "Start",
            enabled && !running,
            project,
            ComposeAction::Start,
            clicked_action,
        );
        action_button(
            ui,
            "Restart",
            enabled,
            project,
            ComposeAction::Restart,
            clicked_action,
        );

        if ui
            .add(egui::Button::new("Logs").small())
            .on_hover_text("Show compose logs")
            .clicked()
        {
            *clicked_action = Some(RowAction::Logs {
                name: project.name.clone(),
                files: project.config_files.clone(),
            });
        }

        let down_clicked = ui
            .add_enabled_ui(enabled, |ui| components::danger_button(ui, "Down"))
            .inner
            .clicked();
        if down_clicked {
            *clicked_action = Some(RowAction::ConfirmDown {
                name: project.name.clone(),
                files: project.config_files.clone(),
            });
        }
    });
}

fn action_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    project: &ComposeProject,
    action: ComposeAction,
    clicked_action: &mut Option<RowAction>,
) {
    if ui
        .add_enabled(enabled, egui::Button::new(label).small())
        .clicked()
    {
        *clicked_action = Some(RowAction::Dispatch {
            name: project.name.clone(),
            files: project.config_files.clone(),
            action,
        });
    }
}

fn apply(app: &mut App, action: RowAction) {
    match action {
        RowAction::Dispatch {
            name,
            files,
            action,
        } => app.dispatch(DockerCommand::Compose {
            name,
            files,
            action,
        }),
        RowAction::Logs { name, files } => app.open_logs(LogTarget::Compose { name, files }),
        RowAction::ConfirmDown { name, files } => app.ask(down_confirm(name, files)),
    }
}

fn down_confirm(name: String, files: Vec<String>) -> ConfirmRequest {
    ConfirmRequest {
        title: format!("Down · {name}"),
        message: format!(
            "This removes all containers and networks of project \"{name}\". \
             Named volumes are kept unless you select the volume option below."
        ),
        action: PendingAction::Compose {
            name,
            files,
            action: ComposeAction::Down,
        },
        confirm_label: "Down project".to_string(),
        danger: true,
        volume_checkbox: true,
        remove_volumes: false,
    }
}

fn matches_query(project: &ComposeProject, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    project.name.to_lowercase().contains(query)
        || project
            .config_files
            .iter()
            .any(|path| path.to_lowercase().contains(query))
}

fn project_count_label(total: usize) -> String {
    if total == 1 {
        "1 project".to_string()
    } else {
        format!("{total} projects")
    }
}
