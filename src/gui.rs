use crate::app;

pub struct Interface {
    info_pane: InfoPane,
}

pub struct InfoPane;

impl Interface {
    pub fn new() -> Self {
        Self {
            info_pane: InfoPane,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, globals: &mut app::Globals) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |_| {
                egui::Window::new("Info")
                    .default_open(true)
                    .show(ctx, |ui: &mut egui::Ui| {
                        self.info_pane.ui(ui, globals);
                    });
            });
    }
}

impl InfoPane {
    fn ui(&mut self, ui: &mut egui::Ui, globals: &mut app::Globals) {
        draw_section(ui, "Timing", |ui| {
            ui.label("FPS");
            ui.label(egui::RichText::new(format!("{:.2}", globals.timing.avs_fps)).monospace());

            ui.end_row();

            ui.label("Time");
            ui.label(egui::RichText::new(format!("{:.2}", globals.timing.time)).monospace());
        });
    }
}

fn draw_section<F>(ui: &mut egui::Ui, name: &'static str, builder: F)
where
    F: FnOnce(&mut egui::Ui),
{
    egui::CollapsingHeader::new(name)
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new(name)
                .striped(true)
                .spacing([10.0, 10.0])
                .min_col_width(100.0)
                .show(ui, |ui| {
                    builder(ui);
                });
        });
}
