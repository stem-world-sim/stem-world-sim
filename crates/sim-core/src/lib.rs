//! Stem World Sim — Sprint 10 flyover catch_up budget (closed basin).
//!
//! Water mass (A = 1): M = h_surf + sum_i (theta_i * L_i)
//! Grid mass: sum of column M. Closed basin: M + et_lost() + extract_lost() == mass ever added.
//! Tick order: infiltrate → pond → soil → lake_snap → at_rest/awake prune; advance clock;
//! if clock.tick % N_ET == 0: ET → occupants → lake_snap → rest/awake again.
//! Hydro + sinks visit observed-chunk cells ∪ awake set. CHUNK=8; ignore allowed while moving.
//! Rain / add_occupant / flux / catch_up wake cell+4-nbrs.
//! After a tick, at_rest cells whose 4-neighbors are also at_rest leave the awake set.
//! Pond: every lower-H 4-neighbor with ΔH > H_REST, weights ∝ ΔH, each edge ≤ R_MAX
//! and ≤ 0.5 ΔH; donor-scale then receiver-cap; drop V ≤ V_REST. Mass conserved.
//! Soil percolate/lateral: V = min(old cap, unused pore room on receiver); unused=0 ⇒ V=0;
//! blocked volume is NOT converted into pond inside the soil pass.
//! Lake snap: 4-connected components with h > V_REST and intra |ΔH| ≤ H_REST snap to
//! mean H* with Σh conserved; mark snapped cells at_rest. S04 sleep kept.
//! ET: pond first (E_OPEN) else top-layer soil (E_SOIL, may go below θ_fc); skip ≤ V_REST.
//! S = clamp(sum alive shade, 0, 1); E_eff = E * (1 - S). PlantStub shade default 0.25.
//! Occupants: per-cell list ≤ MAX_OCCUPANTS; PlantStub uptake by root mask; wilt at T_WILT.
//! catch_up(K, rain): optional source dump; then K blocks of (≤N_ET hydro, 1 unit sink).
//! catch_up_chunk: same calendar on one chunk + 1-cell halo; other chunks unchanged.
//! Unique-min-H chute revoked. Capillary stays.

/// Absolute / relative tolerance for water-mass comparisons (f64).
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

pub const MASS_EPSILON: f64 = 1e-9;

/// Max pond flux per edge per tick (metres water, A = 1). S03.3.
pub const R_MAX: f64 = 0.15;

/// Pond edge ignored if ΔH ≤ H_REST (metres). S04.
pub const H_REST: f64 = 0.02;

/// Soil / infiltrate / drain flux ignored if |V| ≤ V_REST (metres water). S04.
pub const V_REST: f64 = 1e-4;

/// Pond evaporation depth per ET-step (metres). S05.
pub const E_OPEN: f64 = 0.02;

/// Top-layer soil evaporation depth per ET-step (metres water). S05.
pub const E_SOIL: f64 = 0.005;

/// ET-step cadence: run evaporation when clock.tick % N_ET == 0 after advance. S05.
pub const N_ET: u64 = 10;

/// Max occupants per cell. S06.
pub const MAX_OCCUPANTS: usize = 8;

/// Root-mask length (matches default two-layer columns). S06.
pub const N_LAYERS: usize = 2;

/// PlantStub max uptake per sink-step (metres water, A = 1). S06.
pub const P_MAX: f64 = 0.01;

/// Dry sink-steps before PlantStub wilts (alive = false). S06.
pub const T_WILT: u32 = 3;

/// Max live hydro settle ticks inside [`World::catch_up`]. S08.
pub const T_SETTLE: u64 = 64;

/// Chunk edge length in cells (`chunk_id(x,y) = (x/CHUNK, y/CHUNK)`). S09.
pub const CHUNK: usize = 8;

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

/// Occupant kind. S06 PlantStub; later Animal, Machine, Drip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OccupantKind {
    PlantStub,
}

/// Cell occupant with root mask and uptake demand. S06.
#[derive(Clone, Debug, PartialEq)]
pub struct Occupant {
    pub kind: OccupantKind,
    pub alive: bool,
    pub uptake_max: f64,
    pub root: [f64; N_LAYERS],
    pub shade: f64,
    pub dry_steps: u32,
}

impl Occupant {
    /// Default PlantStub: root top-only, shade 0.25, uptake P_MAX, alive. S07.
    pub fn plant_stub() -> Self {
        Self {
            kind: OccupantKind::PlantStub,
            alive: true,
            uptake_max: P_MAX,
            root: [1.0, 0.0],
            shade: 0.25,
            dry_steps: 0,
        }
    }

    /// PlantStub with an explicit root mask (weights should sum to 1). Shade 0.25. S07.
    pub fn plant_stub_with_root(root: [f64; N_LAYERS]) -> Self {
        Self {
            kind: OccupantKind::PlantStub,
            alive: true,
            uptake_max: P_MAX,
            root,
            shade: 0.25,
            dry_steps: 0,
        }
    }
}

/// Error when [`World::add_occupant`] would exceed [`MAX_OCCUPANTS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OccupantsFull;

/// Historical S09 busy signal. S09.1: [`World::ignore_chunk`] always succeeds (never Err).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkBusy;

#[derive(Clone, Debug, PartialEq)]
struct ChunkState {
    observed: bool,
    t_away: u64,
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
    /// Soil stack; inline for N_LAYERS to keep 1024² worlds allocatable. S10.
    pub layers: SmallVec<[SoilLayer; N_LAYERS]>,
    /// Per-cell occupants, capped at [`MAX_OCCUPANTS`]. S06. Inline one plant.
    pub occupants: SmallVec<[Occupant; 1]>,
}

