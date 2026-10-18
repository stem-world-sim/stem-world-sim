//! S17 critic: a one-tile harness over sim-core. The UI and the tests call this API only.
//! No physics here: layers, thinning and light all come from `World::query_tile`.

pub use sim_core::{LayerKind, PlantTaxonError, TileOccupantView, TileView};
use sim_core::{Column, SoilLayer, Texture, World, STRESS_THRESHOLDS as TH};

/// Kernel thresholds by index into `STRESS_THRESHOLDS`.
const WILT: u32 = TH[0].2;
const DARK: u32 = TH[1].2;
const WL: u32 = TH[2].2;
const WL_D: u32 = TH[3].2;
const SUB: u32 = TH[4].2;
const CROWD: u32 = TH[5].2;

/// Fixed critic seed (no RNG is used by the kernel paths exercised here).
pub const CRITIC_SEED: u64 = 17;
/// Critic climate: mild temperate, inside the demo pine/oak/rye envelopes.
pub const CRITIC_T_AIR_C: f64 = 20.0;
pub const CRITIC_RAIN_YEAR_MM: f64 = 1000.0;

fn critic_column() -> Column {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    Column::new(0.0, 0.0, vec![SoilLayer::new(0.3, fc, tex), SoilLayer::new(0.5, fc, tex)])
}

/// Seedling preset: 0.10 × mature height.
pub const SEEDLING_FRAC: f64 = 0.10;

/// S17.1 plant height control. Mature is the default.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HeightChoice {
    Seedling,
    Half,
    #[default]
    Mature,
    Metres(f64),
}

/// One tile (1×1 world) driven only through the sim-core API.
pub struct Critic {
    world: World,
    events: Vec<String>,
}

/// S17.2: how many recent events the critic keeps.
pub const EVENT_KEEP: usize = 8;
/// S17.2: rain-button depth (m), added through the kernel rain path.
pub const RAIN_STEP_M: f64 = 0.01;

/// S17.2 reason line from kernel counters only. Plain words.
pub fn reason(o: &TileOccupantView) -> String {
    if !o.alive {
        return death_reason(o);
    }
    let mut parts = Vec::new();
    if o.dry_steps > 0 {
        parts.push(format!("dry {}/{}", o.dry_steps, WILT));
    }
    if o.dark_steps > 0 {
        parts.push(format!("dark {}/{}", o.dark_steps, DARK));
    }
    if o.waterlog_steps > 0 {
        parts.push(format!("waterlogged {}/{}", o.waterlog_steps, WL));
    }
    if o.submerge_steps > 0 {
        parts.push(format!("submerged {}/{}", o.submerge_steps, SUB));
    }
    if o.crowd_steps > 0 {
        parts.push(format!("crowded {}/{}", o.crowd_steps, CROWD));
    }
    if parts.is_empty() { "ok".into() } else { parts.join(", ") }
}

/// Cause for a plant that left the living list, judged from its last counters.
/// A counter one step short of its threshold means the kernel tripped it this tick.
pub fn death_reason(o: &TileOccupantView) -> String {
    if o.dry_steps + 1 >= WILT {
        "wilted: soil dry".into()
    } else if o.submerge_steps + 1 >= SUB {
        "died: submerged".into()
    } else if o.waterlog_steps + 1 >= WL.min(WL_D) && o.waterlog_steps > 0 {
        "died: waterlogged".into()
    } else if o.dark_steps + 1 >= DARK {
        "died: too dark".into()
    } else if o.crowd_steps + 1 >= CROWD {
        "thinned: crowded".into()
    } else {
        "removed".into()
    }
}

impl Default for Critic {
    fn default() -> Self {
        Self::new()
    }
}

impl Critic {
    pub fn new() -> Self {
        let mut world = World::new(CRITIC_SEED, critic_column());
        world.set_climate(CRITIC_T_AIR_C, CRITIC_RAIN_YEAR_MM);
        Self { world, events: Vec::new() }
    }

    /// Plant a mature catalog taxon on the tile.
    pub fn plant(&mut self, taxon: &str) -> Result<(), PlantTaxonError> {
        self.world.plant_taxon(0, 0, taxon)
    }

    /// Plant a catalog taxon at a given crown height in metres (clamped to mature height).
    pub fn plant_height(&mut self, taxon: &str, height_m: f64) -> Result<(), PlantTaxonError> {
        let params = match self.world.catalog().get(taxon) {
            Ok(p) => p.clone(),
            Err(_) => return Err(PlantTaxonError::UnknownTaxon),
        };
        let h_mat = params.height_m_mature.unwrap_or(height_m.max(1e-9));
        self.world.plant_params(0, 0, &params, height_m / h_mat)
    }

