use crate::docker;
use crate::model::*;
use crate::tray;
use crate::ui;
use eframe::egui;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

pub struct App {
    pub tab: Tab,
    pub snapshot: Snapshot,
    pub busy: bool,
    pub auto_refresh: bool,
    pub status: String,
    pub error: Option<String>,
    pub confirm: Option<ConfirmRequest>,
    pub logs: Option<LogsView>,
    pub compose_search: String,
    pub volume_search: String,
    pub container_search: String,
    pub image_search: String,
    pub network_search: String,
    pub selected_volumes: BTreeSet<String>,
    pub dark_mode: bool,
    quit_requested: bool,
    visible: Arc<AtomicBool>,
    command_tx: Sender<DockerCommand>,
    event_rx: Receiver<DockerEvent>,
    _tray_handle: Option<ksni::blocking::Handle<tray::DockerDeckTray>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_theme(&cc.egui_ctx);
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        docker::spawn_worker(cc.egui_ctx.clone(), command_rx, event_tx);
        let visible = Arc::new(AtomicBool::new(true));
        let tray_handle = tray::spawn(cc.egui_ctx.clone(), visible.clone());

        let mut app = Self {
            tab: Tab::Compose,
            snapshot: Snapshot::default(),
            busy: false,
            auto_refresh: true,
            status: "Connecting to Docker…".to_string(),
            error: None,
            confirm: None,
            logs: None,
            compose_search: String::new(),
            volume_search: String::new(),
            container_search: String::new(),
            image_search: String::new(),
            network_search: String::new(),
            selected_volumes: BTreeSet::new(),
            dark_mode: true,
            quit_requested: false,
            visible,
            command_tx,
            event_rx,
            _tray_handle: tray_handle,
        };
        app.dispatch(DockerCommand::Refresh);
        app
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.dark_mode = !self.dark_mode;
        ctx.set_theme(if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
    }

    pub fn dispatch(&mut self, command: DockerCommand) {
        self.busy = true;
        self.error = None;
        self.status = match command {
            DockerCommand::Refresh => "Refreshing…".to_string(),
            _ => "Working…".to_string(),
        };
        let _ = self.command_tx.send(command);
    }

    pub fn refresh(&mut self) {
        self.dispatch(DockerCommand::Refresh);
    }

    pub fn ask(&mut self, request: ConfirmRequest) {
        self.confirm = Some(request);
    }

