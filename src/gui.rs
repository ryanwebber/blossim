use std::time::Instant;

use crate::app;

pub struct Interface {
    info_pane: InfoPane,
}

pub struct InfoPane {
    fps: f32,
    checkpoint_fps_frame: usize,
    checkpoint_fps_time: Instant,
}

impl Interface {
    pub fn new() -> Self {
        Self {
            info_pane: InfoPane {
                fps: 0.0,
                checkpoint_fps_frame: 0,
                checkpoint_fps_time: Instant::now(),
            },
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
        if self.checkpoint_fps_time.elapsed().as_secs_f32() > 0.2 {
            let frames = globals.timing.frame - self.checkpoint_fps_frame;
            self.fps = (frames) as f32 / self.checkpoint_fps_time.elapsed().as_secs_f32();
            self.checkpoint_fps_time = Instant::now();
            self.checkpoint_fps_frame = globals.timing.frame;
        }

        draw_section(ui, "Timing", |ui| {
            ui.label("FPS");
            ui.label(egui::RichText::new(format!("{:.2}", self.fps)).monospace());

            ui.end_row();

            ui.label("Time");
            ui.label(
                egui::RichText::new(format!(
                    "{:.2}",
                    globals.timing.start_time.elapsed().as_secs_f32()
                ))
                .monospace(),
            );
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
