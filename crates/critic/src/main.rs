//! S17 critic window (egui). Formats text only; every number comes from `Critic::query_tile`.

use critic::{Critic, HeightChoice, LayerKind};
use eframe::egui;

struct App {
    critic: Critic,
    k: u32,
    playing: bool,
    height: HeightChoice,
    metres: f64,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self { critic: Critic::new(), k: 7, playing: false, height: HeightChoice::Mature, metres: 5.0, status: String::new() }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.playing {
            // Play: advance K ticks, then wait before the next burst.
            self.critic.tick_n(self.k);
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("stem-world-sim critic — one tile");
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.radio_value(&mut self.height, HeightChoice::Seedling, "seedling (0.10×)");
                ui.radio_value(&mut self.height, HeightChoice::Half, "half");
                ui.radio_value(&mut self.height, HeightChoice::Mature, "mature");
                let is_m = matches!(self.height, HeightChoice::Metres(_));
                if ui.radio(is_m, "metres").clicked() || is_m {
                    self.height = HeightChoice::Metres(self.metres);
                }
                if ui.add(egui::DragValue::new(&mut self.metres).range(0.01..=100.0).speed(0.1).suffix(" m")).changed() {
                    self.height = HeightChoice::Metres(self.metres);
                }
            });
            ui.label("Plant:");
            ui.horizontal_wrapped(|ui| {
                for (id, name) in self.critic.demo_taxa() {
                    if ui.button(&name).on_hover_text(&id).clicked() {
                        self.status = match self.critic.plant_choice(&id, self.height) {
                            Ok(()) => format!("planted {id}"),
                            Err(e) => format!("{id}: {e:?}"),
                        };
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Tick").clicked() {
                    self.critic.tick();
                }
                ui.add(egui::DragValue::new(&mut self.k).range(1..=1000).prefix("K="));
                if ui.button(format!("Tick×{}", self.k)).clicked() {
                    self.critic.tick_n(self.k);
                }
                let label = if self.playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    self.playing = !self.playing;
                }
                if ui.button("Reset").clicked() {
                    self.critic.reset();
                    self.playing = false;
                }
            });
            ui.label(&self.status);
            let q = self.critic.query_tile();
            ui.separator();
            ui.label(format!("tick {}   living {}/{}", self.critic.tick_count(), q.living, q.cap));
            egui::Grid::new("layers").striped(true).show(ui, |ui| {
                for h in ["slot", "taxon", "height_m", "h_frac", "layer", "A", "over", "light", "alive"] {
                    ui.strong(h);
                }
                ui.end_row();
                let mut rows = q.occupants.clone();
                rows.sort_by(|a, b| b.height_m.partial_cmp(&a.height_m).unwrap_or(std::cmp::Ordering::Equal));
                for o in rows {
                    let kind = match o.layer_kind {
                        LayerKind::Woody => "woody",
                        LayerKind::Herb => "herb",
                    };
                    ui.label(o.slot.to_string());
                    ui.label(o.taxon.clone().unwrap_or_else(|| "-".into()));
                    ui.label(format!("{:.2}", o.height_m));
                    ui.label(format!("{:.3}", o.height_frac));
                    ui.label(format!("{kind} {}", o.layer_id));
                    ui.label(format!("{:.3}", o.a));
                    ui.label(o.over.to_string());
                    ui.label(o.light.map(|l| format!("{l:.3}")).unwrap_or_else(|| "-".into()));
                    ui.label(o.alive.to_string());
                    ui.end_row();
                }
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "stem-world-sim critic",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("critic_canvas"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("canvas #critic_canvas");
        eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|_cc| Ok(Box::new(App::default()))))
            .await
            .expect("start critic");
    });
}
