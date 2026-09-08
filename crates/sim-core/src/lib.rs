//! Stem World Sim — Sprint 3.3.1 pond receiver cap (S03.3 sender caps + receiver room).
//!
//! Water mass (A = 1): M = h_surf + sum_i (theta_i * L_i)
//! Grid mass: sum of column M.
//! Tick order: rate-limited infiltrate → pond pass → percolate → lateral → clock.
//! Pond: every lower-H 4-neighbor, weights ∝ ΔH, each edge ≤ R_MAX and ≤ 0.5 ΔH;
//! donor-scale then receiver-cap so H_j after ≤ min donor snapshot H. Mass conserved.
//! Unique-min-H chute revoked. Soil percolate/lateral stay S03.2. Capillary stays.

/// Absolute / relative tolerance for water-mass comparisons (f64).
pub const MASS_EPSILON: f64 = 1e-9;

/// Max pond flux per edge per tick (metres water, A = 1). S03.3.
pub const R_MAX: f64 = 0.15;

/// Soil texture: porosity, field capacity, infiltrate and drain rate limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Texture {
    Sand,
    Loam,
    Clay,
}

impl Texture {
    /// Porosity φ (volume fraction).
    pub fn porosity(self) -> f64 {
        match self {
            Texture::Sand => 0.40,
            Texture::Loam => 0.45,
            Texture::Clay => 0.50,
        }
    }

    /// Field capacity θ_fc = 0.50 * φ.
    pub fn theta_fc(self) -> f64 {
        0.50 * self.porosity()
    }

    /// Max infiltrate depth per tick (metres water, A = 1).
    pub fn i_max(self) -> f64 {
        match self {
            Texture::Sand => 0.20,
            Texture::Loam => 0.05,
            Texture::Clay => 0.01,
        }
    }

    /// Max gravity-drain depth per tick (metres water, A = 1).
    pub fn d_max(self) -> f64 {
        match self {
            Texture::Sand => 0.04,
            Texture::Loam => 0.01,
            Texture::Clay => 0.002,
        }
    }
}

/// One soil layer: thickness L, volumetric moisture theta, texture (φ from table).
#[derive(Clone, Debug, PartialEq)]
pub struct SoilLayer {
    pub thickness_m: f64,
    pub theta: f64,
    pub porosity: f64,
    pub texture: Texture,
}

impl SoilLayer {
    /// Build a layer from texture; porosity is the texture's φ.
    pub fn new(thickness_m: f64, theta: f64, texture: Texture) -> Self {
        Self {
            thickness_m,
            theta,
            porosity: texture.porosity(),
            texture,
        }
    }

    /// Alias for [`SoilLayer::new`].
    pub fn with_texture(thickness_m: f64, theta: f64, texture: Texture) -> Self {
        Self::new(thickness_m, theta, texture)
    }

    pub fn theta_fc(&self) -> f64 {
        self.texture.theta_fc()
    }

    /// Remaining pore water volume (depth equivalent) in this layer: (phi - theta) * L.
    pub fn remaining_pore_m(&self) -> f64 {
        (self.porosity - self.theta).max(0.0) * self.thickness_m
    }

    /// Water mass stored in this layer: theta * L (A = 1).
    pub fn water_mass(&self) -> f64 {
        self.theta * self.thickness_m
    }

