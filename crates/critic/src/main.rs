//! S17/S17.2 critic window (egui). Narrow widths stack and wrap. Formats text only; every number comes from `Critic::query_tile`.

use critic::{fmt_sim_time, reason, Critic, HeightChoice, LayerKind, TickUnit, RAIN_STEP_M};
use sim_core::STRESS_THRESHOLDS;
use eframe::egui;

struct App {
    critic: Critic,
    k: u32,
    playing: bool,
    height: HeightChoice,
    metres: f64,
    status: String,
    show_key: bool,
    /// S18 text size (egui zoom factor).
    text_scale: f32,
    /// S18 continuous rain, mm per tick (UI copy of the kernel setting).
    rain_mm: f64,
}

impl Default for App {
    fn default() -> Self {
        Self { critic: Critic::new(), k: 7, playing: false, height: HeightChoice::Mature, metres: 5.0, status: String::new(), show_key: false, text_scale: 1.0, rain_mm: 0.0 }
    }
}

/// Width (points) below which the page uses the stacked phone layout.
const NARROW_W: f32 = 600.0;

const KEY: &[(&str, &str)] = &[
    ("living/cap", "living plants on the tile / most the tile can hold"),
    ("pond", "standing water on the surface (m)"),
    ("θ", "soil water content of a layer (m³/m³)"),
    ("θ_fc", "field capacity: θ the layer holds after draining"),
    ("height_m", "crown height in metres"),
    ("h_frac", "height as a fraction of mature height (≥0.5 sets seed)"),
    ("layer", "woody = tree/shrub canopy layer, herb = 5 cm herb band; id from tallest"),
    ("A", "crowd load of the layer: sum of α·h_frac²; above 1 is overcrowded"),
    ("over", "ticks in a row the layer has had A > 1"),
    ("light", "light reaching the plant (1 = full sun)"),
    ("roots", "current root depth (catalog root depth × h_frac) and deepest soil layer reached"),
    ("tick =", "length of one tick; every rate (growth, ET, infiltration, uptake) scales with it"),
    ("rain/tick", "rain added automatically on every tick (mm)"),
];

