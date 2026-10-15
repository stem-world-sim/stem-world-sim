//! S17 critic: a one-tile harness over sim-core. The UI and the tests call this API only.
//! No physics here: layers, thinning and light all come from `World::query_tile`.

pub use sim_core::{LayerKind, PlantTaxonError, TileOccupantView, TileView};
use sim_core::{Column, SoilLayer, Texture, World};

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

/// One tile (1×1 world) driven only through the sim-core API.
pub struct Critic {
    world: World,
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
        Self { world }
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

    pub fn tick(&mut self) {
        self.world.tick();
    }

    pub fn tick_n(&mut self, k: u32) {
        for _ in 0..k {
            self.world.tick();
        }
    }

    pub fn query_tile(&self) -> TileView {
        self.world.query_tile(0, 0)
    }

    pub fn tick_count(&self) -> u64 {
        self.world.tick_count()
    }

    /// Demo taxa from the kernel catalog: (taxon_id, display name).
    pub fn demo_taxa(&self) -> Vec<(String, String)> {
        self.world
            .catalog()
            .iter()
            .filter(|p| p.demo)
            .map(|p| (p.taxon_id.clone(), p.name_norm.clone()))
            .collect()
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
