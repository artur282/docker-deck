use crate::app::App;
use crate::model::{ConfirmRequest, PendingAction, PruneKind, SystemDf};
use crate::ui::components;
use eframe::egui;

const DOCKER_SOCKET: &str = "unix:///var/run/docker.sock";
const RECLAIMABLE_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 180, 80);

struct PruneAction {
    kind: PruneKind,
    title: &'static str,
    description: &'static str,
    button: &'static str,
}

const PRUNE_ACTIONS: [PruneAction; 5] = [
    PruneAction {
        kind: PruneKind::Containers,
        title: "Stopped containers",
        description: "Containers that have exited and are not running.",
        button: "Prune containers",
    },
    PruneAction {
        kind: PruneKind::Images,
        title: "Dangling images",
        description: "Untagged images with no dependent containers.",
        button: "Prune images",
    },
    PruneAction {
        kind: PruneKind::Volumes,
        title: "Unused volumes",
        description: "Unused volumes. Pruning permanently deletes the data stored inside them.",
        button: "Prune volumes",
    },
    PruneAction {
        kind: PruneKind::Networks,
        title: "Unused networks",
        description: "Networks with no attached containers.",
        button: "Prune networks",
    },
    PruneAction {
        kind: PruneKind::BuildCache,
        title: "Build cache",
        description: "Cached build layers that are no longer used.",
        button: "Prune build cache",
    },
];

pub fn draw(app: &mut App, ui: &mut egui::Ui) {
    let busy = app.busy;
    let engine_version = app.snapshot.engine_version.clone();
    let system = app.snapshot.system.clone();

    header(ui, &engine_version);
    ui.add_space(6.0);

    let requested = egui::ScrollArea::vertical()
        .id_salt("system_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            disk_usage(ui, &system);
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);
            prune_section(ui, !busy)
        })
        .inner;

    if let Some(kind) = requested {
        app.ask(prune_confirmation(kind));
    }
}

fn header(ui: &mut egui::Ui, engine_version: &str) {
    ui.horizontal(|ui| {
        ui.heading("System");
        let version = if engine_version.trim().is_empty() {
            "unknown".to_string()
        } else {
            engine_version.to_string()
        };
        components::muted(ui, &format!("engine {version}"));
        components::muted(ui, DOCKER_SOCKET);
    });
}

fn disk_usage(ui: &mut egui::Ui, system: &[SystemDf]) {
    ui.label(egui::RichText::new("Disk usage").strong().size(16.0));
    ui.add_space(2.0);

    if system.is_empty() {
        components::empty_state(ui, "No disk usage data reported by Docker.");
        return;
    }

    egui::Grid::new("system_df_grid")
        .num_columns(5)
        .striped(true)
        .min_col_width(90.0)
        .spacing([28.0, 8.0])
        .show(ui, |ui| {
            for label in ["Type", "Total", "Active", "Size", "Reclaimable"] {
                ui.label(egui::RichText::new(label).strong());
            }
            ui.end_row();

            for entry in system {
                ui.label(entry.kind.as_str());
                ui.label(entry.total.to_string());
                ui.label(entry.active.to_string());
                ui.label(entry.size.as_str());

                let reclaimable = entry.reclaimable.trim();
                let text = egui::RichText::new(reclaimable);
                if reclaimable.is_empty() || reclaimable == "0" || reclaimable == "0B" {
                    ui.label(text);
                } else {
                    ui.label(text.color(RECLAIMABLE_COLOR));
                }
                ui.end_row();
            }
        });
}

fn prune_section(ui: &mut egui::Ui, enabled: bool) -> Option<PruneKind> {
    ui.label(egui::RichText::new("Prune").strong().size(16.0));
    ui.label(
        egui::RichText::new(
            "Prune removes only resources Docker reports as unused: stopped containers, dangling \
             images, unused networks and build cache. Volumes need extra care — an unused volume \
             can still hold data, and pruning it permanently deletes that data.",
        )
        .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(10.0);

    let mut requested = None;
    for action in &PRUNE_ACTIONS {
        if prune_row(ui, action, enabled) {
            requested = Some(action.kind);
        }
        ui.add_space(6.0);
    }
    requested
}

fn prune_row(ui: &mut egui::Ui, action: &PruneAction, enabled: bool) -> bool {
    let is_volume = action.kind == PruneKind::Volumes;
    let dark = ui.visuals().dark_mode;
    let fill = if is_volume {
        if dark {
            egui::Color32::from_rgb(46, 30, 32)
        } else {
            egui::Color32::from_rgb(255, 238, 238)
        }
    } else {
        ui.visuals().faint_bg_color
    };
    let volume_text_color = if dark {
        egui::Color32::from_rgb(235, 160, 150)
    } else {
        egui::Color32::from_rgb(150, 30, 30)
    };
    let text_width = (ui.available_width() - 190.0).max(260.0);
    let mut clicked = false;

    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .corner_radius(6)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(text_width);
                    ui.label(egui::RichText::new(action.title).strong());
                    if is_volume {
                        ui.label(
                            egui::RichText::new(action.description)
                                .small()
                                .color(volume_text_color),
                        );
                    } else {
                        components::muted(ui, action.description);
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    clicked = danger_action(ui, enabled, action.button).clicked();
                });
            });
        });

    clicked
}

fn danger_action(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    if enabled {
        components::danger_button(ui, text)
    } else {
        ui.add_enabled(false, egui::Button::new(text))
            .on_disabled_hover_text("An operation is in progress.")
    }
}

fn prune_confirmation(kind: PruneKind) -> ConfirmRequest {
    let (message, confirm_label) = match kind {
        PruneKind::Containers => (
            "Remove all stopped containers? Running containers are kept. This cannot be undone.",
            "Prune containers",
        ),
        PruneKind::Images => (
            "Remove all dangling images? Images referenced by a tag or container are kept. \
             This cannot be undone.",
            "Prune images",
        ),
        PruneKind::Volumes => (
            "Remove the unused volumes Docker reports as reclaimable? Their data is permanently \
             deleted and cannot be undone. On current Docker engines this removes anonymous \
             volumes; named volumes are kept.",
            "Prune volumes",
        ),
        PruneKind::Networks => (
            "Remove all unused networks? Networks with attached containers are kept. \
             This cannot be undone.",
            "Prune networks",
        ),
        PruneKind::BuildCache => (
            "Remove all unused build cache? Cached layers not used by current builds are \
             deleted. This cannot be undone.",
            "Prune build cache",
        ),
    };

    ConfirmRequest {
        title: format!("Prune {}", kind.label()),
        message: message.to_string(),
        action: PendingAction::Prune(kind),
        confirm_label: confirm_label.to_string(),
        danger: true,
        volume_checkbox: false,
        remove_volumes: false,
    }
}