impl App {
    fn controls(&mut self, ui: &mut egui::Ui, narrow: bool) {
        let height_row = |ui: &mut egui::Ui, s: &mut Self| {
            ui.label("Height:");
            ui.radio_value(&mut s.height, HeightChoice::Seedling, "seedling (0.10×)");
            ui.radio_value(&mut s.height, HeightChoice::Half, "half");
            ui.radio_value(&mut s.height, HeightChoice::Mature, "mature");
            let is_m = matches!(s.height, HeightChoice::Metres(_));
            if ui.radio(is_m, "metres").clicked() || is_m {
                s.height = HeightChoice::Metres(s.metres);
            }
            if ui.add(egui::DragValue::new(&mut s.metres).range(0.01..=100.0).speed(0.1).suffix(" m")).changed() {
                s.height = HeightChoice::Metres(s.metres);
            }
        };
        ui.horizontal_wrapped(|ui| {
            ui.label("Text size:");
            ui.add(egui::Slider::new(&mut self.text_scale, 0.75..=2.5).step_by(0.05).suffix("×"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Tick =");
            let cur = self.critic.tick_unit();
            for u in [TickUnit::Second, TickUnit::Day, TickUnit::Month, TickUnit::Year] {
                if ui.radio(cur == u, u.label()).clicked() && cur != u {
                    self.critic.set_tick_unit(u);
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Rain per tick:");
            if ui.add(egui::DragValue::new(&mut self.rain_mm).range(0.0..=500.0).speed(0.1).suffix(" mm")).changed() {
                self.critic.set_rain_per_tick_m(self.rain_mm / 1000.0);
            }
        });
        ui.horizontal_wrapped(|ui| height_row(ui, self));
        ui.label("Plant:");
        ui.horizontal_wrapped(|ui| {
            for (id, name) in self.critic.demo_taxa() {
                let b = egui::Button::new(&name);
                let b = if narrow { b.min_size(egui::vec2(ui.available_width().min(260.0), 40.0)) } else { b };
                if ui.add(b).on_hover_text(&id).clicked() {
                    self.status = match self.critic.plant_choice(&id, self.height) {
                        Ok(()) => format!("planted {name}"),
                        Err(e) => format!("{name}: {e:?}"),
                    };
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
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
            if ui.button(format!("Rain {:.0} mm", RAIN_STEP_M * 1000.0)).clicked() {
                self.critic.rain(RAIN_STEP_M);
            }
            if ui.button("Reset").clicked() {
                self.critic.reset();
                self.playing = false;
            }
            if ui.button(if self.show_key { "Hide key" } else { "Key" }).clicked() {
                self.show_key = !self.show_key;
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.playing {
            // Play: advance K ticks, then wait before the next burst.
            self.critic.tick_n(self.k);
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        if (ctx.zoom_factor() - self.text_scale).abs() > 1e-3 {
            ctx.set_zoom_factor(self.text_scale);
        }
        let narrow = ctx.screen_rect().width() < NARROW_W;
        ctx.style_mut(|s| {
            s.spacing.interact_size.y = if narrow { 36.0 } else { 20.0 };
            s.spacing.button_padding = if narrow { egui::vec2(10.0, 8.0) } else { egui::vec2(4.0, 1.0) };
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("stem-world-sim critic — one tile");
                self.controls(ui, narrow);
                ui.label(&self.status);
                if self.show_key {
                    ui.group(|ui| {
                        ui.strong("Key");
                        for (k, v) in KEY {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(*k);
                                ui.label(*v);
                            });
                        }
                        ui.strong("Stress counters (count up each bad step; plant dies or is thinned at the threshold)");
                        for (counter, name, v) in STRESS_THRESHOLDS {
                            ui.label(format!("{counter} n/{v}: steps toward {name}={v}"));
                        }
                        ui.label("dry: no water taken; dark: below minimum light; waterlogged: saturated soil; submerged: under pond; crowded: layer A > 1, weakest thinned");
                    });
                }
                let q = self.critic.query_tile();
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("tick {}", q.tick));
                    ui.label(format!("time {}", fmt_sim_time(q.sim_time_s)));
                    ui.label(format!("tick = {}", self.critic.tick_unit().label()));
                    ui.label(format!("rain/tick {:.1} mm", q.rain_per_tick_m * 1000.0));
                    ui.label(format!("living {}/{}", q.living, q.cap));
                    ui.label(format!("pond {:.4} m", q.pond_m));
                    for (i, (th, fc)) in q.layers.iter().enumerate() {
                        ui.label(format!("layer{i} θ {th:.3} / θ_fc {fc:.3}"));
                    }
                });
                ui.label("Events:");
                for e in self.critic.events().iter().rev() {
                    ui.small(e);
                }
                ui.separator();
                let mut rows = q.occupants.clone();
                rows.sort_by(|a, b| b.height_m.partial_cmp(&a.height_m).unwrap_or(std::cmp::Ordering::Equal));
                for o in rows {
                    let kind = match o.layer_kind {
                        LayerKind::Woody => "woody",
                        LayerKind::Herb => "herb",
                    };
                    let name = self.critic.label(o.taxon.as_deref().unwrap_or("-"));
                    ui.group(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(format!("#{} {name}", o.slot));
                            ui.label(format!("— {}", reason(&o)));
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("height_m {:.2}", o.height_m));
                            ui.label(format!("h_frac {:.3}", o.height_frac));
                            ui.label(format!("layer {kind} {}", o.layer_id));
                            ui.label(format!("A {:.3}", o.a));
                            ui.label(format!("over {}", o.over));
                            ui.label(format!("light {}", o.light.map(|l| format!("{l:.3}")).unwrap_or_else(|| "-".into())));
                            ui.label(format!("alive {}", o.alive));
                            ui.label(match (o.root_depth_m, o.max_layer) {
                                (Some(r), Some(k)) => format!("roots {r:.2} m → layer{k}"),
                                (None, Some(k)) => format!("roots → layer{k}"),
                                _ => "roots -".into(),
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            let th = STRESS_THRESHOLDS;
                            ui.small(format!(
                                "dry {}/{}  dark {}/{}  waterlogged {}/{}  submerged {}/{}  crowded {}/{}",
                                o.dry_steps, th[0].2, o.dark_steps, th[1].2, o.waterlog_steps, th[2].2,
                                o.submerge_steps, th[4].2, o.crowd_steps, th[5].2
                            ));
                        });
                    });
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