    /// Mature crown height (m) of a catalog taxon, if the catalog has one.
    pub fn mature_height(&self, taxon: &str) -> Option<f64> {
        self.world.catalog().get(taxon).ok().and_then(|p| p.height_m_mature)
    }

    /// Plant a catalog taxon at a fraction of its mature height (S17.1 height control).
    pub fn plant_frac(&mut self, taxon: &str, frac: f64) -> Result<(), PlantTaxonError> {
        let params = match self.world.catalog().get(taxon) {
            Ok(p) => p.clone(),
            Err(_) => return Err(PlantTaxonError::UnknownTaxon),
        };
        self.world.plant_params(0, 0, &params, frac)
    }

    /// Plant at the UI's chosen height.
    pub fn plant_choice(&mut self, taxon: &str, h: HeightChoice) -> Result<(), PlantTaxonError> {
        match h {
            HeightChoice::Mature => self.plant(taxon),
            HeightChoice::Half => self.plant_frac(taxon, 0.5),
            HeightChoice::Seedling => self.plant_frac(taxon, SEEDLING_FRAC),
            HeightChoice::Metres(m) => self.plant_height(taxon, m),
        }
    }

    pub fn tick(&mut self) {
        let before = self.query_tile();
        self.world.tick();
        let after = self.query_tile();
        self.diff_events(&before, &after);
    }

    pub fn tick_n(&mut self, k: u32) {
        for _ in 0..k {
            self.tick();
        }
    }

    /// S17.2 rain control: the kernel's existing `add_rain_at` path. No new source.
    pub fn rain(&mut self, depth_m: f64) {
        self.world.add_rain_at(0, 0, depth_m);
        self.push_event(format!("rain {:.0} mm", depth_m * 1000.0));
    }

    /// Last few events, oldest first.
    pub fn events(&self) -> &[String] {
        &self.events
    }

    fn push_event(&mut self, e: String) {
        let t = self.world.tick_count();
        self.events.push(format!("t{t}: {e}"));
        let n = self.events.len();
        if n > EVENT_KEEP {
            self.events.drain(0..n - EVENT_KEEP);
        }
    }

    /// Display label for a taxon id.
    pub fn label(&self, taxon: &str) -> String {
        self.world.catalog().get(taxon).map(|p| p.display_name()).unwrap_or_else(|_| taxon.to_string())
    }

    /// Match before/after living plants by taxon and nearest height_frac; report changes.
    fn diff_events(&mut self, before: &TileView, after: &TileView) {
        let mut used = vec![false; after.occupants.len()];
        let mut gone = Vec::new();
        for b in &before.occupants {
            let mut best: Option<(usize, f64)> = None;
            for (j, a) in after.occupants.iter().enumerate() {
                if used[j] || a.taxon != b.taxon || a.height_frac + 1e-12 < b.height_frac {
                    continue;
                }
                let d = a.height_frac - b.height_frac;
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((j, d));
                }
            }
            match best {
                Some((j, _)) => used[j] = true,
                None => gone.push(b.clone()),
            }
        }
        for g in gone {
            let name = self.label(g.taxon.as_deref().unwrap_or("-"));
            self.push_event(format!("{name}: {}", death_reason(&g)));
        }
        let new: Vec<String> = after
            .occupants
            .iter()
            .zip(used.iter())
            .filter(|(_, u)| !**u)
            .map(|(a, _)| self.label(a.taxon.as_deref().unwrap_or("-")))
            .collect();
        for n in new {
            self.push_event(format!("seed placed: {n}"));
        }
    }

    pub fn query_tile(&self) -> TileView {
        self.world.query_tile(0, 0)
    }

    pub fn tick_count(&self) -> u64 {
        self.world.tick_count()
    }

    /// Demo taxa from the kernel catalog: (taxon_id, `Common (Scientific)`), sorted by label.
    pub fn demo_taxa(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .world
            .catalog()
            .iter()
            .filter(|p| p.demo)
            .map(|p| (p.taxon_id.clone(), p.display_name()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    }

    /// Catalog common name, if any.
    pub fn common_name(&self, taxon: &str) -> Option<String> {
        self.world.catalog().get(taxon).ok().and_then(|p| p.common_name.clone())
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