    pub fn open_logs(&mut self, target: LogTarget) {
        self.status = "Loading logs…".to_string();
        self.dispatch(DockerCommand::LoadLogs(target));
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                DockerEvent::Snapshot(snapshot) => {
                    self.snapshot = *snapshot;
                    self.busy = false;
                    self.status = "Up to date".to_string();
                    self.selected_volumes.retain(|name| {
                        self.snapshot
                            .volumes
                            .iter()
                            .any(|volume| &volume.name == name)
                    });
                }
                DockerEvent::Done(message) => {
                    self.status = message;
                    self.busy = false;
                }
                DockerEvent::Error(error) => {
                    self.error = Some(error);
                    self.busy = false;
                    self.status = "Operation failed".to_string();
                }
                DockerEvent::Logs(logs) => {
                    self.logs = Some(logs);
                    self.busy = false;
                    self.status = "Logs loaded".to_string();
                }
            }
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) || self.quit_requested {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.visible.store(false, Ordering::Relaxed);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.status = "Hidden in tray".to_string();
    }

    fn execute_action(&mut self, action: PendingAction, remove_volumes: bool) {
        match action {
            PendingAction::Compose {
                name,
                files,
                action,
            } => {
                let action = if remove_volumes && action == ComposeAction::Down {
                    ComposeAction::DownVolumes
                } else {
                    action
                };
                self.dispatch(DockerCommand::Compose {
                    name,
                    files,
                    action,
                });
            }
            PendingAction::VolumeRemove(names) => {
                self.selected_volumes.clear();
                self.dispatch(DockerCommand::VolumeRemove(names));
            }
            PendingAction::Container { id, name, action } => {
                self.dispatch(DockerCommand::Container { id, name, action });
            }
            PendingAction::ImageRemove(id) => {
                self.dispatch(DockerCommand::ImageRemove(id));
            }
            PendingAction::NetworkRemove(id) => {
                self.dispatch(DockerCommand::NetworkRemove(id));
            }
            PendingAction::Prune(kind) => {
                self.dispatch(DockerCommand::Prune(kind));
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Docker Deck");
            ui.label(
                egui::RichText::new(format!("engine {}", self.snapshot.engine_version))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.busy {
                    ui.add(egui::Spinner::new().size(16.0));
                }
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
                let theme_label = if self.dark_mode {
                    "Light theme"
                } else {
                    "Dark theme"
                };
                if ui
                    .button(theme_label)
                    .on_hover_text("Switch between light and dark theme")
                    .clicked()
                {
                    self.toggle_theme(ui.ctx());
                }
                ui.checkbox(&mut self.auto_refresh, "Auto refresh");
                if let Some(refreshed_at) = self.snapshot.refreshed_at {
                    ui.label(
                        egui::RichText::new(format!(
                            "updated {}s ago",
                            refreshed_at.elapsed().as_secs()
                        ))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                    );
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("RESOURCES")
                .small()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(4.0);

        for tab in Tab::ALL {
            let count = match tab {
                Tab::Compose => self.snapshot.compose.len(),
                Tab::Volumes => self.snapshot.volumes.len(),
                Tab::Containers => self.snapshot.containers.len(),
                Tab::Images => self.snapshot.images.len(),
                Tab::Networks => self.snapshot.networks.len(),
                Tab::System => self.snapshot.system.len(),
            };
            let label = format!("{}   {}", tab.label(), count);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::selectable(self.tab == tab, label),
                )
                .clicked()
            {
                self.tab = tab;
            }
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(8.0);
            if let Some(refreshed_at) = self.snapshot.refreshed_at {
                ui.label(
                    egui::RichText::new(format!(
                        "Last refresh: {}s ago",
                        refreshed_at.elapsed().as_secs()
                    ))
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
            }
            ui.label(
                egui::RichText::new("native engine\nunix:///var/run/docker.sock")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let color = if self.busy {
                egui::Color32::from_rgb(230, 180, 80)
            } else if self.error.is_some() {
                egui::Color32::from_rgb(235, 100, 100)
            } else {
                egui::Color32::from_rgb(120, 190, 120)
            };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, color);
            ui.label(&self.status);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} compose · {} volumes · {} containers",
                        self.snapshot.compose.len(),
                        self.snapshot.volumes.len(),
                        self.snapshot.containers.len()
                    ))
                    .small()
                    .color(ui.visuals().weak_text_color()),
                );
            });
        });
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = self.error.clone() {
            let dark = ui.visuals().dark_mode;
            egui::Frame::new()
                .fill(if dark {
                    egui::Color32::from_rgb(70, 28, 28)
                } else {
                    egui::Color32::from_rgb(255, 232, 232)
                })
                .inner_margin(egui::Margin::symmetric(10, 8))
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(error).color(if dark {
                            egui::Color32::from_rgb(255, 210, 210)
                        } else {
                            egui::Color32::from_rgb(140, 30, 30)
                        }));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Dismiss").clicked() {
                                self.error = None;
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }

        match self.tab {
            Tab::Compose => ui::compose::draw(self, ui),
            Tab::Volumes => ui::volumes::draw(self, ui),
            Tab::Containers => ui::containers::draw(self, ui),
            Tab::Images => ui::images::draw(self, ui),
            Tab::Networks => ui::networks::draw(self, ui),
            Tab::System => ui::system::draw(self, ui),
        }
    }

    fn confirm_modal(&mut self, ui: &mut egui::Ui) {
        let Some(mut confirm) = self.confirm.take() else {
            return;
        };

        let mut confirmed = false;
        let mut cancelled = false;

        egui::Modal::new(egui::Id::new("confirm-modal")).show(ui.ctx(), |ui| {
            ui.set_min_width(460.0);
            ui.heading(&confirm.title);
            ui.add_space(4.0);
            ui.label(&confirm.message);
            ui.add_space(8.0);
            if confirm.volume_checkbox {
                ui.checkbox(
                    &mut confirm.remove_volumes,
                    "Also remove named volumes (down -v)",
                );
                ui.label(
                    egui::RichText::new("This permanently deletes the project's data volumes.")
                        .small()
                        .color(egui::Color32::from_rgb(230, 140, 120)),
                );
                ui.add_space(6.0);
            }
            if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
                let button = egui::Button::new(
                    egui::RichText::new(&confirm.confirm_label)
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(if confirm.danger {
                    egui::Color32::from_rgb(170, 45, 45)
                } else {
                    egui::Color32::from_rgb(45, 100, 170)
                });
                if ui.add(button).clicked() {
                    confirmed = true;
                }
            });
        });

        if confirmed {
            self.execute_action(confirm.action, confirm.remove_volumes);
        } else if !cancelled {
            self.confirm = Some(confirm);
        }
    }

    fn logs_window(&mut self, ui: &mut egui::Ui) {
        let Some(logs) = &self.logs else {
            return;
        };
        let mut open = true;
        egui::Window::new(&logs.title)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(900.0)
            .default_height(560.0)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&logs.text).monospace().size(12.0),
                            )
                            .selectable(true),
                        );
                    });
            });
        if !open {
            self.logs = None;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui
            .ctx()
            .input(|input| input.modifiers.command && input.key_pressed(egui::Key::T))
        {
            self.toggle_theme(ui.ctx());
        }
        if ui
            .ctx()
            .input(|input| input.modifiers.command && input.key_pressed(egui::Key::R))
            && !self.busy
        {
            self.refresh();
        }
        if ui
            .ctx()
            .input(|input| input.modifiers.command && input.key_pressed(egui::Key::Q))
        {
            self.quit_requested = true;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        self.poll_events();
        self.handle_close_request(ui.ctx());

        if self.auto_refresh
            && !self.busy
            && self.confirm.is_none()
            && self
                .snapshot
                .refreshed_at
                .map(|instant| instant.elapsed() >= Duration::from_secs(30))
                .unwrap_or(false)
        {
            self.refresh();
        }

        if self.auto_refresh || self.busy {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
        }

        egui::Panel::top("top_bar")
            .exact_size(38.0)
            .show(ui, |ui| self.top_bar(ui));

        egui::Panel::left("navigation")
            .resizable(false)
            .exact_size(180.0)
            .show(ui, |ui| self.sidebar(ui));

        egui::Panel::bottom("status_bar")
            .exact_size(26.0)
            .show(ui, |ui| self.status_bar(ui));

        egui::CentralPanel::default().show(ui, |ui| self.content(ui));

        self.confirm_modal(ui);
        self.logs_window(ui);
    }
}

