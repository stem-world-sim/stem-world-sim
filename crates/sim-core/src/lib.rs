//! Stem World Sim — Sprint 2 (D1) slope runoff on a 4-neighbor grid.
//!
//! Water mass (A = 1): M = h_surf + sum_i (theta_i * L_i)
//! Grid mass: sum of column M.
//! Tick order: infiltrate every column (S01 capacity-step), then one runoff pass.
//! Only surface water moves laterally. Soil theta does not jump sideways.

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

    /// Hydraulic head H = elevation_m + surface_water_m.
    pub fn head(&self) -> f64 {
        self.elevation_m + self.surface_water_m
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

    /// One capacity-step infiltration on this column (no clock).
    fn infiltrate_step(&mut self) {
        let mut remaining = self.surface_water_m;
        for layer in &mut self.layers {
            if remaining <= 0.0 {
                break;
            }
            let capacity = layer.remaining_pore_m();
            if capacity <= 0.0 {
                continue;
            }
            let fill = remaining.min(capacity);
            layer.theta += fill / layer.thickness_m;
            remaining -= fill;
        }
        self.surface_water_m = remaining;
    }
}

/// Simulation clock (tick count).
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

/// Neighbor offsets: N, E, S, W (y increases north).
const NEIGHBOR_OFFSETS: [(i32, i32); 4] = [
    (0, 1),  // N
    (1, 0),  // E
    (0, -1), // S
    (-1, 0), // W
];

/// World / Sim: seed + clock + regular 4-neighbor grid of columns.
/// S01 1-column world = grid 1×1; single-cell queries operate on (0,0).
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    seed: u64,
    clock: Clock,
    width: usize,
    height: usize,
    /// Row-major: index = y * width + x, y in [0, height), x in [0, width).
    columns: Vec<Column>,
}

impl World {
    /// S01 constructor: 1×1 grid with the given column.
    pub fn new(seed: u64, column: Column) -> Self {
        Self {
            seed,
            clock: Clock::new(),
            width: 1,
            height: 1,
            columns: vec![column],
        }
    }

    /// Build a width×height grid from columns in row-major order (y outer, x inner).
    pub fn grid(seed: u64, width: usize, height: usize, columns: Vec<Column>) -> Self {
        assert!(width > 0 && height > 0, "grid dimensions must be positive");
        assert_eq!(
            columns.len(),
            width * height,
            "columns.len() must equal width * height"
        );
        Self {
            seed,
            clock: Clock::new(),
            width,
            height,
            columns,
        }
    }

    /// Alias for [`World::grid`].
    pub fn from_columns(seed: u64, width: usize, height: usize, columns: Vec<Column>) -> Self {
        Self::grid(seed, width, height, columns)
    }