impl Column {
    pub fn new(
        elevation_m: f64,
        surface_water_m: f64,
        layers: impl Into<SmallVec<[SoilLayer; N_LAYERS]>>,
    ) -> Self {
        Self {
            elevation_m,
            surface_water_m,
            layers: layers.into(),
            occupants: SmallVec::new(),
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
    /// Skips if take ≤ V_REST. Returns depth moved (0 if skipped).
    fn infiltrate_step(&mut self) -> f64 {
        let mut budget = self.surface_water_m.min(self.i_max());
        if budget <= V_REST {
            return 0.0;
        }
        let mut remaining_surface = self.surface_water_m;
        let mut moved = 0.0;
        for layer in &mut self.layers {
            if budget <= 0.0 {
                break;
            }
            let capacity = layer.remaining_pore_m();
            if capacity <= 0.0 {
                continue;
            }
            let fill = budget.min(capacity);
            if fill <= V_REST {
                // Tiny residual fill — stop rather than drip below floor.
                break;
            }
            layer.theta += fill / layer.thickness_m;
            budget -= fill;
            remaining_surface -= fill;
            moved += fill;
        }
        self.surface_water_m = remaining_surface.max(0.0);
        if moved <= V_REST {
            // Should not happen if we filled, but treat sub-floor as idle.
            return 0.0;
        }
        moved
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
    #[allow(dead_code)]
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

    /// S04.1: fill soil top-down toward φ only; do not convert leftover into pond.
    /// Returns depth actually placed in soil.
    fn fill_soil_only(&mut self, mut amount: f64) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }
        let mut placed = 0.0;
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
            if layer.theta > layer.porosity {
                layer.theta = layer.porosity;
            }
            amount -= fill;
            placed += fill;
        }
        placed
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
                    if take <= V_REST {
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
                    if take <= V_REST {
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
/// S04: `at_rest` tracks cells with no supra-floor hydro flux last tick.
/// S08: `awake` is the explicit hydro visit set (rain/occupant/flux/catch_up wake).
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    seed: u64,
    clock: Clock,
    width: usize,
    height: usize,
    /// Row-major: index = y * width + x, y in [0, height), x in [0, width).
    columns: Vec<Column>,
    /// Per-cell rest flag (updated end of each tick).
    at_rest: Vec<bool>,
    /// Explicit awake set. S08.
    awake: Vec<bool>,
    /// Cached count of `awake[i]==true`. S10.
    n_awake: usize,
    /// Sparse awake indices (no duplicates). S10.
    awake_list: Vec<usize>,
    /// Chunk grid (CHUNK×CHUNK cells); visit = observed ∪ awake. S09.
    n_chunk_x: usize,
    n_chunk_y: usize,
    chunks: Vec<ChunkState>,
    /// Cached count of observed chunks. S10.
    n_observed: usize,
    /// Cells in the visit set of the last [`Self::hydro_step`]. S09.
    last_hydro_visits: usize,
    /// Cumulative water depth evaporated (A = 1); closed-basin sink. S05.
    et_lost: f64,
    /// Cumulative water depth extracted by occupants (A = 1). S06.
    extract_lost: f64,
}

impl World {
    /// S01 constructor: 1×1 grid with the given column.
    pub fn new(seed: u64, column: Column) -> Self {
        let (n_chunk_x, n_chunk_y, chunks) = Self::make_chunks(1, 1);
        let n_observed = chunks.len();
        Self {
            seed,
            clock: Clock::new(),
            width: 1,
            height: 1,
            columns: vec![column],
            at_rest: vec![false],
            awake: vec![true],
            n_awake: 1,
            awake_list: vec![0],
            n_chunk_x,
            n_chunk_y,
            chunks,
            n_observed,
            last_hydro_visits: 0,
            et_lost: 0.0,
            extract_lost: 0.0,
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
        let n = columns.len();
        let (n_chunk_x, n_chunk_y, chunks) = Self::make_chunks(width, height);
        let n_observed = chunks.len();
        Self {
            seed,
            clock: Clock::new(),
            width,
            height,
            columns,
            at_rest: vec![false; n],
            awake: vec![true; n],
            n_awake: n,
            awake_list: (0..n).collect(),
            n_chunk_x,
            n_chunk_y,
            chunks,
            n_observed,
            last_hydro_visits: 0,
            et_lost: 0.0,
            extract_lost: 0.0,
        }
    }

    /// Alias for [`World::grid`].
    pub fn from_columns(seed: u64, width: usize, height: usize, columns: Vec<Column>) -> Self {
        Self::grid(seed, width, height, columns)
    }

    pub fn set_column(&mut self, x: usize, y: usize, column: Column) {
        let i = self.idx(x, y);
        self.columns[i] = column;
        self.wake_idx(i);
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
        let i = self.idx(x, y);
        self.columns[i].surface_water_m += r;
        self.wake_idx(i);
    }

    /// Explicit surface-water add (wakes the cell). Alias semantics of rain for mass.
    pub fn add_water(&mut self, amount: f64) {
        self.add_water_at(0, 0, amount);
    }

    /// Explicit surface-water add at (x,y); wakes cell + 4-neighbors. S08.
    pub fn add_water_at(&mut self, x: usize, y: usize, amount: f64) {
        debug_assert!(amount >= 0.0, "water depth must be non-negative");
        let i = self.idx(x, y);
        self.columns[i].surface_water_m += amount;
        self.wake_idx(i);
    }

    /// S04: true if cell had no supra-floor hydro flux last tick.
    pub fn cell_at_rest(&self, x: usize, y: usize) -> bool {
        self.at_rest[self.idx(x, y)]
    }

    /// S04: true when every cell is at_rest.
    pub fn grid_at_rest(&self) -> bool {
        self.at_rest.iter().all(|&r| r)
    }

    /// S08: number of cells currently in the awake set.
    pub fn awake_count(&self) -> usize {
        self.n_awake
    }

    /// S09: number of chunks currently marked observed.
    pub fn observed_chunk_count(&self) -> usize {
        self.n_observed
    }

    /// S09: cells visited by the most recent hydro step (observed ∪ awake).
    pub fn last_hydro_visits(&self) -> usize {
        self.last_hydro_visits
    }

    /// S09: mark chunk (cx, cy) observed and reset t_away.
    pub fn observe_chunk(&mut self, cx: usize, cy: usize) {
        let i = self.chunk_index(cx, cy);
        if !self.chunks[i].observed {
            self.chunks[i].observed = true;
            self.n_observed += 1;
        }
        self.chunks[i].t_away = 0;
    }

    /// S09.1: mark chunk ignored even if cells are still moving.
    /// Come-back [`Self::catch_up_chunk`] owns any unfinished cascade.
    pub fn ignore_chunk(&mut self, cx: usize, cy: usize) -> Result<(), ChunkBusy> {
        let i = self.chunk_index(cx, cy);
        if self.chunks[i].observed {
            self.chunks[i].observed = false;
            self.n_observed = self.n_observed.saturating_sub(1);
        }
        self.chunks[i].t_away = 0;
        Ok(())
    }

    /// S10: mark every cell at_rest and clear the awake set (large-map init).
    /// Does not change column water; caller must ensure hydro-rest state.
    pub fn force_sleep_all(&mut self) {
        self.at_rest.fill(true);
        self.awake.fill(false);
        self.n_awake = 0;
        self.awake_list.clear();
    }

    /// S10: catch_up each listed chunk with the same (K, rain) calendar.
    pub fn catch_up_chunks(&mut self, chunks: &[(usize, usize)], k: u32, rain_per_step: f64) {
        for &(cx, cy) in chunks {
            self.catch_up_chunk(cx, cy, k, rain_per_step);
        }
    }

    /// S09.1: calendar-block catch-up on chunk (cx,cy) plus a 1-cell (4-neighbor) halo.
    /// Other chunks' cells are not sourced or sink-processed.
    pub fn catch_up_chunk(&mut self, cx: usize, cy: usize, k: u32, rain_per_step: f64) {
        debug_assert!(rain_per_step >= 0.0, "rain_per_step must be non-negative");
        let _ = self.chunk_index(cx, cy); // bounds check
        let region_ids = self.chunk_halo_ids(cx, cy);
        let region_set: HashSet<usize> = region_ids.iter().copied().collect();
        let k_u = k as u64;

        // Optional source dump (one dump if R>0); wake region cells.
        if k > 0 {
            let add = (k as f64) * rain_per_step;
            if add > 0.0 {
                for &i in &region_ids {
                    self.columns[i].surface_water_m += add;
                    self.wake_idx(i);
                }
            }
        }
        // R=0 come-back: still drain any pond/mobile left in the region. S10.
        for &i in &region_ids {
            let col = &self.columns[i];
            if col.surface_water_m > V_REST || col.mobile_water_m() > V_REST {
                self.wake_one(i);
            }
        }

        // K calendar blocks: ≤N_ET scoped hydro, then one unit sink. No settle-before-sink.
        for _ in 0..k {
            for _ in 0..N_ET {
                let mut step_ids: Vec<usize> = Vec::new();
                for &i in &region_ids {
                    if self.awake[i] {
                        step_ids.push(i);
                    }
                }
                if step_ids.is_empty() {
                    break;
                }
                self.hydro_step_active_ids(&step_ids, Some(&region_set));
            }
            let mut sink_busy: HashSet<usize> = HashSet::new();
            self.et_pass_ids(&mut sink_busy, &region_ids);
            self.occupant_pass_ids(&mut sink_busy, &region_ids);
            self.lake_snap_ids(&region_ids);
            for &i in &region_ids {
                let is_busy = sink_busy.contains(&i);
                self.at_rest[i] = !is_busy;
                if is_busy {
                    self.wake_one(i);
                }
            }
            self.prune_awake();
        }

        // Catch-up clears away-time for this chunk (still ignored unless observed).
        let ci = self.chunk_index(cx, cy);
        self.chunks[ci].t_away = 0;

        self.clock.tick = self.clock.tick.saturating_add(k_u.saturating_mul(N_ET));
    }

    fn make_chunks(width: usize, height: usize) -> (usize, usize, Vec<ChunkState>) {
        let n_chunk_x = (width + CHUNK - 1) / CHUNK;
        let n_chunk_y = (height + CHUNK - 1) / CHUNK;
        // Default observed=true so W*H<=12 (and larger) stay live; tests ignore explicitly.
        let chunks = vec![
            ChunkState {
                observed: true,
                t_away: 0,
            };
            n_chunk_x * n_chunk_y
        ];
        (n_chunk_x, n_chunk_y, chunks)
    }

    fn chunk_index(&self, cx: usize, cy: usize) -> usize {
        assert!(
            cx < self.n_chunk_x && cy < self.n_chunk_y,
            "chunk ({cx},{cy}) out of bounds {}x{}",
            self.n_chunk_x,
            self.n_chunk_y
        );
        cy * self.n_chunk_x + cx
    }

    fn chunk_id_of_cell(&self, x: usize, y: usize) -> (usize, usize) {
        (x / CHUNK, y / CHUNK)
    }

    fn chunk_observed_at_cell(&self, x: usize, y: usize) -> bool {
        let (cx, cy) = self.chunk_id_of_cell(x, y);
        self.chunks[self.chunk_index(cx, cy)].observed
    }

    /// Cells in chunk (cx,cy) plus any cell that is a 4-neighbor of a chunk cell.
    fn chunk_halo_mask(&self, cx: usize, cy: usize) -> Vec<bool> {
        let n = self.columns.len();
        let mut region = vec![false; n];
        for i in self.chunk_halo_ids(cx, cy) {
            region[i] = true;
        }
        region
    }

    /// Sparse chunk+halo cell indices. S10.
    fn chunk_halo_ids(&self, cx: usize, cy: usize) -> Vec<usize> {
        let x0 = cx * CHUNK;
        let y0 = cy * CHUNK;
        let x1 = ((cx + 1) * CHUNK).min(self.width);
        let y1 = ((cy + 1) * CHUNK).min(self.height);
        let mut ids = Vec::with_capacity(CHUNK * CHUNK + 4 * CHUNK);
        let mut seen = HashSet::new();
        for y in y0..y1 {
            for x in x0..x1 {
                let i = y * self.width + x;
                if seen.insert(i) {
                    ids.push(i);
                }
                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    if seen.insert(j) {
                        ids.push(j);
                    }
                }
            }
        }
        ids
    }

    /// Visit mask for live ticks: observed-chunk cells ∪ awake set.
    fn visit_mask(&self) -> Vec<bool> {
        let n = self.columns.len();
        let mut v = vec![false; n];
        if self.n_observed == 0 && self.n_awake == 0 {
            return v;
        }
        // Mark observed chunk cells without scanning the whole desert. S10.
        if self.n_observed > 0 {
            for cy in 0..self.n_chunk_y {
                for cx in 0..self.n_chunk_x {
                    let ci = cy * self.n_chunk_x + cx;
                    if !self.chunks[ci].observed {
                        continue;
                    }
                    let x0 = cx * CHUNK;
                    let y0 = cy * CHUNK;
                    let x1 = ((cx + 1) * CHUNK).min(self.width);
                    let y1 = ((cy + 1) * CHUNK).min(self.height);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            v[y * self.width + x] = true;
                        }
                    }
                }
            }
        }
        if self.n_awake > 0 {
            for &i in &self.awake_list {
                if self.awake[i] {
                    v[i] = true;
                }
            }
        }
        v
    }

    /// Sparse visit indices (observed ∪ awake) without building a full bool mask when empty.
    fn visit_indices(&self) -> Vec<usize> {
        if self.n_observed == 0 && self.n_awake == 0 {
            return Vec::new();
        }
        let mut seen: HashSet<usize> = HashSet::new();
        let mut ids = Vec::new();
        if self.n_observed > 0 {
            for cy in 0..self.n_chunk_y {
                for cx in 0..self.n_chunk_x {
                    let ci = cy * self.n_chunk_x + cx;
                    if !self.chunks[ci].observed {
                        continue;
                    }
                    let x0 = cx * CHUNK;
                    let y0 = cy * CHUNK;
                    let x1 = ((cx + 1) * CHUNK).min(self.width);
                    let y1 = ((cy + 1) * CHUNK).min(self.height);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let i = y * self.width + x;
                            if seen.insert(i) {
                                ids.push(i);
                            }
                        }
                    }
                }
            }
        }
        if self.n_awake > 0 {
            for &i in &self.awake_list {
                if self.awake[i] && seen.insert(i) {
                    ids.push(i);
                }
            }
        }
        ids
    }

    /// Grid water mass = sum of column M (I3).
    pub fn grid_water_mass(&self) -> f64 {
        self.columns.iter().map(Column::water_mass).sum()
    }

    /// Cumulative ET sink (metres water, A = 1). I3: grid_water_mass() + et_lost()
    /// + extract_lost() equals mass ever added (initial + rain/additions).
    pub fn et_lost(&self) -> f64 {
        self.et_lost
    }

    /// Cumulative occupant extract sink (metres water, A = 1). S06.
    pub fn extract_lost(&self) -> f64 {
        self.extract_lost
    }

    /// True if any alive [`OccupantKind::PlantStub`] is present at (x,y). S06.
    pub fn plant_at(&self, x: usize, y: usize) -> bool {
        self.column_at(x, y).occupants.iter().any(|o| {
            o.alive && matches!(o.kind, OccupantKind::PlantStub)
        })
    }

    /// Number of occupants at (x,y) (alive or wilted). S06.
    pub fn occupant_count(&self, x: usize, y: usize) -> usize {
        self.column_at(x, y).occupants.len()
    }

    /// Layer theta at (x,y), layer index i. S06 kernel query.
    pub fn layer_theta_at(&self, x: usize, y: usize, i: usize) -> f64 {
        self.column_at(x, y).layers[i].theta
    }

    /// Add an occupant at (x,y). Err if the cell already has [`MAX_OCCUPANTS`].
    /// Wakes the cell and its 4-neighbors. S08.
    pub fn add_occupant(&mut self, x: usize, y: usize, occ: Occupant) -> Result<(), OccupantsFull> {
        let i = self.idx(x, y);
        let col = &mut self.columns[i];
        if col.occupants.len() >= MAX_OCCUPANTS {
            return Err(OccupantsFull);
        }
        col.occupants.push(occ);
        self.wake_idx(i);
        Ok(())
    }

    /// Clear all occupants at (x,y).
    pub fn clear_occupants(&mut self, x: usize, y: usize) {
        self.column_at_mut(x, y).occupants.clear();
    }

    /// Clear occupants on every cell.
    pub fn clear_all_occupants(&mut self) {
        for col in &mut self.columns {
            col.occupants.clear();
        }
    }

    /// Infiltrate every column (rate-limited); no runoff/drain; advances clock.
    pub fn tick_infiltration(&mut self) {
        self.infiltrate_all();
        self.clock.advance();
    }

    /// Full tick: infiltrate → pond → soil → lake_snap → at_rest/awake prune; advance clock;
    /// if clock.tick % N_ET == 0: ET → occupants → lake_snap → rest/awake again (S05/S06/S08).
    /// S08: hydro visits only the awake set. S04 sleep integrated via awake prune.
    /// S04.1: soil flux capped by unused pore room; lake snap after soil.
    /// S05/S06: sink cadence after advance; ticks 10,20,… run one ET+occupant step.
    pub fn tick(&mut self) {
        // S10: fully ignored + asleep worlds must stay wall-clock cheap (no O(n) scans).
        if self.n_observed == 0 && self.n_awake == 0 {
            self.last_hydro_visits = 0;
            self.clock.advance();
            for ch in &mut self.chunks {
                ch.t_away = ch.t_away.saturating_add(1);
            }
            return;
        }

        self.hydro_step();
        self.clock.advance();

        // S09: ignored chunks accumulate away-time each live tick.
        for ch in &mut self.chunks {
            if !ch.observed {
                ch.t_away = ch.t_away.saturating_add(1);
            }
        }

        // S05/S06 sink batch: after advance, when tick is a multiple of N_ET.
        // S09: sinks only on observed ∪ awake (same visit set as hydro).
        if self.clock.tick % N_ET == 0 {
            let ids = self.visit_indices();
            if ids.is_empty() {
                return;
            }
            let mut sink_busy: HashSet<usize> = HashSet::new();
            self.et_pass_ids(&mut sink_busy, &ids);
            self.occupant_pass_ids(&mut sink_busy, &ids);
            self.lake_snap_ids(&ids);
            for &i in &ids {
                let is_busy = sink_busy.contains(&i);
                self.at_rest[i] = !is_busy;
                if is_busy {
                    self.wake_one(i);
                }
            }
            self.prune_awake();
        }
    }

    /// S09.1 catch-up facsimile: K calendar blocks of unobserved climate.
    /// Order: optional source dump → for each of K: ≤N_ET hydro then one unit sink.
    /// Do not hydro-to-rest before the first sink. Calendar advances by K * N_ET.
    /// When already at rest and rain_per_step == 0, inner hydro loops are empty and
    /// sinks match K live unit sink-steps (exact θ/h/et/extract).
    pub fn catch_up(&mut self, k: u32, rain_per_step: f64) {
        debug_assert!(rain_per_step >= 0.0, "rain_per_step must be non-negative");
        let k_u = k as u64;
        let n = self.columns.len();

        // 1. Optional source: every cell h += K * rain_per_step; wake cell + neighbors.
        if k > 0 {
            let add = (k as f64) * rain_per_step;
            if add > 0.0 {
                for i in 0..n {
                    self.columns[i].surface_water_m += add;
                    self.wake_idx(i);
                }
            }
        }

        // 2. K calendar blocks — interleave hydro and unit sinks (no settle-first).
        for _ in 0..k {
            for _ in 0..N_ET {
                if self.awake_count() == 0 {
                    break;
                }
                self.hydro_step();
            }
            let mut sink_busy = vec![false; n];
            self.et_pass(&mut sink_busy, None);
            self.occupant_pass(&mut sink_busy, None);
            self.lake_snap();
            for i in 0..n {
                self.at_rest[i] = !sink_busy[i];
                if sink_busy[i] {
                    self.wake_one(i);
                }
            }
            self.prune_awake();
        }

        // 3. Calendar: K sink-steps ≡ K * N_ET hydro ticks.
        self.clock.tick = self.clock.tick.saturating_add(k_u.saturating_mul(N_ET));
    }

    /// Hydro-only step (no clock, no ET): visit = observed ∪ awake. S09.
    fn hydro_step(&mut self) {
        if self.n_observed == 0 && self.n_awake == 0 {
            self.last_hydro_visits = 0;
            return;
        }
        let ids = self.visit_indices();
        self.hydro_step_active_ids(&ids, None);
    }

    /// Hydro-only step over an explicit active mask.
    /// If `receiver_bound` is set, pond/soil edges may only deliver inside that mask
    /// (used by [`Self::catch_up_chunk`] to keep flux inside chunk+halo).
    fn hydro_step_active(&mut self, active: &[bool], receiver_bound: Option<&HashSet<usize>>) {
        let active_ids: Vec<usize> = active
            .iter()
            .enumerate()
            .filter_map(|(i, &a)| if a { Some(i) } else { None })
            .collect();
        self.hydro_step_active_ids(&active_ids, receiver_bound);
    }

    fn hydro_step_active_ids(
        &mut self,
        active_ids: &[usize],
        receiver_bound: Option<&HashSet<usize>>,
    ) {
        self.last_hydro_visits = active_ids.len();
        if active_ids.is_empty() {
            return;
        }
        let mut busy: HashSet<usize> = HashSet::new();
        let mut flux_ids: Vec<usize> = Vec::new();
        let mut flux_seen: HashSet<usize> = HashSet::new();

        self.infiltrate_active_ids(active_ids, &mut busy);
        self.pond_pass_ids(active_ids, &mut busy, &mut flux_seen, &mut flux_ids, receiver_bound);
        let percolated = self.percolate_active_ids(active_ids, &mut busy);
        self.lateral_drain_pass_ids(
            &percolated,
            active_ids,
            &mut busy,
            &mut flux_seen,
            &mut flux_ids,
            receiver_bound,
        );
        self.lake_snap_ids(active_ids);

        for &i in &flux_ids {
            self.wake_idx(i);
        }
        for &i in active_ids {
            // Only update rest flags for cells we visited; ignored sleepers stay put.
            let is_busy = busy.contains(&i);
            self.at_rest[i] = !is_busy;
            if is_busy {
                self.wake_one(i);
            }
        }
        self.prune_awake();
    }

    /// Wake cell i and its 4-neighbors (clear at_rest, enter awake set). S08.
    fn wake_idx(&mut self, i: usize) {
        self.wake_one(i);
        let x = i % self.width;
        let y = i / self.width;
        for &(dx, dy) in &NEIGHBOR_OFFSETS {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                continue;
            }
            let j = ny as usize * self.width + nx as usize;
            self.wake_one(j);
        }
    }

    #[inline]
    fn wake_one(&mut self, i: usize) {
        if !self.awake[i] {
            self.awake[i] = true;
            self.n_awake += 1;
            self.awake_list.push(i);
        }
        self.at_rest[i] = false;
    }

    /// Drop awake cells that are at_rest with all 4-neighbors at_rest. S08.
    fn prune_awake(&mut self) {
        if self.n_awake == 0 {
            return;
        }
        let awake_ids = self.awake_list.clone();
        let mut leave: Vec<usize> = Vec::new();
        for &i in &awake_ids {
            if !self.awake[i] {
                continue;
            }
            if !self.at_rest[i] {
                continue;
            }
            let x = i % self.width;
            let y = i / self.width;
            let mut all_nbrs_rest = true;
            for &(dx, dy) in &NEIGHBOR_OFFSETS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                    continue;
                }
                let j = ny as usize * self.width + nx as usize;
                if !self.at_rest[j] {
                    all_nbrs_rest = false;
                    break;
                }
            }
            if all_nbrs_rest {
                leave.push(i);
            }
        }
        for i in leave {
            if self.awake[i] {
                self.awake[i] = false;
                self.n_awake = self.n_awake.saturating_sub(1);
            }
        }
        // Compact sparse list.
        self.awake_list.retain(|&i| self.awake[i]);
        debug_assert_eq!(self.awake_list.len(), self.n_awake);
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


    fn infiltrate_active_ids(&mut self, active_ids: &[usize], busy: &mut HashSet<usize>) {
        for &i in active_ids {
            let moved = self.columns[i].infiltrate_step();
            if moved > V_REST {
                busy.insert(i);
            }
        }
    }

    fn percolate_active_ids(
        &mut self,
        active_ids: &[usize],
        busy: &mut HashSet<usize>,
    ) -> HashMap<usize, f64> {
        let mut out = HashMap::new();
        for &i in active_ids {
            let moved = self.columns[i].percolate_step();
            if moved != 0.0 {
                out.insert(i, moved);
            }
            if moved > V_REST {
                busy.insert(i);
            }
        }
        out
    }

    fn et_pass_ids(&mut self, busy: &mut HashSet<usize>, ids: &[usize]) {
        for &i in ids {
            let (take, from_pond) = {
                let col = &self.columns[i];
                let sum_shade: f64 = col
                    .occupants
                    .iter()
                    .filter(|o| o.alive)
                    .map(|o| o.shade)
                    .sum();
                let shade_factor = 1.0 - sum_shade.clamp(0.0, 1.0);
                if col.surface_water_m > 0.0 {
                    let e_eff = E_OPEN * shade_factor;
                    (col.surface_water_m.min(e_eff), true)
                } else if let Some(top) = col.layers.first() {
                    let e_eff = E_SOIL * shade_factor;
                    ((top.theta * top.thickness_m).min(e_eff), false)
                } else {
                    (0.0, false)
                }
            };
            if take <= V_REST {
                continue;
            }
            busy.insert(i);
            let col = &mut self.columns[i];
            if from_pond {
                col.surface_water_m = (col.surface_water_m - take).max(0.0);
            } else if let Some(top) = col.layers.first_mut() {
                let mass = (top.theta * top.thickness_m - take).max(0.0);
                top.theta = if top.thickness_m > 0.0 {
                    mass / top.thickness_m
                } else {
                    0.0
                };
            }
            self.et_lost += take;
        }
    }

    fn occupant_pass_ids(&mut self, busy: &mut HashSet<usize>, ids: &[usize]) {
        for &i in ids {
            let occ_len = self.columns[i].occupants.len();
            if occ_len == 0 {
                continue;
            }
            let mut cell_taken = 0.0f64;
            for oi in 0..occ_len {
                if !self.columns[i].occupants[oi].alive {
                    continue;
                }
                let u = self.columns[i].occupants[oi].uptake_max;
                let root = self.columns[i].occupants[oi].root;
                let mut taken = 0.0f64;
                for k in 0..N_LAYERS {
                    if k >= self.columns[i].layers.len() {
                        break;
                    }
                    let demand = u * root[k];
                    if demand <= 0.0 {
                        continue;
                    }
                    let layer = &mut self.columns[i].layers[k];
                    let avail = (layer.theta * layer.thickness_m).max(0.0);
                    let take = demand.min(avail);
                    if take <= 0.0 {
                        continue;
                    }
                    let mass = (avail - take).max(0.0);
                    layer.theta = if layer.thickness_m > 0.0 {
                        mass / layer.thickness_m
                    } else {
                        0.0
                    };
                    taken += take;
                }
                if taken == 0.0 {
                    self.columns[i].occupants[oi].dry_steps =
                        self.columns[i].occupants[oi].dry_steps.saturating_add(1);
                } else {
                    self.columns[i].occupants[oi].dry_steps = 0;
                }
                if self.columns[i].occupants[oi].dry_steps >= T_WILT {
                    self.columns[i].occupants[oi].alive = false;
                }
                cell_taken += taken;
            }
            if cell_taken > V_REST {
                busy.insert(i);
            }
            self.extract_lost += cell_taken;
        }
    }

    /// Sparse lake snap over candidate seed ids (plus any wet cells among them).
    fn lake_snap_ids(&mut self, seed_ids: &[usize]) {
        if seed_ids.is_empty() {
            return;
        }
        // Candidates: seed cells with pond above the rest floor.
        let mut cand_ids: Vec<usize> = Vec::new();
        let mut is_cand = vec![false; self.columns.len()];
        for &i in seed_ids {
            if self.columns[i].surface_water_m > V_REST && !is_cand[i] {
                is_cand[i] = true;
                cand_ids.push(i);
            }
        }
        if cand_ids.is_empty() {
            return;
        }
        // Grow to full connected wet components that touch the seeds (global physics).
        // For scoped catch-up the seed set is the region; components stay inside if
        // exterior cells are dry. Scan neighbors of candidates iteratively.
        let mut stack = cand_ids.clone();
        while let Some(i) = stack.pop() {
            let x = i % self.width;
            let y = i / self.width;
            let h_i = self.columns[i].head();
            for &(dx, dy) in &NEIGHBOR_OFFSETS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                    continue;
                }
                let j = ny as usize * self.width + nx as usize;
                if is_cand[j] {
                    continue;
                }
                if self.columns[j].surface_water_m <= V_REST {
                    continue;
                }
                if (h_i - self.columns[j].head()).abs() <= H_REST {
                    is_cand[j] = true;
                    cand_ids.push(j);
                    stack.push(j);
                }
            }
        }

        let m = cand_ids.len();
        let mut parent: Vec<usize> = (0..m).collect();
        let mut rank: Vec<u8> = vec![0; m];
        let mut index_of = vec![usize::MAX; self.columns.len()];
        for (k, &i) in cand_ids.iter().enumerate() {
            index_of[i] = k;
        }
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        fn unite(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
            let mut ra = find(parent, a);
            let mut rb = find(parent, b);
            if ra == rb {
                return;
            }
            if rank[ra] < rank[rb] {
                std::mem::swap(&mut ra, &mut rb);
            }
            parent[rb] = ra;
            if rank[ra] == rank[rb] {
                rank[ra] = rank[ra].saturating_add(1);
            }
        }

        for &i in &cand_ids {
            let x = i % self.width;
            let y = i / self.width;
            let ki = index_of[i];
            let h_i = self.columns[i].head();
            for &(dx, dy) in &[(1i32, 0i32), (0i32, 1i32)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                    continue;
                }
                let j = ny as usize * self.width + nx as usize;
                let kj = index_of[j];
                if kj == usize::MAX {
                    continue;
                }
                if (h_i - self.columns[j].head()).abs() <= H_REST {
                    unite(&mut parent, &mut rank, ki, kj);
                }
            }
        }

        let mut comps: Vec<Vec<usize>> = vec![Vec::new(); m];
        for (k, &i) in cand_ids.iter().enumerate() {
            let r = find(&mut parent, k);
            comps[r].push(i);
        }

        for members in comps.into_iter() {
            let n_c = members.len();
            if n_c == 0 {
                continue;
            }
            if !members.iter().all(|&i| self.column_soil_full(i)) {
                continue;
            }
            let elevations_tmp: Vec<f64> =
                members.iter().map(|&i| self.columns[i].elevation_m).collect();
            // singleton check needs neighbor elevations — use full column elev
            let eligible = if n_c >= 2 {
                true
            } else {
                self.singleton_snap_eligible_live(members[0])
            };
            if !eligible {
                continue;
            }
            let _ = elevations_tmp;
            let sum_h: f64 = members.iter().map(|&i| self.columns[i].surface_water_m).sum();
            let sum_z: f64 = members.iter().map(|&i| self.columns[i].elevation_m).sum();
            let h_star = (sum_h + sum_z) / n_c as f64;

            let mut assigned: Vec<(usize, f64)> = Vec::with_capacity(n_c);
            let mut sum_new = 0.0;
            for &i in &members {
                let h_i = (h_star - self.columns[i].elevation_m).max(0.0);
                assigned.push((i, h_i));
                sum_new += h_i;
            }
            if sum_new > 0.0 && (sum_new - sum_h).abs() > 0.0 {
                let scale = sum_h / sum_new;
                for (_, h) in assigned.iter_mut() {
                    *h *= scale;
                }
                let s2: f64 = assigned.iter().map(|(_, h)| *h).sum();
                let residual = sum_h - s2;
                if residual.abs() > 0.0 {
                    if let Some((_, h)) = assigned.first_mut() {
                        *h = (*h + residual).max(0.0);
                    }
                }
            } else if sum_new == 0.0 && sum_h > 0.0 {
                continue;
            }
            for (i, h) in assigned {
                self.columns[i].surface_water_m = h;
            }
        }
    }

    fn singleton_snap_eligible_live(&self, i: usize) -> bool {
        let mobile = self.columns[i].mobile_water_m();
        if mobile <= V_REST {
            return true;
        }
        let z_i = self.columns[i].elevation_m;
        let x = i % self.width;
        let y = i / self.width;
        for &(dx, dy) in &NEIGHBOR_OFFSETS {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                continue;
            }
            let j = ny as usize * self.width + nx as usize;
            if self.columns[j].elevation_m < z_i {
                return false;
            }
        }
        true
    }

    fn infiltrate_all(&mut self) {
        for col in &mut self.columns {
            let _ = col.infiltrate_step();
        }
    }

    fn infiltrate_active(&mut self, active: &[bool], busy: &mut [bool]) {
        for (i, col) in self.columns.iter_mut().enumerate() {
            if !active[i] {
                continue;
            }
            let moved = col.infiltrate_step();
            if moved > V_REST {
                busy[i] = true;
            }
        }
    }

    /// Sparse pond pass over active donor ids. S10.
    fn pond_pass_ids(
        &mut self,
        active_ids: &[usize],
        busy: &mut HashSet<usize>,
        flux_seen: &mut HashSet<usize>,
        flux_ids: &mut Vec<usize>,
        receiver_bound: Option<&HashSet<usize>>,
    ) {
        if active_ids.is_empty() {
            return;
        }

        // Edges after donor scale: (from, to, volume).
        let mut edges: Vec<(usize, usize, f64)> = Vec::new();

        for &i in active_ids {
            let x = i % self.width;
            let y = i / self.width;
            let h_i = self.columns[i].surface_water_m;
            if h_i <= 0.0 {
                continue;
            }
            let h_head = self.columns[i].head();

            let mut lower: Vec<(usize, f64)> = Vec::new();
            for &(dx, dy) in &NEIGHBOR_OFFSETS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                    continue;
                }
                let j = ny as usize * self.width + nx as usize;
                if let Some(bound) = receiver_bound {
                    if !bound.contains(&j) {
                        continue;
                    }
                }
                let h_nbr = self.columns[j].head();
                if h_nbr < h_head {
                    let d_h = h_head - h_nbr;
                    if d_h > H_REST {
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

            let scale = if sum_v > h_i { h_i / sum_v } else { 1.0 };
            for &(j, v_j) in &sends {
                let v = v_j * scale;
                if v > 0.0 {
                    edges.push((i, j, v));
                }
            }
        }

        // Snapshot heads for receiver-cap (pre-apply).
        // Receiver cap: sparse HashMap instead of Vec<Vec> sized to n. S10.
        let mut incoming: HashMap<usize, Vec<usize>> = HashMap::new();
        for (eidx, &(_i, j, _v)) in edges.iter().enumerate() {
            incoming.entry(j).or_default().push(eidx);
        }
        for (&j, idxs) in incoming.iter() {
            if idxs.is_empty() {
                continue;
            }
            let in_sum: f64 = idxs.iter().map(|&e| edges[e].2).sum();
            if in_sum <= 0.0 {
                continue;
            }
            let min_donor_h = idxs
                .iter()
                .map(|&e| self.columns[edges[e].0].head())
                .fold(f64::INFINITY, f64::min);
            let room = (min_donor_h - self.columns[j].head()).max(0.0);
            if in_sum > room {
                let scale = room / in_sum;
                for &e in idxs {
                    edges[e].2 *= scale;
                }
            }
        }

        let mut delta: HashMap<usize, f64> = HashMap::new();
        for (i, j, v) in edges {
            if v <= V_REST {
                continue;
            }
            *delta.entry(i).or_insert(0.0) -= v;
            *delta.entry(j).or_insert(0.0) += v;
            busy.insert(i);
            busy.insert(j);
            if flux_seen.insert(j) {
                flux_ids.push(j);
            }
        }

        for (i, d) in delta {
            if d == 0.0 {
                continue;
            }
            let col = &mut self.columns[i];
            col.surface_water_m += d;
            if col.surface_water_m < 0.0 && col.surface_water_m.abs() <= MASS_EPSILON {
                col.surface_water_m = 0.0;
            }
        }
    }

    /// Sparse lateral drain over active donor ids. S10.
    fn lateral_drain_pass_ids(
        &mut self,
        percolated: &HashMap<usize, f64>,
        active_ids: &[usize],
        busy: &mut HashSet<usize>,
        flux_seen: &mut HashSet<usize>,
        flux_ids: &mut Vec<usize>,
        receiver_bound: Option<&HashSet<usize>>,
    ) {
        if active_ids.is_empty() {
            return;
        }

        let mut transfers: Vec<(usize, usize, f64)> = Vec::new();

        for &i in active_ids {
            let x = i % self.width;
            let y = i / self.width;
            let m_i = self.columns[i].mobile_water_m();
            if m_i <= V_REST {
                continue;
            }
            let d_i = self.columns[i].d_max();
            let d_rem = (d_i - percolated.get(&i).copied().unwrap_or(0.0)).max(0.0);
            if d_i <= 0.0 || d_rem <= 0.0 {
                continue;
            }
            let z_i = self.columns[i].elevation_m;

            let mut lower: Vec<(usize, f64)> = Vec::new();
            let mut equal_poorer: Vec<(usize, f64)> = Vec::new();

            for &(dx, dy) in &NEIGHBOR_OFFSETS {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                    continue;
                }
                let j = ny as usize * self.width + nx as usize;
                if let Some(bound) = receiver_bound {
                    if !bound.contains(&j) {
                        continue;
                    }
                }
                let z_nbr = self.columns[j].elevation_m;
                if z_nbr < z_i {
                    lower.push((j, z_nbr));
                } else if z_nbr == z_i {
                    let m_nbr = self.columns[j].mobile_water_m();
                    if m_nbr < m_i {
                        equal_poorer.push((j, m_nbr));
                    }
                }
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
                    let contact = d_rem.min(self.columns[j].d_max());
                    let v_j = equal_share.min(contact);
                    if v_j > V_REST {
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
                    if v > V_REST {
                        transfers.push((i, j, v));
                    }
                }
            } else if !equal_poorer.is_empty() {
                let mut planned: Vec<(usize, f64)> = Vec::new();
                let mut total = 0.0;
                for (j, m_nbr) in equal_poorer {
                    let contact = d_rem.min(self.columns[j].d_max());
                    let v_j = contact.min(0.5 * (m_i - m_nbr));
                    if v_j > V_REST {
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
                    if v > V_REST {
                        transfers.push((i, j, v));
                    }
                }
            }
        }

        // Cap by unused pore room (snapshot at receivers).
        let mut incoming: HashMap<usize, f64> = HashMap::new();
        for &(_from, to, amount) in &transfers {
            if amount > V_REST {
                *incoming.entry(to).or_insert(0.0) += amount;
            }
        }
        let mut recv_scale: HashMap<usize, f64> = HashMap::new();
        for (&j, &inc) in incoming.iter() {
            if inc <= 0.0 {
                continue;
            }
            let room = self.columns[j].remaining_pore_capacity_m().max(0.0);
            if room <= 0.0 {
                recv_scale.insert(j, 0.0);
            } else if inc > room {
                recv_scale.insert(j, room / inc);
            }
        }

        let mut remove_amt: HashMap<usize, f64> = HashMap::new();
        let mut soil_add: HashMap<usize, f64> = HashMap::new();

        for (from, to, amount) in transfers {
            let scale = recv_scale.get(&to).copied().unwrap_or(1.0);
            let v = amount * scale;
            if v <= V_REST {
                continue;
            }
            *remove_amt.entry(from).or_insert(0.0) += v;
            *soil_add.entry(to).or_insert(0.0) += v;
            busy.insert(from);
            busy.insert(to);
            if flux_seen.insert(to) {
                flux_ids.push(to);
            }
        }

        for (i, amt) in remove_amt {
            if amt > V_REST {
                self.columns[i].remove_mobile_water(amt);
            }
        }
        for (i, amt) in soil_add {
            if amt > V_REST {
                let _ = self.columns[i].fill_soil_only(amt);
            }
        }
    }

    /// Simultaneous pond pass (S03.3 + S03.3.1 receiver cap + S04 floors):
    /// Snapshot H and h. Drop edges with ΔH ≤ H_REST before weights.
    /// Candidate V_ij = min(h_i w_ij, 0.5 ΔH_ij, R_MAX); donor-scale if ΣV > h_i.
    /// Then receiver cap: for each j, room = max(0, min(donor H) − H_j); if In > room scale by room/In.
    /// After all scales, drop V ≤ V_REST. Only active cells donate; receivers may be sleeping.
    /// Apply after both caps. Equal H ⇒ no flux. Closed boundary.
    fn pond_pass(
        &mut self,
        active: &[bool],
        busy: &mut [bool],
        flux_recv: &mut [bool],
        receiver_bound: Option<&[bool]>,
    ) {
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
                if !active[i] {
                    continue;
                }
                let h_i = depths[i];
                if h_i <= 0.0 {
                    continue;
                }
                let h_head = heads[i];

                // Every 4-neighbor with ΔH > H_REST (strictly lower H by more than floor).
                let mut lower: Vec<(usize, f64)> = Vec::new();
                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    if let Some(bound) = receiver_bound {
                        if !bound[j] {
                            continue;
                        }
                    }
                    let h_nbr = heads[j];
                    if h_nbr < h_head {
                        let d_h = h_head - h_nbr;
                        if d_h > H_REST {
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

        // S04: after all scales, drop V ≤ V_REST (treat as 0).
        let mut delta = vec![0.0f64; n];
        for (i, j, v) in edges {
            if v <= V_REST {
                continue;
            }
            delta[i] -= v;
            delta[j] += v;
            busy[i] = true;
            busy[j] = true;
            flux_recv[j] = true;
        }

        for (col, d) in self.columns.iter_mut().zip(delta.iter()) {
            if *d == 0.0 {
                continue;
            }
            col.surface_water_m += *d;
            // Numerical guard: never go negative from fp noise.
            if col.surface_water_m < 0.0 && col.surface_water_m.abs() <= MASS_EPSILON {
                col.surface_water_m = 0.0;
            }
        }
    }

    /// S04: percolate only active columns; mark busy if moved > V_REST.
    fn percolate_active(&mut self, active: &[bool], busy: &mut [bool]) -> Vec<f64> {
        let n = self.columns.len();
        let mut out = vec![0.0f64; n];
        for i in 0..n {
            if !active[i] {
                continue;
            }
            let moved = self.columns[i].percolate_step();
            out[i] = moved;
            if moved > V_REST {
                busy[i] = true;
            }
        }
        out
    }

    /// Lateral / downhill drain on leftover mobile m after percolate (S03.2 + S04.1).
    /// Snapshot z, leftover m, D_max, unused pore room; then apply.
    /// Candidates: lower-z OR (equal-z && m_nbr < m_i); never z_nbr > z_i.
    /// Contact flux ≤ min(D_i, D_j); column outflow also ≤ remaining D_max after percolate.
    /// S04.1: V ≤ unused pore room on receiver; unused=0 ⇒ V=0; no soil→pond convert.
    /// If any lower-z: targets at min_z; equal split of leftover m, each ≤ contact.
    /// Else flat: half-diff of leftover m, ≤ contact; total ≤ leftover m and remaining D.
    /// Receiver: soil top-down only. Capillary does not move.
    fn lateral_drain_pass(
        &mut self,
        percolated: &[f64],
        active: &[bool],
        busy: &mut [bool],
        flux_recv: &mut [bool],
        receiver_bound: Option<&[bool]>,
    ) {
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
                if !active[i] {
                    continue;
                }
                let m_i = mobiles[i];
                if m_i <= V_REST {
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
                    if let Some(bound) = receiver_bound {
                        if !bound[j] {
                            continue;
                        }
                    }
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
                        if v_j > V_REST {
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
                        if v > V_REST {
                            transfers.push((i, j, v));
                        }
                    }
                } else if !equal_poorer.is_empty() {
                    let mut planned: Vec<(usize, f64)> = Vec::new();
                    let mut total = 0.0;
                    for (j, m_nbr) in equal_poorer {
                        let contact = d_rem.min(d_maxes[j]);
                        let v_j = contact.min(0.5 * (m_i - m_nbr));
                        if v_j > V_REST {
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
                        if v > V_REST {
                            transfers.push((i, j, v));
                        }
                    }
                }
            }
        }

        // S04.1: cap by unused pore room on receiving columns (snapshot).
        // If unused = 0, V = 0. Do NOT convert blocked volume into pond here.
        let pore_room: Vec<f64> = self
            .columns
            .iter()
            .map(Column::remaining_pore_capacity_m)
            .collect();
        let mut incoming: Vec<f64> = vec![0.0; n];
        for &(_from, to, amount) in &transfers {
            if amount > V_REST {
                incoming[to] += amount;
            }
        }
        let mut recv_scale: Vec<f64> = vec![1.0; n];
        for j in 0..n {
            if incoming[j] <= 0.0 {
                continue;
            }
            let room = pore_room[j].max(0.0);
            if room <= 0.0 {
                recv_scale[j] = 0.0;
            } else if incoming[j] > room {
                recv_scale[j] = room / incoming[j];
            }
        }

        let mut remove_amt = vec![0.0f64; n];
        let mut soil_add = vec![0.0f64; n];

        for (from, to, amount) in transfers {
            let v = amount * recv_scale[to];
            if v <= V_REST {
                continue;
            }
            remove_amt[from] += v;
            soil_add[to] += v;
            busy[from] = true;
            busy[to] = true;
            flux_recv[to] = true;
        }

        for i in 0..n {
            if remove_amt[i] > V_REST {
                self.columns[i].remove_mobile_water(remove_amt[i]);
            }
        }
        for i in 0..n {
            if soil_add[i] > V_REST {
                // Fill soil only; leftover must not become pond in the soil pass.
                let placed = self.columns[i].fill_soil_only(soil_add[i]);
                // If fp left a speck unplaced, it stays unapplied (donor already
                // matched the scaled V); mass error is sub-ε after clamp.
                let _ = placed;
            }
        }
    }

    /// S05/S07 batched evaporation: one ET-step (E*1). Pond first if h>0, else top soil.
    /// S = clamp(sum alive shade, 0, 1); E_eff = E * (1 - S). Dead/wilted shade = 0.
    /// Skip take ≤ V_REST. Wakes cells that lose > V_REST via `busy`.
    fn et_pass(&mut self, busy: &mut [bool], active: Option<&[bool]>) {
        let n = self.columns.len();
        for i in 0..n {
            if let Some(mask) = active {
                if !mask[i] {
                    continue;
                }
            }
            let (take, from_pond) = {
                let col = &self.columns[i];
                let sum_shade: f64 = col
                    .occupants
                    .iter()
                    .filter(|o| o.alive)
                    .map(|o| o.shade)
                    .sum();
                let shade_factor = 1.0 - sum_shade.clamp(0.0, 1.0);
                if col.surface_water_m > 0.0 {
                    let e_eff = E_OPEN * shade_factor;
                    (col.surface_water_m.min(e_eff), true)
                } else if let Some(top) = col.layers.first() {
                    let e_eff = E_SOIL * shade_factor;
                    ((top.theta * top.thickness_m).min(e_eff), false)
                } else {
                    (0.0, false)
                }
            };
            if take <= V_REST {
                continue;
            }
            busy[i] = true;
            let col = &mut self.columns[i];
            if from_pond {
                col.surface_water_m = (col.surface_water_m - take).max(0.0);
            } else if let Some(top) = col.layers.first_mut() {
                let mass = (top.theta * top.thickness_m - take).max(0.0);
                top.theta = if top.thickness_m > 0.0 {
                    mass / top.thickness_m
                } else {
                    0.0
                };
            }
            self.et_lost += take;
        }
    }

    /// S06 occupant extract after ET on the same sink cadence. Alive occupants in
    /// insert order: demand u = uptake_max; take min(u*root[k], θ_k*L) per layer
    /// (below θ_fc OK). No pond drink. Wilt after T_WILT consecutive dry steps.
    fn occupant_pass(&mut self, busy: &mut [bool], active: Option<&[bool]>) {
        let n = self.columns.len();
        for i in 0..n {
            if let Some(mask) = active {
                if !mask[i] {
                    continue;
                }
            }
            // Collect per-occupant totals then mutate layers / dry_steps.
            let occ_len = self.columns[i].occupants.len();
            if occ_len == 0 {
                continue;
            }
            let mut cell_taken = 0.0f64;
            for oi in 0..occ_len {
                if !self.columns[i].occupants[oi].alive {
                    continue;
                }
                let u = self.columns[i].occupants[oi].uptake_max;
                let root = self.columns[i].occupants[oi].root;
                let mut taken = 0.0f64;
                for k in 0..N_LAYERS {
                    if k >= self.columns[i].layers.len() {
                        break;
                    }
                    let demand = u * root[k];
                    if demand <= 0.0 {
                        continue;
                    }
                    let layer = &mut self.columns[i].layers[k];
                    let avail = (layer.theta * layer.thickness_m).max(0.0);
                    let take = demand.min(avail);
                    if take <= 0.0 {
                        continue;
                    }
                    let mass = (avail - take).max(0.0);
                    layer.theta = if layer.thickness_m > 0.0 {
                        mass / layer.thickness_m
                    } else {
                        0.0
                    };
                    taken += take;
                }
                if taken == 0.0 {
                    self.columns[i].occupants[oi].dry_steps =
                        self.columns[i].occupants[oi].dry_steps.saturating_add(1);
                } else {
                    self.columns[i].occupants[oi].dry_steps = 0;
                }
                if self.columns[i].occupants[oi].dry_steps >= T_WILT {
                    self.columns[i].occupants[oi].alive = false;
                }
                cell_taken += taken;
            }
            if cell_taken > V_REST {
                busy[i] = true;
            }
            self.extract_lost += cell_taken;
        }
    }

    /// S04.1 lake snap: after infiltrate + pond + soil, find 4-connected pond
    /// components where every member has h > V_REST and every intra-edge has
    /// |ΔH| ≤ H_REST. Snap each eligible component to mean H* with Σh conserved;
    /// mark those cells at_rest. Does not create or destroy mass.
    fn lake_snap(&mut self) {
        let n = self.columns.len();
        if n == 0 {
            return;
        }
        // S10: sparse path — only touch wet cells (avoids O(n) UF on deserts).
        let wet: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter_map(|(i, c)| if c.surface_water_m > V_REST { Some(i) } else { None })
            .collect();
        if wet.is_empty() {
            return;
        }
        // For modest wet sets use the sparse snap; fall through only if huge.
        if wet.len() * 8 < n || n <= 4096 {
            self.lake_snap_ids(&wet);
            return;
        }

        let heads: Vec<f64> = self.columns.iter().map(Column::head).collect();
        let depths: Vec<f64> = self
            .columns
            .iter()
            .map(|c| c.surface_water_m)
            .collect();
        let elevations: Vec<f64> = self.columns.iter().map(|c| c.elevation_m).collect();

        // Candidates: cells with pond above the rest floor.
        let mut cand = vec![false; n];
        for i in 0..n {
            if depths[i] > V_REST {
                cand[i] = true;
            }
        }

        // Union-Find over candidate 4-neighbors with |ΔH| ≤ H_REST.
        let mut parent: Vec<usize> = (0..n).collect();
        let mut rank: Vec<u8> = vec![0; n];
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        fn unite(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
            let mut ra = find(parent, a);
            let mut rb = find(parent, b);
            if ra == rb {
                return;
            }
            if rank[ra] < rank[rb] {
                std::mem::swap(&mut ra, &mut rb);
            }
            parent[rb] = ra;
            if rank[ra] == rank[rb] {
                rank[ra] = rank[ra].saturating_add(1);
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                if !cand[i] {
                    continue;
                }
                // Only unite along +E and +N to visit each undirected edge once.
                for &(dx, dy) in &[(1i32, 0i32), (0i32, 1i32)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0
                        || ny < 0
                        || nx as usize >= self.width
                        || ny as usize >= self.height
                    {
                        continue;
                    }
                    let j = ny as usize * self.width + nx as usize;
                    if !cand[j] {
                        continue;
                    }
                    if (heads[i] - heads[j]).abs() <= H_REST {
                        unite(&mut parent, &mut rank, i, j);
                    }
                }
            }
        }

        // Gather components.
        let mut comps: Vec<Vec<usize>> = vec![Vec::new(); n];
        for i in 0..n {
            if !cand[i] {
                continue;
            }
            let r = find(&mut parent, i);
            comps[r].push(i);
        }

        for members in comps.into_iter() {
            let n_c = members.len();
            if n_c == 0 {
                continue;
            }
            // Lake snap applies to free surface on full soil (sprint: when soil full).
            // Every member must have soil layers and no unused pore room.
            // Empty-soil pond-only grids keep S04 H_REST sleep without snap.
            if !members
                .iter()
                .all(|&i| self.column_soil_full(i))
            {
                continue;
            }
            let eligible = if n_c >= 2 {
                true
            } else {
                // N=1: also require no mobile that can leave to a lower-z cell.
                self.singleton_snap_eligible(members[0], &elevations)
            };
            if !eligible {
                continue;
            }

            let sum_h: f64 = members.iter().map(|&i| depths[i]).sum();
            let sum_z: f64 = members.iter().map(|&i| elevations[i]).sum();
            let h_star = (sum_h + sum_z) / n_c as f64;

            let mut assigned: Vec<(usize, f64)> = Vec::with_capacity(n_c);
            let mut sum_new = 0.0;
            for &i in &members {
                let h_i = (h_star - elevations[i]).max(0.0);
                assigned.push((i, h_i));
                sum_new += h_i;
            }

            // Tiny renormalize if fp noise broke Σh.
            if sum_new > 0.0 && (sum_new - sum_h).abs() > 0.0 {
                let scale = sum_h / sum_new;
                for (_, h) in assigned.iter_mut() {
                    *h *= scale;
                }
                let s2: f64 = assigned.iter().map(|(_, h)| *h).sum();
                let residual = sum_h - s2;
                if residual.abs() > 0.0 {
                    if let Some((_, h)) = assigned.first_mut() {
                        *h = (*h + residual).max(0.0);
                    }
                }
            } else if sum_new == 0.0 && sum_h > 0.0 {
                // Degenerate: all H* < z_i — should not happen for pond candidates;
                // leave depths unchanged rather than destroy mass.
                continue;
            }

            for (i, h) in assigned {
                self.columns[i].surface_water_m = h;
            }
        }
    }

    /// True when the column has soil layers and no unused pore room (at φ).
    fn column_soil_full(&self, i: usize) -> bool {
        let col = &self.columns[i];
        !col.layers.is_empty() && col.remaining_pore_capacity_m() <= MASS_EPSILON
    }

    /// N=1 lake-snap eligibility (soil-full already checked): no mobile that can
    /// leave to a lower-z 4-neighbor.
    fn singleton_snap_eligible(&self, i: usize, elevations: &[f64]) -> bool {
        let mobile = self.columns[i].mobile_water_m();
        if mobile <= V_REST {
            return true;
        }
        let z_i = elevations[i];
        let x = i % self.width;
        let y = i / self.width;
        for &(dx, dy) in &NEIGHBOR_OFFSETS {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= self.width || ny as usize >= self.height {
                continue;
            }
            let j = ny as usize * self.width + nx as usize;
            if elevations[j] < z_i {
                return false;
            }
        }
        true
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