    /// Mobile water depth above field capacity: max(0, θ - θ_fc) * L.
    pub fn mobile_water_m(&self) -> f64 {
        (self.theta - self.theta_fc()).max(0.0) * self.thickness_m
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

    /// Column texture: top layer, or Sand if empty (World::new default).
    pub fn texture(&self) -> Texture {
        self.layers
            .first()
            .map(|l| l.texture)
            .unwrap_or(Texture::Sand)
    }

    pub fn i_max(&self) -> f64 {
        self.texture().i_max()
    }

    pub fn d_max(&self) -> f64 {
        self.texture().d_max()
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

    /// Soil-only water mass (no surface).
    pub fn soil_water_mass(&self) -> f64 {
        self.layers.iter().map(SoilLayer::water_mass).sum()
    }

    /// Total remaining pore capacity across all layers (depth equivalent).
    pub fn remaining_pore_capacity_m(&self) -> f64 {
        self.layers.iter().map(SoilLayer::remaining_pore_m).sum()
    }

    /// Total mobile soil water depth: sum max(0, θ - θ_fc) * L.
    pub fn mobile_water_m(&self) -> f64 {
        self.layers.iter().map(SoilLayer::mobile_water_m).sum()
    }

    /// True when surface is dry (within ε) or no pore space remains.
    pub fn is_quiescent(&self) -> bool {
        self.surface_water_m <= MASS_EPSILON || self.remaining_pore_capacity_m() <= MASS_EPSILON
    }

    /// Rate-limited capacity-step infiltration (no clock).
    /// budget = min(h_surf, I_max); fill top-down into remaining pores.
    fn infiltrate_step(&mut self) {
        let mut budget = self.surface_water_m.min(self.i_max());
        let mut remaining_surface = self.surface_water_m;
        for layer in &mut self.layers {
            if budget <= 0.0 {
                break;
            }
            let capacity = layer.remaining_pore_m();
            if capacity <= 0.0 {
                continue;
            }
            let fill = budget.min(capacity);
            layer.theta += fill / layer.thickness_m;
            budget -= fill;
            remaining_surface -= fill;
        }
        self.surface_water_m = remaining_surface.max(0.0);
    }

    /// Remove up to `amount` metres of mobile water (θ → θ_fc), top-down.
    fn remove_mobile_water(&mut self, mut amount: f64) {
        for layer in &mut self.layers {
            if amount <= 0.0 {
                break;
            }
            let mobile = layer.mobile_water_m();
            if mobile <= 0.0 {
                continue;
            }
            let take = amount.min(mobile);
            layer.theta -= take / layer.thickness_m;
            // Numerical guard: never go below θ_fc from fp noise.
            let fc = layer.theta_fc();
            if layer.theta < fc {
                layer.theta = fc;
            }
            amount -= take;
        }
    }

    /// Add water into soil top-down toward φ; any leftover goes to surface.
    /// Returns the amount that landed in soil (for debugging); surface absorbs the rest.
    fn add_soil_water(&mut self, mut amount: f64) {
        if amount <= 0.0 {
            return;
        }
        for layer in &mut self.layers {
            if amount <= 0.0 {
                break;
            }
            let capacity = layer.remaining_pore_m();
            if capacity <= 0.0 {
                continue;
            }
            let fill = amount.min(capacity);
            layer.theta += fill / layer.thickness_m;
            // Numerical guard: never exceed φ from fp noise.
            if layer.theta > layer.porosity {
                layer.theta = layer.porosity;
            }
            amount -= fill;
        }
        if amount > 0.0 {
            self.surface_water_m += amount;
        }
    }

    /// Intra-column percolation (S03.2): mobile water in layer k fills remaining pore
    /// in layers k+1…. Snapshot θ, then apply. Do not pull below θ_fc.
    /// Shared texture → one D_max budget for the column; else each source layer's D_max.
    /// Returns total depth moved (counts against the column's lateral D_max budget this tick).
    fn percolate_step(&mut self) -> f64 {
        let n = self.layers.len();
        if n < 2 {
            return 0.0;
        }

        let thetas: Vec<f64> = self.layers.iter().map(|l| l.theta).collect();
        let same_texture = self
            .layers
            .windows(2)
            .all(|w| w[0].texture == w[1].texture);

        // Remaining pore from snapshot; shrink as we plan fills.
        let mut pore_left: Vec<f64> = self
            .layers
            .iter()
            .zip(thetas.iter())
            .map(|(layer, &th)| (layer.porosity - th).max(0.0) * layer.thickness_m)
            .collect();
        let mut delta = vec![0.0f64; n];

        if same_texture {
            let mut budget = self.d_max();
            for k in 0..n {
                if budget <= 0.0 {
                    break;
                }
                let fc = self.layers[k].theta_fc();
                let l_k = self.layers[k].thickness_m;
                let mut available = (thetas[k] - fc).max(0.0) * l_k;
                if available <= 0.0 {
                    continue;
                }
                for dest in (k + 1)..n {
                    if budget <= 0.0 || available <= 0.0 {
                        break;
                    }
                    let can = pore_left[dest];
                    if can <= 0.0 {
                        continue;
                    }
                    let take = available.min(can).min(budget);
                    if take <= 0.0 {
                        continue;
                    }
                    delta[k] -= take;
                    delta[dest] += take;
                    available -= take;
                    budget -= take;
                    pore_left[dest] -= take;
                }
            }
        } else {
            for k in 0..n {
                let mut budget = self.layers[k].texture.d_max();
                if budget <= 0.0 {
                    continue;
                }
                let fc = self.layers[k].theta_fc();
                let l_k = self.layers[k].thickness_m;
                let mut available = (thetas[k] - fc).max(0.0) * l_k;
                if available <= 0.0 {
                    continue;
                }
                for dest in (k + 1)..n {
                    if budget <= 0.0 || available <= 0.0 {
                        break;
                    }
                    let can = pore_left[dest];
                    if can <= 0.0 {
                        continue;
                    }
                    let take = available.min(can).min(budget);
                    if take <= 0.0 {
                        continue;
                    }
                    delta[k] -= take;
                    delta[dest] += take;
                    available -= take;
                    budget -= take;
                    pore_left[dest] -= take;
                }
            }
        }

        let mut moved = 0.0f64;
        for (i, layer) in self.layers.iter_mut().enumerate() {
            if delta[i] == 0.0 {
                continue;
            }
            if delta[i] > 0.0 {
                moved += delta[i];
            }
            layer.theta += delta[i] / layer.thickness_m;
            let fc = layer.theta_fc();
            if layer.theta < fc {
                layer.theta = fc;
            }
            if layer.theta > layer.porosity {
                layer.theta = layer.porosity;
            }
        }
        moved
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

/// Neighbor offsets: N, E, S, W (y increases north). NESW tie-break order.
const NEIGHBOR_OFFSETS: [(i32, i32); 4] = [
    (0, 1),  // N
    (1, 0),  // E
    (0, -1), // S
    (-1, 0), // W
];

/// World / Sim: seed + clock + regular 4-neighbor grid of columns.
/// S01 1-column world = grid 1×1; single-cell queries operate on (0,0).
/// Default texture for unspecified layers: Sand.
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

    /// Infiltrate every column (rate-limited); no runoff/drain; advances clock.
    pub fn tick_infiltration(&mut self) {
        self.infiltrate_all();
        self.clock.advance();
    }

    /// Full tick: infiltrate → pond pass (S03.3.1) → percolate → lateral; advances clock once.
    pub fn tick(&mut self) {
        self.infiltrate_all();
        self.pond_pass();
        let percolated = self.percolate_all();
        self.lateral_drain_pass(&percolated);
        self.clock.advance();
    }

    /// Tick until every column is quiescent (surface dry or saturated).
    /// Bound accounts for Sand I_max rate limiting plus pond redistribute.
    pub fn tick_until_surface_dry_or_saturated(&mut self) {
        let max_layers = self
            .columns
            .iter()
            .map(|c| c.layers.len())
            .max()
            .unwrap_or(0);
        let max_h = self
            .columns
            .iter()
            .map(|c| c.surface_water_m)
            .fold(0.0_f64, f64::max);
        let min_i_max = self
            .columns
            .iter()
            .map(|c| c.i_max())
            .fold(f64::INFINITY, f64::min);
        let min_i_max = if min_i_max.is_finite() && min_i_max > 0.0 {
            min_i_max
        } else {
            Texture::Sand.i_max()
        };
        // ceil(h / I_max) infiltrate steps + layers + pond slack, × grid size.
        let infil_ticks = ((max_h / min_i_max).ceil() as usize).saturating_add(1);
        let cells = self.width.saturating_mul(self.height).max(1);
        let max_steps = infil_ticks
            .saturating_add(max_layers)
            .saturating_add(8)
            .saturating_mul(cells)
            .max(64);

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

    /// Simultaneous pond pass (S03.3 + S03.3.1 receiver cap):
    /// Snapshot H and h. Candidate V_ij = min(h_i w_ij, 0.5 ΔH_ij, R_MAX); donor-scale if ΣV > h_i.
    /// Then receiver cap: for each j, room = max(0, min(donor H) − H_j); if In > room scale by room/In.
    /// Apply after both caps. Equal H ⇒ no flux. Closed boundary.
    fn pond_pass(&mut self) {
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

        // Edges after donor scale: (from, to, volume).
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let h_i = depths[i];
                if h_i <= 0.0 {
                    continue;
                }
                let h_head = heads[i];

                // Every 4-neighbor with strictly lower H (no unique-min-H).
                let mut lower: Vec<(usize, f64)> = Vec::new();
                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    let h_nbr = heads[j];
                    if h_nbr < h_head {
                        let d_h = h_head - h_nbr;
                        if d_h > 0.0 {
                            lower.push((j, d_h));
                        }
                    }
                }
                if lower.is_empty() {
                    continue;
                }

                let sum_delta: f64 = lower.iter().map(|&(_, d)| d).sum();
                if sum_delta <= 0.0 {
                    continue;
                }

                // Desired send: V_j = min(h_i * w_j, 0.5 * Δ_j, R_MAX)
                let mut sends: Vec<(usize, f64)> = Vec::with_capacity(lower.len());
                let mut sum_v = 0.0;
                for &(j, d_h) in &lower {
                    let w = d_h / sum_delta;
                    let v_j = (h_i * w).min(0.5 * d_h).min(R_MAX);
                    if v_j > 0.0 {
                        sends.push((j, v_j));
                        sum_v += v_j;
                    }
                }
                if sum_v <= 0.0 {
                    continue;
                }

                // Donor scale: if Σ V_j > h_i, scale all V_j by h_i / Σ V_j.
                let scale = if sum_v > h_i { h_i / sum_v } else { 1.0 };
                for &(j, v_j) in &sends {
                    let v = v_j * scale;
                    if v > 0.0 {
                        edges.push((i, j, v));
                    }
                }
            }
        }

        // S03.3.1 receiver cap: H_j after ≤ every donor snapshot H that sent to j.
        // Group edge indices by receiver.
        let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (eidx, &(_i, j, _v)) in edges.iter().enumerate() {
            incoming[j].push(eidx);
        }
        for j in 0..n {
            let idxs = &incoming[j];
            if idxs.is_empty() {
                continue;
            }
            let in_sum: f64 = idxs.iter().map(|&e| edges[e].2).sum();
            if in_sum <= 0.0 {
                continue;
            }
            let min_donor_h = idxs
                .iter()
                .map(|&e| heads[edges[e].0])
                .fold(f64::INFINITY, f64::min);
            let room = (min_donor_h - heads[j]).max(0.0);
            if in_sum > room {
                let scale = room / in_sum;
                for &e in idxs {
                    edges[e].2 *= scale;
                }
            }
        }

        let mut delta = vec![0.0f64; n];
        for (i, j, v) in edges {
            if v > 0.0 {
                delta[i] -= v;
                delta[j] += v;
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

    /// S03.2: percolate every column (profile-first), apply immediately.
    /// Returns per-column volume percolated (for shared D_max with lateral).
    fn percolate_all(&mut self) -> Vec<f64> {
        self.columns
            .iter_mut()
            .map(Column::percolate_step)
            .collect()
    }

    /// Lateral / downhill drain on leftover mobile m after percolate (S03.2).
    /// Snapshot z, leftover m, D_max; then apply.
    /// Candidates: lower-z OR (equal-z && m_nbr < m_i); never z_nbr > z_i.
    /// Contact flux ≤ min(D_i, D_j); column outflow also ≤ remaining D_max after percolate.
    /// If any lower-z: targets at min_z; equal split of leftover m, each ≤ contact.
    /// Else flat: half-diff of leftover m, ≤ contact; total ≤ leftover m and remaining D.
    /// Receiver: soil top-down (overflow past φ → surface). Capillary does not move.
    fn lateral_drain_pass(&mut self, percolated: &[f64]) {
        let n = self.columns.len();
        if n == 0 {
            return;
        }

        let elevations: Vec<f64> = self.columns.iter().map(|c| c.elevation_m).collect();
        let mobiles: Vec<f64> = self.columns.iter().map(Column::mobile_water_m).collect();
        let d_maxes: Vec<f64> = self.columns.iter().map(Column::d_max).collect();
        // Remaining drain budget after profile percolation (shared column D_max).
        let rem_d: Vec<f64> = (0..n)
            .map(|i| (d_maxes[i] - percolated.get(i).copied().unwrap_or(0.0)).max(0.0))
            .collect();

        // Planned: (from, to, amount) — always delivered to soil first.
        let mut transfers: Vec<(usize, usize, f64)> = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                let m_i = mobiles[i];
                if m_i <= 0.0 {
                    continue;
                }
                let d_i = d_maxes[i];
                let d_rem = rem_d[i];
                if d_i <= 0.0 || d_rem <= 0.0 {
                    continue;
                }
                let z_i = elevations[i];

                let mut lower: Vec<(usize, f64)> = Vec::new();
                let mut equal_poorer: Vec<(usize, f64)> = Vec::new();

                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    let z_nbr = elevations[j];
                    if z_nbr < z_i {
                        lower.push((j, z_nbr));
                    } else if z_nbr == z_i && mobiles[j] < m_i {
                        equal_poorer.push((j, mobiles[j]));
                    }
                    // z_nbr > z_i: never send
                }

                if !lower.is_empty() {
                    let z_star = lower.iter().map(|&(_, z)| z).fold(f64::INFINITY, f64::min);
                    let targets: Vec<usize> = lower
                        .into_iter()
                        .filter(|&(_, z)| z == z_star)
                        .map(|(j, _)| j)
                        .collect();
                    let n_t = targets.len();
                    if n_t == 0 {
                        continue;
                    }
                    let mut planned: Vec<(usize, f64)> = Vec::new();
                    let mut total = 0.0;
                    let equal_share = m_i / n_t as f64;
                    for j in targets {
                        // Contact: min(donor remaining D, neighbor D_max).
                        let contact = d_rem.min(d_maxes[j]);
                        let v_j = equal_share.min(contact);
                        if v_j > 0.0 {
                            planned.push((j, v_j));
                            total += v_j;
                        }
                    }
                    if total <= 0.0 {
                        continue;
                    }
                    let cap = m_i.min(d_rem);
                    let scale = if total > cap { cap / total } else { 1.0 };
                    for (j, v_j) in planned {
                        let v = v_j * scale;
                        if v > 0.0 {
                            transfers.push((i, j, v));
                        }
                    }
                } else if !equal_poorer.is_empty() {
                    let mut planned: Vec<(usize, f64)> = Vec::new();
                    let mut total = 0.0;
                    for (j, m_nbr) in equal_poorer {
                        let contact = d_rem.min(d_maxes[j]);
                        let v_j = contact.min(0.5 * (m_i - m_nbr));
                        if v_j > 0.0 {
                            planned.push((j, v_j));
                            total += v_j;
                        }
                    }
                    if total <= 0.0 {
                        continue;
                    }
                    let cap = m_i.min(d_rem);
                    let scale = if total > cap { cap / total } else { 1.0 };
                    for (j, v_j) in planned {
                        let v = v_j * scale;
                        if v > 0.0 {
                            transfers.push((i, j, v));
                        }
                    }
                }
            }
        }

        let mut remove_amt = vec![0.0f64; n];
        let mut soil_add = vec![0.0f64; n];

        for (from, to, amount) in transfers {
            remove_amt[from] += amount;
            soil_add[to] += amount;
        }

        for i in 0..n {
            if remove_amt[i] > 0.0 {
                self.columns[i].remove_mobile_water(remove_amt[i]);
            }
        }
        for i in 0..n {
            if soil_add[i] > 0.0 {
                // add_soil_water fills top-down; overflow past φ → surface.
                self.columns[i].add_soil_water(soil_add[i]);
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
            vec![
                SoilLayer::new(0.5, 0.2, Texture::Sand),
                SoilLayer::new(0.3, 0.1, Texture::Loam),
            ],
        );
        let expected = 0.1 + 0.2 * 0.5 + 0.1 * 0.3;
        assert!((col.water_mass() - expected).abs() < MASS_EPSILON);
    }

    #[test]
    fn head_is_elevation_plus_surface() {
        let col = Column::new(10.0, 0.25, vec![]);
        assert!((col.head() - 10.25).abs() < MASS_EPSILON);
    }

    #[test]
    fn texture_table_sand_defaults() {
        assert!((Texture::Sand.porosity() - 0.40).abs() < MASS_EPSILON);
        assert!((Texture::Sand.theta_fc() - 0.20).abs() < MASS_EPSILON);
        assert!((Texture::Sand.i_max() - 0.20).abs() < MASS_EPSILON);
        assert!((Texture::Sand.d_max() - 0.04).abs() < MASS_EPSILON);
    }
}