fn configure_theme(ctx: &egui::Context) {
    let mut dark = egui::Visuals::dark();
    dark.panel_fill = egui::Color32::from_rgb(24, 26, 30);
    dark.window_fill = egui::Color32::from_rgb(30, 33, 38);
    dark.extreme_bg_color = egui::Color32::from_rgb(17, 19, 22);
    dark.faint_bg_color = egui::Color32::from_rgb(29, 32, 37);
    dark.selection.bg_fill = egui::Color32::from_rgb(45, 100, 170);
    dark.hyperlink_color = egui::Color32::from_rgb(100, 170, 240);

    let mut light = egui::Visuals::light();
    light.panel_fill = egui::Color32::from_rgb(246, 247, 249);
    light.window_fill = egui::Color32::from_rgb(255, 255, 255);
    light.extreme_bg_color = egui::Color32::from_rgb(235, 237, 241);
    light.faint_bg_color = egui::Color32::from_rgb(240, 242, 245);
    light.selection.bg_fill = egui::Color32::from_rgb(45, 100, 170);
    light.hyperlink_color = egui::Color32::from_rgb(30, 100, 200);

    for (theme, visuals) in [(egui::Theme::Dark, dark), (egui::Theme::Light, light)] {
        let mut style = (*ctx.style_of(theme)).clone();
        style.visuals = visuals;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_style_of(theme, style);
    }
    ctx.set_theme(egui::Theme::Dark);
}
