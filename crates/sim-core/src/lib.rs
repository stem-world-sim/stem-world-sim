//! Stem World Sim — Sprint 1 (D0) capacity-step column infiltration.
//!
//! Water mass (A = 1): M = h_surf + sum_i (theta_i * L_i)
//! Infiltration fills remaining pore space top-down; excess ponds on the surface.

/// Absolute / relative tolerance for water-mass comparisons (f64).
pub const MASS_EPSILON: f64 = 1e-9;

/// One soil layer: thickness L, volumetric moisture theta, porosity phi.
#[derive(Clone, Debug, PartialEq)]
pub struct SoilLayer {
    pub thickness_m: f64,
    pub theta: f64,
    pub porosity: f64,
}

impl SoilLayer {
    pub fn new(thickness_m: f64, theta: f64, porosity: f64) -> Self {
        Self {
            thickness_m,
            theta,
            porosity,
        }
    }

    /// Remaining pore water volume (depth equivalent) in this layer: (phi - theta) * L.
    pub fn remaining_pore_m(&self) -> f64 {
        (self.porosity - self.theta).max(0.0) * self.thickness_m
    }

    /// Water mass stored in this layer: theta * L (A = 1).
    pub fn water_mass(&self) -> f64 {
        self.theta * self.thickness_m
    }
}

/// Single soil column with surface pond and ordered layers (top → bottom).
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub elevation_m: f64,
    pub surface_water_m: f64,
    pub layers: Vec<SoilLayer>,
}

impl Column {
    pub fn new(elevation_m: f64, surface_water_m: f64, layers: Vec<SoilLayer>) -> Self {
        Self {
            elevation_m,
            surface_water_m,
            layers,
        }
    }

    /// Column water mass M = h_surf + sum(theta_i * L_i).
    pub fn water_mass(&self) -> f64 {
        self.surface_water_m
            + self.layers.iter().map(SoilLayer::water_mass).sum::<f64>()
    }

    /// Total remaining pore capacity across all layers (depth equivalent).
    pub fn remaining_pore_capacity_m(&self) -> f64 {
        self.layers.iter().map(SoilLayer::remaining_pore_m).sum()
    }

    /// True when surface is dry (within ε) or no pore space remains.
    pub fn is_quiescent(&self) -> bool {
        self.surface_water_m <= MASS_EPSILON || self.remaining_pore_capacity_m() <= MASS_EPSILON
    }
}

/// Simulation clock (tick count for Sprint 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    pub tick: u64,
}

impl Clock {
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    pub fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

/// World / Sim: seed + clock + one column.
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    seed: u64,
    clock: Clock,
    column: Column,
}

impl World {
    pub fn new(seed: u64, column: Column) -> Self {
        Self {
            seed,
            clock: Clock::new(),
            column,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn tick_count(&self) -> u64 {
        self.clock.tick
    }

    pub fn surface_water_m(&self) -> f64 {
        self.column.surface_water_m
    }

    pub fn elevation_m(&self) -> f64 {
        self.column.elevation_m
    }

    pub fn layer_count(&self) -> usize {
        self.column.layers.len()
    }

    pub fn layer_theta(&self, i: usize) -> f64 {
        self.column.layers[i].theta
    }

    pub fn layer_porosity(&self, i: usize) -> f64 {
        self.column.layers[i].porosity
    }

    pub fn layer_thickness_m(&self, i: usize) -> f64 {
        self.column.layers[i].thickness_m
    }

    /// Kernel query: column water mass (I6).
    pub fn water_mass(&self) -> f64 {
        self.column.water_mass()
    }

    pub fn remaining_pore_capacity_m(&self) -> f64 {
        self.column.remaining_pore_capacity_m()
    }

    /// Snapshot of layer thetas for equality / I5 checks.
    pub fn layer_thetas(&self) -> Vec<f64> {
        self.column.layers.iter().map(|l| l.theta).collect()
    }

    /// Add rain depth R to the surface (A = 1 ⇒ mass R).
    pub fn add_rain(&mut self, r: f64) {
        debug_assert!(r >= 0.0, "rain depth must be non-negative");
        self.column.surface_water_m += r;
    }

    /// One capacity-step infiltration: move surface water into remaining pores top-down.
    /// Excess stays as pond. Does not silently clamp in a way that destroys mass.
    pub fn tick_infiltration(&mut self) {
        let mut remaining = self.column.surface_water_m;
        for layer in &mut self.column.layers {
            if remaining <= 0.0 {
                break;
            }
            let capacity = layer.remaining_pore_m();
            if capacity <= 0.0 {
                continue;
            }
            let fill = remaining.min(capacity);
            // theta' = theta + fill / L
            layer.theta += fill / layer.thickness_m;
            remaining -= fill;
        }
        self.column.surface_water_m = remaining;
        self.clock.advance();
    }

    /// Alias for one capacity step.
    pub fn tick(&mut self) {
        self.tick_infiltration();
    }

    /// Tick until surface dry or soil saturated (quiescence).
    pub fn tick_until_surface_dry_or_saturated(&mut self) {
        // Finite layers ⇒ at most one useful pass after rain, but loop for API clarity.
        let max_steps = self.column.layers.len().saturating_add(2).max(1);
        for _ in 0..max_steps {
            if self.column.is_quiescent() {
                break;
            }
            self.tick_infiltration();
        }
    }
}

#[cfg(test)]
mod unit_smoke {
    use super::*;

    #[test]
    fn mass_formula_matches_definition() {
        let col = Column::new(
            0.0,
            0.1,
            vec![SoilLayer::new(0.5, 0.2, 0.4), SoilLayer::new(0.3, 0.1, 0.35)],
        );
        let expected = 0.1 + 0.2 * 0.5 + 0.1 * 0.3;
        assert!((col.water_mass() - expected).abs() < MASS_EPSILON);
    }
}
