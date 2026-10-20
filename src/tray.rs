use eframe::egui;
use ksni::blocking::TrayMethods;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct DockerDeckTray {
    ctx: egui::Context,
    visible: Arc<AtomicBool>,
}

impl DockerDeckTray {
    fn show(&self) {
        self.visible.store(true, Ordering::Relaxed);
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Visible(true));
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.ctx.request_repaint();
    }

    fn hide(&self) {
        self.visible.store(false, Ordering::Relaxed);
        self.ctx
            .send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn toggle(&self) {
        if self.visible.load(Ordering::Relaxed) {
            self.hide();
        } else {
            self.show();
        }
    }

    fn quit(&self) {
        std::process::exit(0);
    }
}

impl ksni::Tray for DockerDeckTray {
    fn id(&self) -> String {
        "docker-deck".to_string()
    }

    fn title(&self) -> String {
        "Docker Deck".to_string()
    }

    fn icon_name(&self) -> String {
        "docker-deck".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![crate::icon::tray_icon()]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.toggle();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        let visible = self.visible.load(Ordering::Relaxed);
        let (label, icon_name) = if visible {
            ("Hide Docker Deck", "window-close")
        } else {
            ("Show Docker Deck", "window-new")
        };

        vec![
            StandardItem {
                label: label.to_string(),
                icon_name: icon_name.to_string(),
                activate: Box::new(|tray: &mut Self| tray.toggle()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(|tray: &mut Self| tray.quit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn(
    ctx: egui::Context,
    visible: Arc<AtomicBool>,
) -> Option<ksni::blocking::Handle<DockerDeckTray>> {
    let tray = DockerDeckTray { ctx, visible };
    match tray.spawn() {
        Ok(handle) => Some(handle),
        Err(error) => {
            eprintln!("Docker Deck tray unavailable: {error}");
            None
        }
    }
}
