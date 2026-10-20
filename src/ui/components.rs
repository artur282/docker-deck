use eframe::egui;

pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.label(
            egui::RichText::new(subtitle)
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    });
    ui.add_space(4.0);
}

pub fn search_box(ui: &mut egui::Ui, value: &mut String, hint: &str) {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(280.0),
    );
}

pub fn badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.22))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}

pub fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    if ui.is_enabled() {
        let dark = ui.visuals().dark_mode;
        let (fill, text_color) = if dark {
            (
                egui::Color32::from_rgb(70, 30, 30),
                egui::Color32::from_rgb(255, 170, 160),
            )
        } else {
            (
                egui::Color32::from_rgb(255, 235, 235),
                egui::Color32::from_rgb(150, 30, 30),
            )
        };
        ui.add(egui::Button::new(egui::RichText::new(text).color(text_color)).fill(fill))
    } else {
        ui.add(egui::Button::new(text))
    }
}

pub fn empty_state(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(
            egui::RichText::new(text)
                .color(ui.visuals().weak_text_color())
                .size(15.0),
        );
        ui.add_space(40.0);
    });
}

pub fn truncate_middle(value: &str, max: usize) -> String {
    let count = value.chars().count();
    if count <= max || max < 8 {
        return value.to_string();
    }
    let left = (max - 1) / 2;
    let right = max - 1 - left;
    let start: String = value.chars().take(left).collect();
    let end: String = value
        .chars()
        .rev()
        .take(right)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{start}…{end}")
}

pub fn status_color(state: &str) -> egui::Color32 {
    match state.to_ascii_lowercase().as_str() {
        "running" => egui::Color32::from_rgb(110, 200, 130),
        "exited" | "dead" => egui::Color32::from_rgb(220, 110, 110),
        "paused" | "restarting" => egui::Color32::from_rgb(230, 180, 80),
        _ => egui::Color32::from_gray(160),
    }
}

pub fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .small()
            .color(ui.visuals().weak_text_color()),
    );
}
