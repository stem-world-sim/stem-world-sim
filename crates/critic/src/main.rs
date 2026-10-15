//! S17 critic window (egui). Formats text only; every number comes from `Critic::query_tile`.

use critic::{Critic, LayerKind};
use eframe::egui;

struct App {
    critic: Critic,
    k: u32,
    playing: bool,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self { critic: Critic::new(), k: 7, playing: false, status: String::new() }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.playing {
            self.critic.tick();
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("stem-world-sim critic — one tile");
            ui.label("Plant (mature):");
            ui.horizontal_wrapped(|ui| {
                for (id, name) in self.critic.demo_taxa() {
                    if ui.button(&name).on_hover_text(&id).clicked() {
                        self.status = match self.critic.plant(&id) {
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