    pub fn set_column(&mut self, x: usize, y: usize, column: Column) {
        let i = self.idx(x, y);
        self.columns[i] = column;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn tick_count(&self) -> u64 {
        self.clock.tick
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        assert!(x < self.width && y < self.height, "column ({x},{y}) out of bounds");
        y * self.width + x
    }

    pub fn column_at(&self, x: usize, y: usize) -> &Column {
        &self.columns[self.idx(x, y)]
    }

    pub fn column_at_mut(&mut self, x: usize, y: usize) -> &mut Column {
        let i = self.idx(x, y);
        &mut self.columns[i]
    }

    // --- S01 single-cell queries (operate on (0,0)) ---

    pub fn surface_water_m(&self) -> f64 {
        self.surface_water_m_at(0, 0)
    }

    pub fn elevation_m(&self) -> f64 {
        self.elevation_at(0, 0)
    }

    pub fn layer_count(&self) -> usize {
        self.column_at(0, 0).layers.len()
    }

    pub fn layer_theta(&self, i: usize) -> f64 {
        self.column_at(0, 0).layers[i].theta
    }

    pub fn layer_porosity(&self, i: usize) -> f64 {
        self.column_at(0, 0).layers[i].porosity
    }

    pub fn layer_thickness_m(&self, i: usize) -> f64 {
        self.column_at(0, 0).layers[i].thickness_m
    }

    /// Kernel query: cell (0,0) water mass (I6).
    pub fn water_mass(&self) -> f64 {
        self.water_mass_at(0, 0)
    }

    pub fn remaining_pore_capacity_m(&self) -> f64 {
        self.column_at(0, 0).remaining_pore_capacity_m()
    }

    /// Snapshot of layer thetas at (0,0) for equality / I5 checks.
    pub fn layer_thetas(&self) -> Vec<f64> {
        self.column_at(0, 0)
            .layers
            .iter()
            .map(|l| l.theta)
            .collect()
    }

    /// Add rain depth R to cell (0,0) surface (A = 1 ⇒ mass R).
    pub fn add_rain(&mut self, r: f64) {
        self.add_rain_at(0, 0, r);
    }

    // --- Grid cell queries ---

    pub fn surface_water_m_at(&self, x: usize, y: usize) -> f64 {
        self.column_at(x, y).surface_water_m
    }

    pub fn water_mass_at(&self, x: usize, y: usize) -> f64 {
        self.column_at(x, y).water_mass()
    }

    pub fn elevation_at(&self, x: usize, y: usize) -> f64 {
        self.column_at(x, y).elevation_m
    }

    pub fn add_rain_at(&mut self, x: usize, y: usize, r: f64) {
        debug_assert!(r >= 0.0, "rain depth must be non-negative");
        self.column_at_mut(x, y).surface_water_m += r;
    }

    /// Grid water mass = sum of column M (I3).
    pub fn grid_water_mass(&self) -> f64 {
        self.columns.iter().map(Column::water_mass).sum()
    }

    /// Infiltrate every column (S01 capacity-step); no runoff; advances clock.
    pub fn tick_infiltration(&mut self) {
        self.infiltrate_all();
        self.clock.advance();
    }

    /// Full tick: infiltrate all columns, then one runoff pass; advances clock once.
    pub fn tick(&mut self) {
        self.infiltrate_all();
        self.runoff_pass();
        self.clock.advance();
    }

    /// Tick until every column is quiescent (surface dry or saturated).
    /// Each step runs a full tick (infiltrate + runoff). On 1×1 runoff is a no-op.
    pub fn tick_until_surface_dry_or_saturated(&mut self) {
        let max_layers = self
            .columns
            .iter()
            .map(|c| c.layers.len())
            .max()
            .unwrap_or(0);
        // Extra steps allow runoff to redistribute then re-infiltrate.
        let max_steps = max_layers.saturating_add(2).max(1).saturating_mul(
            self.width.saturating_mul(self.height).max(1),
        );
        for _ in 0..max_steps {
            if self.columns.iter().all(Column::is_quiescent) {
                break;
            }
            self.tick();
        }
    }

    fn infiltrate_all(&mut self) {
        for col in &mut self.columns {
            col.infiltrate_step();
        }
    }

    /// Simultaneous runoff pass: snapshot heads/depths, accumulate deltas, apply.
    /// Each cell sends at most once. Closed boundary (no flux off-grid).
    fn runoff_pass(&mut self) {
        let n = self.columns.len();
        if n == 0 {
            return;
        }

        let heads: Vec<f64> = self.columns.iter().map(Column::head).collect();
        let depths: Vec<f64> = self
            .columns
            .iter()
            .map(|c| c.surface_water_m)
            .collect();
        let mut delta = vec![0.0f64; n];

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let h_i = depths[i];
                if h_i <= 0.0 {
                    continue;
                }
                let h_head = heads[i];

                // Candidates: neighbors with strictly lower H. Pick lowest H;
                // ties broken by N, E, S, W scan order (first among minimal H).
                let mut best: Option<(usize, f64)> = None; // (nbr_idx, H_nbr)
                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    let h_nbr = heads[j];
                    if h_nbr >= h_head {
                        continue;
                    }
                    match best {
                        None => best = Some((j, h_nbr)),
                        Some((_, best_h)) if h_nbr < best_h => best = Some((j, h_nbr)),
                        Some(_) => {} // equal or higher than current best H: keep earlier (N,E,S,W)
                    }
                }

                if let Some((j, h_nbr)) = best {
                    let vol = h_i.min(h_head - h_nbr);
                    if vol > 0.0 {
                        delta[i] -= vol;
                        delta[j] += vol;
                    }
                }
            }
        }

        for (col, d) in self.columns.iter_mut().zip(delta.iter()) {
            col.surface_water_m += *d;
            // Numerical guard: never go negative from fp noise.
            if col.surface_water_m < 0.0 && col.surface_water_m.abs() <= MASS_EPSILON {
                col.surface_water_m = 0.0;
            }
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

    #[test]
    fn head_is_elevation_plus_surface() {
        let col = Column::new(10.0, 0.25, vec![]);
        assert!((col.head() - 10.25).abs() < MASS_EPSILON);
    }
}
