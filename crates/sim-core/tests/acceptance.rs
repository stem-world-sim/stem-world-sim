//! Acceptance tests — named contracts from docs/spec/ACCEPTANCE.md.
//!
//! ε: 1e-9 relative on f64 water mass (MASS_EPSILON from sim_core).
//! Cell area A = 1. Column water mass M = h_surf + sum(theta_i * L_i).
//! Grid mass: sum of column M.

use sim_core::{
    alpha_for_occupant, alpha_from_params, Catalog, ChunkBusy, ClimateReject, Column,
    HydroReject, LightReject, Occupant, OccupantParams, PlantTaxonError, SoilLayer, Texture,
    World, CHUNK, E_OPEN, E_SOIL, H_BAND, H_POND, H_REST, L0, MASS_EPSILON, MAX_OCCUPANTS,
    N_ET, N_LAYERS, P_MAX, R_MAX, T_DARK, T_SETTLE, T_SUB, T_WILT, T_WL, V_REST,
};

fn assert_mass_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    let tol = MASS_EPSILON * scale;
    assert!(
        (actual - expected).abs() <= tol,
        "mass not conserved: actual={actual} expected={expected} tol={tol} (ε={MASS_EPSILON} relative)"
    );
}

fn assert_approx_eq(a: f64, b: f64, label: &str) {
    let scale = b.abs().max(1.0);
    let tol = MASS_EPSILON * scale;
    assert!(
        (a - b).abs() <= tol,
        "{label}: {a} != {b} (tol={tol})"
    );
}


/// Closed-basin inventory: on-grid water + ET + extract (I3 / S05 / S06).
fn closed_mass(world: &World) -> f64 {
    world.grid_water_mass() + world.et_lost() + world.extract_lost()
}

/// Default Sand column (φ = 0.40). `porosity` arg kept for call-site shape; ignored — Sand φ used.
fn default_column(theta: f64, _porosity: f64) -> Column {
    Column::new(
        10.0,
        0.0,
        vec![
            SoilLayer::new(0.3, theta, Texture::Sand),
            SoilLayer::new(0.5, theta, Texture::Sand),
        ],
    )
}

/// Saturated Sand soil (θ = φ) so pond stays on the surface for runoff tests.
fn saturated_column(elevation_m: f64, surface_water_m: f64) -> Column {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    Column::new(
        elevation_m,
        surface_water_m,
        vec![
            SoilLayer::new(0.3, phi, tex),
            SoilLayer::new(0.5, phi, tex),
        ],
    )
}

/// Column at field capacity (θ = θ_fc) — no mobile soil water; pore space remains.
fn field_capacity_column(elevation_m: f64, surface_water_m: f64) -> Column {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    Column::new(
        elevation_m,
        surface_water_m,
        vec![
            SoilLayer::new(0.3, fc, tex),
            SoilLayer::new(0.5, fc, tex),
        ],
    )
}

fn textured_column(elevation_m: f64, surface_water_m: f64, theta: f64, texture: Texture) -> Column {
    Column::new(
        elevation_m,
        surface_water_m,
        vec![
            SoilLayer::new(0.3, theta, texture),
            SoilLayer::new(0.5, theta, texture),
        ],
    )
}

// ---------------------------------------------------------------------------
// Sprint 1 — D0 / I1 I2 I5 I6 (unchanged names and intent)
// ---------------------------------------------------------------------------

/// I1: after any legal tick, h_surf >= 0 and 0 <= theta_i <= phi_i; no silent clamp that destroys mass.
#[test]
fn i1_moisture_and_surface_nonnegative() {
    let mut world = World::new(42, default_column(0.1, 0.4));
    world.add_rain(0.25);
    world.tick_until_surface_dry_or_saturated();

    let h = world.surface_water_m();
    assert!(h >= 0.0, "h_surf must be >= 0, got {h}");

    for i in 0..world.layer_count() {
        let theta = world.layer_theta(i);
        let phi = world.layer_porosity(i);
        assert!(
            theta >= 0.0 && theta <= phi + MASS_EPSILON,
            "layer {i}: theta={theta} not in [0, phi={phi}]"
        );
    }
}

/// I2: isolated column — rain R + initial M = final M ±ε (no lateral flux/sink).
#[test]
fn i2_isolated_column_mass_conserved() {
    let mut world = World::new(7, default_column(0.15, 0.4));
    let m0 = world.water_mass();
    let r = 0.2;
    world.add_rain(r);
    world.tick_until_surface_dry_or_saturated();
    let m_final = world.water_mass() + world.et_lost() + world.extract_lost();
    assert_mass_close(m_final, m0 + r);
}

/// I5: same seed + same command script => same moisture and surface fields.
#[test]
fn i5_same_seed_same_script_same_state() {
    let seed = 99u64;
    let script = |w: &mut World| {
        w.add_rain(0.12);
        w.tick();
        w.add_rain(0.05);
        w.tick_until_surface_dry_or_saturated();
    };

    // Sand default (phi 0.4); prior 0.35 porosity migrated to Texture::Sand.
    let mut a = World::new(seed, default_column(0.1, 0.4));
    let mut b = World::new(seed, default_column(0.1, 0.4));
    script(&mut a);
    script(&mut b);

    assert_eq!(a.seed(), b.seed());
    assert_approx_eq(a.surface_water_m(), b.surface_water_m(), "surface");
    assert_eq!(a.layer_thetas(), b.layer_thetas());
    assert_approx_eq(a.water_mass(), b.water_mass(), "mass");
}

/// I6: win/fail numbers come from kernel query methods only.
#[test]
fn i6_queries_are_kernel_methods() {
    // Sand default; prior 0.45 porosity migrated to Texture::Sand.
    let mut world = World::new(1, default_column(0.2, 0.4));
    world.add_rain(0.08);
    world.tick_until_surface_dry_or_saturated();

    // Pass/fail values obtained solely via pub kernel queries (not private fields).
    let mass = world.water_mass();
    let surf = world.surface_water_m();
    let thetas = world.layer_thetas();
    let pore = world.remaining_pore_capacity_m();
    // S04 rest queries are also kernel methods.
    let _cell_rest = world.cell_at_rest(0, 0);
    let _grid_rest = world.grid_at_rest();
    // S05 ET sink query.
    let _et = world.et_lost();
    // S06 occupant / extract queries.
    let _extract = world.extract_lost();
    let _plant = world.plant_at(0, 0);
    let _occ_n = world.occupant_count(0, 0);
    let _th = world.layer_theta_at(0, 0, 0);
    // S08 / S09 region queries.
    let _awake = world.awake_count();
    let _obs = world.observed_chunk_count();
    let _visits = world.last_hydro_visits();

    assert!(mass >= 0.0);
    assert!(surf >= 0.0);
    assert_eq!(thetas.len(), world.layer_count());
    for (i, &theta) in thetas.iter().enumerate() {
        assert_approx_eq(theta, world.layer_theta(i), "theta query consistency");
        assert!(theta <= world.layer_porosity(i) + MASS_EPSILON);
    }
    assert!(pore >= 0.0);
}

/// D0-A: R < remaining pore; after ticks surface dry (≈0), water in soil.
#[test]
fn d0_rain_fits_in_pores_surface_dries() {
    let mut world = World::new(3, default_column(0.1, 0.4));
    let pore0 = world.remaining_pore_capacity_m();
    let r = pore0 * 0.5; // strictly less than remaining pore
    assert!(r < pore0);

    let m0 = world.water_mass();
    world.add_rain(r);
    world.tick_until_surface_dry_or_saturated();

    assert_approx_eq(world.surface_water_m(), 0.0, "surface should be dry");
    assert!(world.water_mass() + world.et_lost() + world.extract_lost() > m0);
    assert_mass_close(world.water_mass() + world.et_lost() + world.extract_lost(), m0 + r);
    // Water entered soil: some theta increased / pore decreased
    assert!(world.remaining_pore_capacity_m() < pore0 - MASS_EPSILON);
}

/// D0-B: R > remaining pore; soil at phi; pond remainder ≈ R - pore_capacity.
#[test]
fn d0_rain_exceeds_pores_remainder_ponds() {
    let mut world = World::new(5, default_column(0.1, 0.4));
    let pore0 = world.remaining_pore_capacity_m();
    let r = pore0 + 0.15;
    assert!(r > pore0);

    world.add_rain(r);
    world.tick_until_surface_dry_or_saturated();

    for i in 0..world.layer_count() {
        assert_approx_eq(
            world.layer_theta(i),
            world.layer_porosity(i),
            &format!("layer {i} at porosity"),
        );
    }
    let expected_pond = r - pore0;
    assert_approx_eq(world.surface_water_m(), expected_pond, "pond remainder");
    assert!(world.surface_water_m() > MASS_EPSILON, "must pond");
}

/// D0-C: novice claim "all rain vanishes" is false — dirt does not eat all rain.
#[test]
fn d0_novice_all_rain_vanishes_is_false() {
    let mut world = World::new(5, default_column(0.1, 0.4));
    let pore0 = world.remaining_pore_capacity_m();
    let r = pore0 + 0.15;
    let m0 = world.water_mass();

    world.add_rain(r);
    world.tick_until_surface_dry_or_saturated();

    let m_final = world.water_mass() + world.et_lost() + world.extract_lost();
    let surface_dry = world.surface_water_m() <= MASS_EPSILON;
    let mass_lost = (m_final - (m0 + r)).abs() > MASS_EPSILON * (m0 + r).abs().max(1.0);

    // Novice claim: all rain vanished (surface 0 AND mass lost). Must be false.
    let novice_all_rain_vanished = surface_dry && mass_lost;
    assert!(
        !novice_all_rain_vanished,
        "novice claim false: dirt must not eat all rain; pond or conserved mass required"
    );
    // Stronger: either pond remains or mass is conserved (actually both for this setup).
    assert!(
        world.surface_water_m() > MASS_EPSILON,
        "excess must pond — proves rain did not all vanish into dirt"
    );
    assert_mass_close(m_final, m0 + r);
}

// ---------------------------------------------------------------------------
// Sprint 2 — D1 slope_runoff / I3 I4
// ε: 1e-9 relative on f64 water mass (MASS_EPSILON).
// ---------------------------------------------------------------------------

/// I3: closed grid, no sink — rain on one cell; infiltrate+runoff+drain; sum M constant ±ε.
#[test]
fn i3_closed_grid_mass_conserved() {
    // 1×2 closed grid; rain on x=0.
    let cols = vec![
        saturated_column(10.0, 0.0),
        saturated_column(5.0, 0.0),
    ];
    let mut world = World::grid(11, 2, 1, cols);
    let r = 0.3;
    world.add_rain_at(0, 0, r);
    let m0 = world.grid_water_mass();

    for _ in 0..8 {
        world.tick(); // infiltrate + runoff + gravity drain
        assert_mass_close(closed_mass(&world), m0);
    }
    assert_mass_close(closed_mass(&world), m0);
}

/// I4: high+low connected pair plus isolated dry column; isolated M unchanged.
/// Isolated cell uses θ = θ_fc so gravity drain does not empty it downhill.
#[test]
fn i4_disconnected_dry_stays_dry() {
    // 1×3: elevations [10, 5, 100]; rain on x=0 → runoff 0→1; x=2 stays dry.
    let cols = vec![
        saturated_column(10.0, 0.0),
        saturated_column(5.0, 0.0),
        field_capacity_column(100.0, 0.0), // no mobile water → no gravity drain
    ];
    let mut world = World::grid(13, 3, 1, cols);
    let m_isolated0 = world.water_mass_at(2, 0);
    assert_approx_eq(m_isolated0, world.water_mass_at(2, 0), "baseline");
    assert!(world.surface_water_m_at(2, 0) <= MASS_EPSILON);

    world.add_rain_at(0, 0, 0.4);
    for _ in 0..8 {
        world.tick();
    }

    // Isolated cell: no inflow, started dry → M unchanged (I4).
    assert_mass_close(world.water_mass_at(2, 0), m_isolated0);
    assert!(
        world.surface_water_m_at(2, 0) <= MASS_EPSILON,
        "isolated cell must stay dry"
    );
    // Pair did move water: low cell gained mass.
    assert!(
        world.water_mass_at(1, 0) > m_isolated0 + MASS_EPSILON
            || world.surface_water_m_at(1, 0) > MASS_EPSILON
            || world.water_mass_at(0, 0) < world.grid_water_mass(),
        "runoff should have acted on the high→low pair"
    );
}

/// D1: z_high > z_low; pond on high; after ticks high surface down and low M up.
#[test]
fn d1_pond_leaves_high_appears_low() {
    let pond = 0.5;
    let cols = vec![
        saturated_column(10.0, pond), // high
        saturated_column(5.0, 0.0),   // low
    ];
    let mut world = World::grid(17, 2, 1, cols);
    let m_low0 = world.water_mass_at(1, 0);
    let h_high0 = world.surface_water_m_at(0, 0);
    assert!(h_high0 > MASS_EPSILON);

    for _ in 0..16 {
        world.tick();
    }

    let h_high = world.surface_water_m_at(0, 0);
    let m_low = world.water_mass_at(1, 0);
    assert!(
        h_high < h_high0 - MASS_EPSILON,
        "pond should leave high: was {h_high0}, now {h_high}"
    );
    assert!(
        m_low > m_low0 + MASS_EPSILON,
        "water should appear on low: was {m_low0}, now {m_low}"
    );
}

/// D1: pond only on the low cell; high stays dry (no uphill creation).
/// High at θ_fc so gravity drain does not move its mass downhill.
#[test]
fn d1_no_uphill_creation() {
    let cols = vec![
        field_capacity_column(10.0, 0.0), // high dry, no mobile
        saturated_column(5.0, 0.4),       // low with pond
    ];
    let mut world = World::grid(19, 2, 1, cols);
    let m_high0 = world.water_mass_at(0, 0);

    for _ in 0..8 {
        world.tick();
    }

    assert_mass_close(world.water_mass_at(0, 0), m_high0);
    assert!(
        world.surface_water_m_at(0, 0) <= MASS_EPSILON,
        "high must stay dry — no uphill creation"
    );
}

/// D1: same z and same h; equal H ⇒ no net transfer.
#[test]
#[allow(non_snake_case)]
fn d1_flat_equal_H_no_net_drain() {
    let h = 0.2;
    let z = 7.0;
    let cols = vec![saturated_column(z, h), saturated_column(z, h)];
    let mut world = World::grid(23, 2, 1, cols);
    let m0_a = world.water_mass_at(0, 0);
    let m0_b = world.water_mass_at(1, 0);
    let surf0_a = world.surface_water_m_at(0, 0);
    let surf0_b = world.surface_water_m_at(1, 0);

    for _ in 0..8 {
        world.tick();
    }

    assert_mass_close(world.water_mass_at(0, 0), m0_a);
    assert_mass_close(world.water_mass_at(1, 0), m0_b);
    assert_approx_eq(world.surface_water_m_at(0, 0), surf0_a, "no net drain A");
    assert_approx_eq(world.surface_water_m_at(1, 0), surf0_b, "no net drain B");
}

// ---------------------------------------------------------------------------
// Sprint 2.1 — D1 head-equalize (pool, no ping-pong)
// ε: 1e-9 relative on f64 water mass (MASS_EPSILON).
// Saturated soil (θ=φ) so pond stays on surface. Same z ⇒ no gravity drain.
// ---------------------------------------------------------------------------

/// S02.1: 1×2 same z, all pond on A; after enough ticks h_A ≈ h_B;
/// one extra tick does not swap (surfaces stay ≈ equal).
#[test]
fn d1_flat_pair_equalizes_no_oscillation() {
    let pond = 0.2;
    let z = 0.0;
    let cols = vec![
        saturated_column(z, pond), // A — all pond
        saturated_column(z, 0.0),  // B — dry
    ];
    let mut world = World::grid(31, 2, 1, cols);
    let m0 = world.grid_water_mass();

    // Half-drop + R_max: equalizes in a few ticks. Stay < N_ET so ET does not eat the pond.
    for _ in 0..8 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let h_a = world.surface_water_m_at(0, 0);
    let h_b = world.surface_water_m_at(1, 0);
    assert_approx_eq(h_a, h_b, "surfaces should equalize");
    assert!(
        (h_a - pond * 0.5).abs() <= MASS_EPSILON.max(1e-6),
        "each cell should hold ~half the pond: h_a={h_a} expected≈{}",
        pond * 0.5
    );

    // One extra tick must not swap / oscillate.
    world.tick();
    let h_a2 = world.surface_water_m_at(0, 0);
    let h_b2 = world.surface_water_m_at(1, 0);
    assert_approx_eq(h_a2, h_b2, "after extra tick surfaces still equal");
    assert_approx_eq(h_a2, h_a, "A must not swap away");
    assert_approx_eq(h_b2, h_b, "B must not swap away");
    assert_mass_close(closed_mass(&world), m0);
}

/// S02.1: 1×3 same z, rain on center; both ends gain surface water.
#[test]
fn d1_same_z_three_share() {
    let z = 0.0;
    let cols = vec![
        saturated_column(z, 0.0),
        saturated_column(z, 0.0),
        saturated_column(z, 0.0),
    ];
    let mut world = World::grid(37, 3, 1, cols);
    let rain = 0.4;
    world.add_rain_at(1, 0, rain); // center
    let m0 = world.grid_water_mass();

    // Stay under N_ET so pond remains to share to the ends.
    for _ in 0..8 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let h_left = world.surface_water_m_at(0, 0);
    let h_right = world.surface_water_m_at(2, 0);
    assert!(
        h_left > MASS_EPSILON,
        "left end should gain surface water, got {h_left}"
    );
    assert!(
        h_right > MASS_EPSILON,
        "right end should gain surface water, got {h_right}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 3 — water mechanics (D3)
// ε: 1e-9 relative on f64 water mass (MASS_EPSILON).
// Sand default unless named clay/loam.
// ---------------------------------------------------------------------------

/// D3: same R, same z, 1×1; after 1 tick clay has more h_surf than sand (I_max).
#[test]
fn d3_clay_ponds_before_sand() {
    let r = 0.15;
    let z = 0.0;
    let theta = 0.1;

    let mut sand = World::new(101, textured_column(z, 0.0, theta, Texture::Sand));
    let mut clay = World::new(101, textured_column(z, 0.0, theta, Texture::Clay));
    sand.add_rain(r);
    clay.add_rain(r);

    sand.tick();
    clay.tick();

    let h_sand = sand.surface_water_m();
    let h_clay = clay.surface_water_m();
    assert!(
        h_clay > h_sand + MASS_EPSILON,
        "clay should pond more than sand after 1 tick: clay={h_clay} sand={h_sand}"
    );
    assert_mass_close(sand.water_mass(), sand.column_at(0, 0).water_mass());
}

/// D3: sand 1×1; R large; after 1 tick, soil mass increase ≤ I_max_sand + ε.
#[test]
#[allow(non_snake_case)] // contract name uses I_max
fn d3_infiltrate_respects_I_max() {
    let tex = Texture::Sand;
    let i_max = tex.i_max();
    let mut world = World::new(103, textured_column(0.0, 0.0, 0.1, tex));
    let soil0 = world.column_at(0, 0).soil_water_mass();
    let r = 1.0; // large rain
    world.add_rain(r);
    world.tick();

    let soil1 = world.column_at(0, 0).soil_water_mass();
    let infiltrated = soil1 - soil0;
    assert!(
        infiltrated <= i_max + MASS_EPSILON,
        "infiltrated={infiltrated} exceeds I_max={i_max}"
    );
    assert!(
        infiltrated > MASS_EPSILON,
        "some water should have infiltrated"
    );
    assert_mass_close(world.water_mass(), soil0 + r);
}

/// D3: 1×2 slope; both start θ=φ (sand); high θ drops toward θ_fc; low M rises.
#[test]
fn d3_hill_soil_drains_to_valley() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    // S04.1: soil flux is capped by unused pore room — valley must have pores
    // so hill mobile can drain into soil (not the old overflow-to-pond artifact).
    let cols = vec![
        textured_column(10.0, 0.0, phi, tex), // high saturated (mobile)
        textured_column(5.0, 0.0, fc, tex),   // low at θ_fc (unused pores)
    ];
    let mut world = World::grid(107, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let m_low0 = world.water_mass_at(1, 0);
    let theta_high0: Vec<f64> = world
        .column_at(0, 0)
        .layers
        .iter()
        .map(|l| l.theta)
        .collect();

    // Mobile ≈ 0.16 m; D_max = 0.04 → need ≥ 4 drain ticks (+ infiltrate of arrivals).
    // Cap < N_ET so gravity-drain ≥θ_fc is not confounded by soil ET.
    for _ in 0..9 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let col_high = world.column_at(0, 0);
    for (i, layer) in col_high.layers.iter().enumerate() {
        assert!(
            layer.theta < theta_high0[i] - MASS_EPSILON,
            "high layer {i} theta should drop: was {}, now {}",
            theta_high0[i],
            layer.theta
        );
        assert!(
            layer.theta + MASS_EPSILON >= fc,
            "high layer {i} must not go below θ_fc={fc}, got {}",
            layer.theta
        );
    }
    assert!(
        world.water_mass_at(1, 0) > m_low0 + MASS_EPSILON,
        "valley mass should rise"
    );
}

/// D3: 1×2 slope; high θ = θ_fc; after ticks high M unchanged (no leak).
#[test]
fn d3_below_fc_does_not_drain() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let cols = vec![
        textured_column(10.0, 0.0, fc, tex), // high at field capacity
        textured_column(5.0, 0.0, fc, tex),  // low at field capacity
    ];
    let mut world = World::grid(109, 2, 1, cols);
    let m_high0 = world.water_mass_at(0, 0);
    let m0 = world.grid_water_mass();

    // < N_ET: soil ET would otherwise lower cell mass without lateral drain.
    for _ in 0..9 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    assert_mass_close(world.water_mass_at(0, 0), m_high0);
    assert!(
        world.surface_water_m_at(1, 0) <= MASS_EPSILON,
        "no drain should create valley pond"
    );
}

/// D3: 1×2; low starts θ=φ, high dry-ish (θ≤θ_fc); high does not gain from low drain.
#[test]
fn d3_no_soil_drain_uphill() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    let cols = vec![
        textured_column(10.0, 0.0, fc * 0.5, tex), // high dry-ish
        textured_column(5.0, 0.0, phi, tex),       // low saturated
    ];
    let mut world = World::grid(113, 2, 1, cols);
    let m_high0 = world.water_mass_at(0, 0);
    let soil_high0 = world.column_at(0, 0).soil_water_mass();
    let m0 = world.grid_water_mass();

    for _ in 0..9 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    assert_mass_close(world.water_mass_at(0, 0), m_high0);
    assert_mass_close(world.column_at(0, 0).soil_water_mass(), soil_high0);
    assert!(
        world.surface_water_m_at(0, 0) <= MASS_EPSILON,
        "high must not gain pond from uphill-forbidden drain"
    );
}

/// D3: pond moves faster than soil — after 2 ticks more valley mass from pond-only (A) than soil-drain (B).
#[test]
fn d3_pond_moves_faster_than_soil() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    // Pond larger than I_max so some surface remains after rate-limited infiltrate
    // (soils at fc still have pore space). Pond runoff is the fast path.
    let pond = 0.5;

    // Case A: pond-only on high, soils at fc (no initial mobile soil).
    let cols_a = vec![
        textured_column(10.0, pond, fc, tex),
        textured_column(5.0, 0.0, fc, tex),
    ];
    let mut world_a = World::grid(127, 2, 1, cols_a);
    let m_low_a0 = world_a.water_mass_at(1, 0);

    // Case B: no pond, high at φ (mobile soil only).
    let cols_b = vec![
        textured_column(10.0, 0.0, phi, tex),
        textured_column(5.0, 0.0, fc, tex),
    ];
    let mut world_b = World::grid(127, 2, 1, cols_b);
    let m_low_b0 = world_b.water_mass_at(1, 0);

    for _ in 0..2 {
        world_a.tick();
        world_b.tick();
    }

    let gained_a = world_a.water_mass_at(1, 0) - m_low_a0;
    let gained_b = world_b.water_mass_at(1, 0) - m_low_b0;
    assert!(
        gained_a > gained_b + MASS_EPSILON,
        "pond path should deliver more to valley after 2 ticks: A={gained_a} B={gained_b}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 3.1 — equal-z lateral interflow (ε = MASS_EPSILON = 1e-9)
// Prefer sand. Capillary θ ≤ θ_fc does not move. Pond is S03.3 (R_max).
// ---------------------------------------------------------------------------

/// D31: 1×2 flat same z; A θ=φ, B θ=θ_fc; B θ rises, A θ falls; no oscillation swap.
#[test]
fn d31_equal_z_mobile_shares() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    let z = 10.0;
    let cols = vec![
        textured_column(z, 0.0, phi, tex), // A saturated
        textured_column(z, 0.0, fc, tex),  // B at field capacity
    ];
    let mut world = World::grid(131, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let theta_a0: Vec<f64> = world
        .column_at(0, 0)
        .layers
        .iter()
        .map(|l| l.theta)
        .collect();
    let theta_b0: Vec<f64> = world
        .column_at(1, 0)
        .layers
        .iter()
        .map(|l| l.theta)
        .collect();

    let soil_b0 = world.column_at(1, 0).soil_water_mass();
    let soil_a0 = world.column_at(0, 0).soil_water_mass();

    // Mobile ≈ 0.16 m; D_max = 0.04 → several ticks to share. Cap < N_ET (no soil ET).
    for _ in 0..9 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let col_a = world.column_at(0, 0);
    let col_b = world.column_at(1, 0);
    // A loses soil water overall; individual layers may redistribute via percolate.
    assert!(
        col_a.soil_water_mass() < soil_a0 - MASS_EPSILON,
        "A soil mass should fall: was {soil_a0}, now {}",
        col_a.soil_water_mass()
    );
    for (i, layer) in col_a.layers.iter().enumerate() {
        assert!(
            layer.theta + MASS_EPSILON >= fc,
            "A layer {i} must stay ≥ θ_fc={fc}, got {}",
            layer.theta
        );
        // No layer should get wetter than start (A was saturated).
        assert!(
            layer.theta <= theta_a0[i] + MASS_EPSILON,
            "A layer {i} should not rise above start: was {}, now {}",
            theta_a0[i],
            layer.theta
        );
    }
    // B gains soil mass; profile-first may park water in deeper layers (top can stay at fc).
    assert!(
        col_b.soil_water_mass() > soil_b0 + MASS_EPSILON,
        "B soil mass should rise: was {soil_b0}, now {}",
        col_b.soil_water_mass()
    );
    let b_rose = col_b
        .layers
        .iter()
        .enumerate()
        .any(|(i, layer)| layer.theta > theta_b0[i] + MASS_EPSILON);
    assert!(b_rose, "at least one B layer θ should rise (got {:?})", 
        col_b.layers.iter().map(|l| l.theta).collect::<Vec<_>>());

    // No oscillation swap: one extra tick must not invert A↔B soil mass.
    let soil_a = col_a.soil_water_mass();
    let soil_b = col_b.soil_water_mass();
    world.tick();
    assert_mass_close(closed_mass(&world), m0);
    let soil_a1 = world.column_at(0, 0).soil_water_mass();
    let soil_b1 = world.column_at(1, 0).soil_water_mass();
    // If A was wetter, it should still be ≥ B (or equal within ε); never swap roles.
    if soil_a + MASS_EPSILON >= soil_b {
        assert!(
            soil_a1 + MASS_EPSILON >= soil_b1
                || (soil_a1 - soil_b1).abs() <= MASS_EPSILON,
            "extra tick swapped wet/dry roles: before A={soil_a} B={soil_b}, after A={soil_a1} B={soil_b1}"
        );
    }
    // Absolute soil masses should not leapfrog past each other by more than a tiny step.
    let before_gap = soil_a - soil_b;
    let after_gap = soil_a1 - soil_b1;
    assert!(
        !(before_gap > MASS_EPSILON && after_gap < -MASS_EPSILON),
        "oscillation swap detected: gap {before_gap} → {after_gap}"
    );
}

/// D31: 1×2 flat; A at θ_fc, B drier; A mobile/mass unchanged (no capillary share).
#[test]
fn d31_equal_z_below_fc_no_share() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let z = 10.0;
    let cols = vec![
        textured_column(z, 0.0, fc, tex),       // A at field capacity
        textured_column(z, 0.0, fc * 0.5, tex), // B drier
    ];
    let mut world = World::grid(137, 2, 1, cols);
    let m_a0 = world.water_mass_at(0, 0);
    let soil_a0 = world.column_at(0, 0).soil_water_mass();
    let mobile_a0 = world.column_at(0, 0).mobile_water_m();
    let m0 = world.grid_water_mass();

    for _ in 0..9 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    assert_mass_close(world.water_mass_at(0, 0), m_a0);
    assert_mass_close(world.column_at(0, 0).soil_water_mass(), soil_a0);
    assert_mass_close(world.column_at(0, 0).mobile_water_m(), mobile_a0);
    assert!(
        world.column_at(0, 0).mobile_water_m() <= MASS_EPSILON,
        "A should have no mobile water"
    );
}

/// D31: wet highland has same-z neighbor and lower-z neighbor; drain prefers valley surface.
/// 2×2: (0,0) z=10 θ=φ; (1,0) z=10 θ=θ_fc; (0,1) z=5 θ=θ_fc; (1,1) z=5 θ=θ_fc.
#[test]
fn d31_downslope_beats_equal_z() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    // Row-major y outer: (0,0), (1,0), (0,1), (1,1)
    let cols = vec![
        textured_column(10.0, 0.0, phi, tex), // (0,0) wet high
        textured_column(10.0, 0.0, fc, tex),  // (1,0) equal-z highland at fc
        textured_column(5.0, 0.0, fc, tex),   // (0,1) valley
        textured_column(5.0, 0.0, fc, tex),   // (1,1) valley
    ];
    let mut world = World::grid(139, 2, 2, cols);
    let m0 = world.grid_water_mass();
    let m_eq0 = world.water_mass_at(1, 0);
    let mobile_eq0 = world.column_at(1, 0).mobile_water_m();
    let m_valley0 = world.water_mass_at(0, 1);

    for _ in 0..8 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let valley_gain = world.water_mass_at(0, 1) - m_valley0;
    assert!(
        valley_gain > MASS_EPSILON,
        "valley (0,1) should gain mass from preferential downslope drain, gain={valley_gain}"
    );
    // Equal-z highland must not gain mobile via the preferential path.
    assert!(
        world.column_at(1, 0).mobile_water_m() <= mobile_eq0 + MASS_EPSILON,
        "equal-z neighbor must not gain mobile from high preferential path"
    );
    assert_mass_close(world.water_mass_at(1, 0), m_eq0);
    // Valley should have risen while equal-z stayed near fc (soil).
    for layer in world.column_at(1, 0).layers.iter() {
        assert!(
            (layer.theta - fc).abs() <= 1e-6 || layer.theta <= fc + 1e-6,
            "equal-z highland layer should stay near θ_fc, got {}",
            layer.theta
        );
    }
}

// ---------------------------------------------------------------------------
// Sprint 3.2 — profile-first percolation then lateral (ε = MASS_EPSILON = 1e-9)
// Two-layer columns unless noted. Sand default. Capillary ≤θ_fc does not move.
// ---------------------------------------------------------------------------

/// D32: 1×2 same z; A top=φ bottom=θ_fc; B at θ_fc; after 1 tick A's bottom rose
/// and B gained less than that internal fill (profile took water first).
#[test]
fn d32_top_percolates_before_lateral() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    let z = 10.0;
    let cols = vec![
        Column::new(
            z,
            0.0,
            vec![
                SoilLayer::new(0.3, phi, tex), // A top saturated
                SoilLayer::new(0.5, fc, tex),  // A bottom at fc
            ],
        ),
        Column::new(
            z,
            0.0,
            vec![
                SoilLayer::new(0.3, fc, tex), // B at fc
                SoilLayer::new(0.5, fc, tex),
            ],
        ),
    ];
    let mut world = World::grid(149, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let theta_a_bot0 = world.column_at(0, 0).layers[1].theta;
    let m_b0 = world.water_mass_at(1, 0);

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let theta_a_bot1 = world.column_at(0, 0).layers[1].theta;
    let internal_fill = (theta_a_bot1 - theta_a_bot0) * world.column_at(0, 0).layers[1].thickness_m;
    assert!(
        internal_fill > MASS_EPSILON,
        "A bottom θ should rise via percolate: was {theta_a_bot0}, now {theta_a_bot1}"
    );
    let b_gain = world.water_mass_at(1, 0) - m_b0;
    assert!(
        b_gain + MASS_EPSILON < internal_fill,
        "B gain ({b_gain}) should be less than A's internal fill ({internal_fill}) — profile first"
    );
}

/// D32: top at θ_fc, bottom drier; top layer water unchanged by percolate (no capillary move).
#[test]
fn d32_no_percolate_below_fc() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let dry = fc * 0.5;
    let col = Column::new(
        10.0,
        0.0,
        vec![
            SoilLayer::new(0.3, fc, tex),  // top at fc — no mobile
            SoilLayer::new(0.5, dry, tex), // bottom drier
        ],
    );
    let mut world = World::new(151, col);
    let top_m0 = world.column_at(0, 0).layers[0].water_mass();
    let top_theta0 = world.column_at(0, 0).layers[0].theta;
    let m0 = world.water_mass();

    world.tick();
    assert_mass_close(world.water_mass(), m0);

    assert_mass_close(world.column_at(0, 0).layers[0].water_mass(), top_m0);
    assert_approx_eq(
        world.column_at(0, 0).layers[0].theta,
        top_theta0,
        "top theta unchanged by percolate when at θ_fc",
    );
}

/// D32: 1×2 slope; high mobile, low empty pores; low θ rises; low h_surf did not take whole increment.
#[test]
fn d32_downslope_fills_soil_before_pond() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let fc = tex.theta_fc();
    let cols = vec![
        Column::new(
            10.0,
            0.0,
            vec![
                SoilLayer::new(0.3, phi, tex),
                SoilLayer::new(0.5, phi, tex),
            ],
        ),
        Column::new(
            5.0,
            0.0,
            vec![
                SoilLayer::new(0.3, fc, tex), // empty pores above φ
                SoilLayer::new(0.5, fc, tex),
            ],
        ),
    ];
    let mut world = World::grid(157, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let m_low0 = world.water_mass_at(1, 0);
    let theta_low0: Vec<f64> = world
        .column_at(1, 0)
        .layers
        .iter()
        .map(|l| l.theta)
        .collect();
    let h_low0 = world.surface_water_m_at(1, 0);

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let m_gain = world.water_mass_at(1, 0) - m_low0;
    assert!(m_gain > MASS_EPSILON, "low column should gain mass, gain={m_gain}");

    let mut theta_rose = false;
    for (i, layer) in world.column_at(1, 0).layers.iter().enumerate() {
        if layer.theta > theta_low0[i] + MASS_EPSILON {
            theta_rose = true;
            break;
        }
    }
    assert!(theta_rose, "low θ should rise (soil fill before pond)");

    let h_gain = world.surface_water_m_at(1, 0) - h_low0;
    assert!(
        h_gain + MASS_EPSILON < m_gain,
        "low h_surf gain ({h_gain}) must not take the whole mass increment ({m_gain})"
    );
}

/// D32: 1×2 flat; sand at φ next to clay at θ_fc; after 1 tick flux ≤ D_max_clay + ε.
#[test]
fn d32_contact_limited_by_slower_soil() {
    let sand = Texture::Sand;
    let clay = Texture::Clay;
    let phi_s = sand.porosity();
    let fc_c = clay.theta_fc();
    let d_clay = clay.d_max();
    let z = 10.0;
    let cols = vec![
        Column::new(
            z,
            0.0,
            vec![
                SoilLayer::new(0.3, phi_s, sand),
                SoilLayer::new(0.5, phi_s, sand),
            ],
        ),
        Column::new(
            z,
            0.0,
            vec![
                SoilLayer::new(0.3, fc_c, clay),
                SoilLayer::new(0.5, fc_c, clay),
            ],
        ),
    ];
    let mut world = World::grid(163, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let m_clay0 = world.water_mass_at(1, 0);

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let flux = world.water_mass_at(1, 0) - m_clay0;
    assert!(
        flux > MASS_EPSILON,
        "some flux should reach clay, got {flux}"
    );
    assert!(
        flux <= d_clay + MASS_EPSILON,
        "contact flux {flux} must be ≤ D_max_clay={d_clay} + ε"
    );
}

// ---------------------------------------------------------------------------
// Sprint 3.3 — pond head + R_max (ε = MASS_EPSILON = 1e-9; R_MAX = 0.15)
// Sand ok. Unique-min-H chute revoked. Soils at φ for pond-only scripts.
// ---------------------------------------------------------------------------

/// D33: 1×2 slope Δz=6; high h=0.50 at φ; after 1 tick high h > 0 and
/// high lost ≤ R_max + ε to the downhill edge.
#[test]
fn d33_steep_face_does_not_empty_in_one_tick() {
    let pond = 0.50;
    let cols = vec![
        saturated_column(6.0, pond), // high
        saturated_column(0.0, 0.0),  // low
    ];
    let mut world = World::grid(173, 2, 1, cols);
    let h0 = world.surface_water_m_at(0, 0);
    let m0 = world.grid_water_mass();
    assert!((h0 - pond).abs() <= MASS_EPSILON);

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let h1 = world.surface_water_m_at(0, 0);
    let lost = h0 - h1;
    assert!(
        h1 > MASS_EPSILON,
        "steep face must not empty high in one tick: h={h1}"
    );
    assert!(
        lost <= R_MAX + MASS_EPSILON,
        "high lost {lost} exceeds R_MAX={R_MAX} + ε"
    );
    assert!(
        lost > MASS_EPSILON,
        "some pond should move downhill, lost={lost}"
    );
}

/// D33: 2×2 — high-A pond, high-B same z dry, valley under A empty;
/// after 1 tick high-B h > 0 AND valley M rose (face capped ⇒ same-z edge gets flux).
#[test]
fn d33_same_z_gets_pond_when_face_capped() {
    let pond = 0.50;
    // Row-major: (0,0) high-A, (1,0) high-B, (0,1) valley-A, (1,1) valley-B
    let cols = vec![
        saturated_column(6.0, pond), // (0,0) high-A
        saturated_column(6.0, 0.0),  // (1,0) high-B same z dry
        saturated_column(0.0, 0.0),  // (0,1) valley under A
        saturated_column(0.0, 0.0),  // (1,1) valley
    ];
    let mut world = World::grid(179, 2, 2, cols);
    let m_valley0 = world.water_mass_at(0, 1);
    let m0 = world.grid_water_mass();

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let h_b = world.surface_water_m_at(1, 0);
    assert!(
        h_b > MASS_EPSILON,
        "high-B should receive same-z pond when steep face is R_max-capped, h_b={h_b}"
    );
    assert!(
        world.water_mass_at(0, 1) > m_valley0 + MASS_EPSILON,
        "valley under A should rise"
    );
}

/// D33: 1×2 Δz=6; enough pond on low that H_low ≈ H_high; after N≥15 ticks
/// |H_high−H_low| shrinks; last 3 ticks each |h| change < 1e-4 (no oscillation).
#[test]
fn d33_drowned_lake_heads_equalize_no_oscillation() {
    // high: z=6 h=0.2 → H=6.2; low: z=0 h=6.0 → H=6.0; near-equal drowned lake
    let h_high0 = 0.20;
    let h_low0 = 6.00;
    let cols = vec![
        saturated_column(6.0, h_high0),
        saturated_column(0.0, h_low0),
    ];
    let mut world = World::grid(181, 2, 1, cols);
    let m0 = world.grid_water_mass();
    let dh0 = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    assert!(dh0 > MASS_EPSILON, "start with a small head gap");

    // End just before an ET tick so last-Δh checks are hydro-only (S05).
    let n = 19usize;
    let mut h_hist: Vec<(f64, f64)> = Vec::with_capacity(n + 1);
    h_hist.push((
        world.surface_water_m_at(0, 0),
        world.surface_water_m_at(1, 0),
    ));

    for _ in 0..n {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
        h_hist.push((
            world.surface_water_m_at(0, 0),
            world.surface_water_m_at(1, 0),
        ));
    }

    let dh_final = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    assert!(
        dh_final < dh0 - 1e-6,
        "|H_high-H_low| should shrink: start={dh0}, end={dh_final}"
    );

    // Last 3 hydro ticks (no ET): each |h| change < 1e-4 on both cells.
    assert!(h_hist.len() >= 4);
    for k in (h_hist.len() - 3)..h_hist.len() {
        let (ha0, hb0) = h_hist[k - 1];
        let (ha1, hb1) = h_hist[k];
        let da = (ha1 - ha0).abs();
        let db = (hb1 - hb0).abs();
        assert!(
            da < 1e-4,
            "last ticks: |Δh_high|={da} not < 1e-4 (tick index {k})"
        );
        assert!(
            db < 1e-4,
            "last ticks: |Δh_low|={db} not < 1e-4 (tick index {k})"
        );
    }
}

/// D33: two cells any z with H equal ⇒ no net pond move.
#[test]
#[allow(non_snake_case)]
fn d33_equal_H_no_flux() {
    // Different z, compensating h so H equal: z=10 h=1 and z=9 h=2 → H=11.
    // Empty soil (no pore / no mobile) so infiltrate and soil drain cannot change H.
    let cols = vec![
        Column::new(10.0, 1.0, vec![]),
        Column::new(9.0, 2.0, vec![]),
    ];
    let mut world = World::grid(191, 2, 1, cols);
    let m_a0 = world.water_mass_at(0, 0);
    let m_b0 = world.water_mass_at(1, 0);
    let h_a0 = world.surface_water_m_at(0, 0);
    let h_b0 = world.surface_water_m_at(1, 0);
    assert_approx_eq(
        world.column_at(0, 0).head(),
        world.column_at(1, 0).head(),
        "heads equal at start",
    );

    for _ in 0..8 {
        world.tick();
    }

    assert_mass_close(world.water_mass_at(0, 0), m_a0);
    assert_mass_close(world.water_mass_at(1, 0), m_b0);
    assert_approx_eq(world.surface_water_m_at(0, 0), h_a0, "no net pond move A");
    assert_approx_eq(world.surface_water_m_at(1, 0), h_b0, "no net pond move B");
}

// ---------------------------------------------------------------------------
// Sprint 3.3.1 — pond receiver cap (ε = MASS_EPSILON; R_MAX = 0.15)
// Donor scale first, then receiver room = max(0, min(donor H) − H_j).
// ---------------------------------------------------------------------------

/// D331: 1×3 same-z; unequal donors A,B into empty mid R; after 1 tick
/// H_R ≤ min(donor snapshot H); no head inversion past either donor.
#[test]
fn d331_multi_donor_no_head_inversion() {
    // Empty layers ⇒ pond-only (no infiltrate / soil drain).
    // A: h=0.40 H=0.40; R: h=0 H=0; B: h=0.25 H=0.25.
    // Without receiver cap In≈0.275 > room=0.25 ⇒ H_R would pass min donor H.
    let cols = vec![
        Column::new(0.0, 0.40, vec![]), // A donor
        Column::new(0.0, 0.0, vec![]),  // R receiver
        Column::new(0.0, 0.25, vec![]), // B donor (lower H)
    ];
    let mut world = World::grid(3311, 3, 1, cols);
    let m0 = world.grid_water_mass();
    let h_a0 = world.column_at(0, 0).head();
    let h_b0 = world.column_at(2, 0).head();
    let min_donor_h = h_a0.min(h_b0);
    assert!(h_a0 > world.column_at(1, 0).head());
    assert!(h_b0 > world.column_at(1, 0).head());

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    let h_r = world.column_at(1, 0).head();
    assert!(
        h_r <= min_donor_h + MASS_EPSILON,
        "receiver H={h_r} must be ≤ min donor snapshot H={min_donor_h} (no head inversion)"
    );
    assert!(
        world.surface_water_m_at(1, 0) > MASS_EPSILON,
        "receiver should receive some pond"
    );
}

/// D331: drowned lake that used to period-2 bounce — 1×2 near-equal H
/// plus multi-donor A–V–B unequal donors. Empty soil ⇒ pond-only.
/// S04: pond stops at H_REST, so final |ΔH| ≤ H_REST (not bit-equal).
#[test]
fn d331_drowned_lake_no_period2() {
    // Critic-style heads on 1×2 (empty layers = pond-only).
    // Start ΔH > H_REST so pond moves, then deadband freezes.
    // high: z=8 h=0.50 → H=8.50; valley: z=0 h=8.20 → H=8.20; ΔH=0.30.
    let cols_pair = vec![
        Column::new(8.0, 0.50, vec![]),
        Column::new(0.0, 8.20, vec![]),
    ];
    let mut world = World::grid(3312, 2, 1, cols_pair);
    let m0 = world.grid_water_mass();
    let dh0 = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    assert!(dh0 > H_REST, "start with head gap above H_REST");

    // End before ET tick 40 so freeze checks are hydro-only.
    let n = 39usize;
    let mut h_hist: Vec<(f64, f64)> = Vec::with_capacity(n + 1);
    h_hist.push((
        world.surface_water_m_at(0, 0),
        world.surface_water_m_at(1, 0),
    ));

    for _ in 0..n {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
        h_hist.push((
            world.surface_water_m_at(0, 0),
            world.surface_water_m_at(1, 0),
        ));
    }

    let dh_final = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    assert!(
        dh_final <= H_REST + MASS_EPSILON,
        "1×2 heads should reach H_REST band: |ΔH|={dh_final} (start was {dh0}, H_REST={H_REST})"
    );
    assert!(
        dh_final < dh0 - 1e-4,
        "1×2 |ΔH| should shrink: start={dh0}, end={dh_final}"
    );
    assert!(h_hist.len() >= 6);
    for k in (h_hist.len() - 5)..h_hist.len() {
        let (ha0, hb0) = h_hist[k - 1];
        let (ha1, hb1) = h_hist[k];
        assert!(
            (ha1 - ha0).abs() < 1e-4,
            "last ticks: |Δh_high| not < 1e-4 (tick {k}) — period-2?"
        );
        assert!(
            (hb1 - hb0).abs() < 1e-4,
            "last ticks: |Δh_valley| not < 1e-4 (tick {k}) — period-2?"
        );
    }

    // Multi-donor drowned A–V–B with unequal high ponds (receiver-cap case).
    // Without cap, mid overshoots min donor H and period-2 bounces.
    let cols3 = vec![
        Column::new(8.0, 0.50, vec![]), // A H=8.50
        Column::new(0.0, 8.20, vec![]), // V H=8.20
        Column::new(8.0, 0.30, vec![]), // B H=8.30 (lower donor)
    ];
    let mut w3 = World::grid(3313, 3, 1, cols3);
    let m3 = w3.grid_water_mass();
    let dh3_0 = (w3.column_at(0, 0).head() - w3.column_at(1, 0).head()).abs();
    let mut v_hist = Vec::with_capacity(41);
    v_hist.push(w3.surface_water_m_at(1, 0));
    for _ in 0..39 {
        w3.tick();
        assert_mass_close(w3.grid_water_mass() + w3.et_lost() + w3.extract_lost(), m3);
        v_hist.push(w3.surface_water_m_at(1, 0));
    }
    let dh3 = (w3.column_at(0, 0).head() - w3.column_at(1, 0).head()).abs();
    assert!(
        dh3 < dh3_0 - 1e-4,
        "A–V–B |H| gap should shrink: start={dh3_0}, end={dh3}"
    );
    for k in (v_hist.len() - 5)..v_hist.len() {
        let dv = (v_hist[k] - v_hist[k - 1]).abs();
        assert!(
            dv < 1e-3,
            "A–V–B last ticks: |Δh_V|={dv} not < 1e-3 (tick {k}) — period-2?"
        );
    }
}

// ---------------------------------------------------------------------------
// Sprint 4 — hydro rest (H_REST / V_REST floors + sleep)
// ---------------------------------------------------------------------------

/// D4: two cells with ΔH = 0.01 ≤ H_REST; after one tick both h unchanged.
#[test]
#[allow(non_snake_case)]
fn d4_pond_below_H_rest_no_flux() {
    // Empty soil ⇒ pond-only. Same z; ΔH = 0.01 ≤ H_REST=0.02.
    let cols = vec![
        Column::new(0.0, 0.11, vec![]),
        Column::new(0.0, 0.10, vec![]),
    ];
    let mut world = World::grid(401, 2, 1, cols);
    let h_a0 = world.surface_water_m_at(0, 0);
    let h_b0 = world.surface_water_m_at(1, 0);
    let dh = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    assert!((dh - 0.01).abs() < 1e-12, "ΔH should be 0.01, got {dh}");
    assert!(dh <= H_REST);

    world.tick();

    assert_approx_eq(world.surface_water_m_at(0, 0), h_a0, "h_a unchanged below H_REST");
    assert_approx_eq(world.surface_water_m_at(1, 0), h_b0, "h_b unchanged below H_REST");
}

/// D4: 2×3 drowned lake like critic; ≤100 ticks → grid_at_rest; then h frozen.
#[test]
fn d4_drowned_lake_reaches_rest() {
    // 3×2 critic layout: valley z=2, high z=8. Empty soil ⇒ pond-only (soil m already
    // "frozen"); near-equal H around 8.15–8.25 equalizes into H_REST band then sleeps.
    let mk = |z: f64, h: f64| Column::new(z, h, vec![]);
    // Row-major y=0 valley, y=1 highland. Slight H spread like critic 8.11–8.21.
    let cols = vec![
        mk(2.0, 6.20), // (0,0) H=8.20
        mk(2.0, 6.18), // (1,0) H=8.18
        mk(2.0, 6.22), // (2,0) H=8.22
        mk(8.0, 0.25), // (0,1) H=8.25
        mk(8.0, 0.15), // (1,1) H=8.15
        mk(8.0, 0.21), // (2,1) H=8.21
    ];
    let mut world = World::grid(404, 3, 2, cols);
    let m0 = world.grid_water_mass();

    let mut reached = false;
    let mut n_rest = 0usize;
    for n in 1..=100 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
        if world.grid_at_rest() {
            reached = true;
            n_rest = n;
            break;
        }
    }
    if !reached {
        let mut heads = Vec::new();
        for y in 0..2 {
            for x in 0..3 {
                heads.push(world.column_at(x, y).head());
            }
        }
        panic!("grid_at_rest() should be true within 100 ticks; last heads: {heads:?}");
    }

    let mut h_snap = Vec::new();
    for y in 0..2 {
        for x in 0..3 {
            h_snap.push(world.surface_water_m_at(x, y));
        }
    }

    // Between ET-steps, sleep holds and h is frozen. Stop before the next ET wake.
    let t = world.tick_count();
    let safe = if t % N_ET == 0 {
        N_ET - 1
    } else {
        N_ET - (t % N_ET) - 1
    };
    for _ in 0..safe {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
        assert!(world.grid_at_rest(), "must stay at rest once reached (after {n_rest} ticks)");
    }

    let mut i = 0usize;
    for y in 0..2 {
        for x in 0..3 {
            let b = world.surface_water_m_at(x, y);
            let a = h_snap[i];
            assert!(
                (a - b).abs() <= MASS_EPSILON,
                "h frozen after rest: cell {i} {a} → {b}"
            );
            i += 1;
        }
    }
}

/// D4: rest grid; add rain one cell; that cell not at_rest after wake tick.
#[test]
fn d4_rain_wakes_rest() {
    // Flat saturated sand, equal ponds → immediately rests after a tick or two.
    let z = 0.0;
    let h = 0.10;
    let cols = vec![
        saturated_column(z, h),
        saturated_column(z, h),
        saturated_column(z, h),
        saturated_column(z, h),
    ];
    let mut world = World::grid(405, 2, 2, cols);
    for _ in 0..8 {
        world.tick();
    }
    assert!(
        world.grid_at_rest(),
        "equal-H saturated grid should be at rest"
    );

    world.add_rain_at(0, 0, 0.05);
    assert!(
        !world.cell_at_rest(0, 0),
        "add_rain must clear at_rest immediately"
    );

    world.tick(); // wake tick
    assert!(
        !world.cell_at_rest(0, 0),
        "rained cell must not be at_rest after wake tick"
    );
}

/// D4: rest path still conserves closed-grid mass (I3).
#[test]
fn d4_closed_mass_still_conserved() {
    let cols = vec![
        saturated_column(10.0, 0.0),
        saturated_column(5.0, 0.0),
        saturated_column(10.0, 0.0),
        saturated_column(5.0, 0.0),
    ];
    let mut world = World::grid(406, 2, 2, cols);
    let r = 0.35;
    world.add_rain_at(0, 0, r);
    let m0 = world.grid_water_mass();

    for _ in 0..64 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    // Drive toward rest and keep checking mass.
    for _ in 0..64 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    assert_mass_close(closed_mass(&world), m0);
    let _ = V_REST; // floor is part of the rest path under test
}


// ---------------------------------------------------------------------------
// Sprint 4.1 — lake snap / soil pore-room cap
// ---------------------------------------------------------------------------

/// D41: donor mobile, receiver both layers at φ; after tick receiver θ unchanged;
/// donor mobile unchanged if no lower-z path.
#[test]
fn d41_no_soil_into_full() {
    // Equal-z: clay at φ has more mobile than sand at φ, so the old equal-m share
    // would try to send — but sand has unused pore room = 0, so V must be 0.
    let clay = Texture::Clay;
    let sand = Texture::Sand;
    let z = 5.0; // equal z — no lower-z path
    let cols = vec![
        textured_column(z, 0.0, clay.porosity(), clay), // donor: more mobile
        textured_column(z, 0.0, sand.porosity(), sand), // receiver: both layers at φ
    ];
    let mut world = World::grid(411, 2, 1, cols);
    let theta_recv0: Vec<f64> = world
        .column_at(1, 0)
        .layers
        .iter()
        .map(|l| l.theta)
        .collect();
    let mobile_don0 = world.column_at(0, 0).mobile_water_m();
    let mobile_recv0 = world.column_at(1, 0).mobile_water_m();
    let m0 = world.grid_water_mass();
    assert!(mobile_don0 > V_REST, "donor must start with mobile water");
    assert!(
        mobile_don0 > mobile_recv0,
        "donor mobile must exceed receiver so equal-z share would otherwise fire"
    );
    assert!(
        world.column_at(1, 0).remaining_pore_capacity_m() <= MASS_EPSILON,
        "receiver must be at φ"
    );

    world.tick();
    assert_mass_close(closed_mass(&world), m0);

    for (i, layer) in world.column_at(1, 0).layers.iter().enumerate() {
        assert_approx_eq(layer.theta, theta_recv0[i], &format!("receiver θ[{i}] unchanged"));
    }
    assert_approx_eq(
        world.column_at(0, 0).mobile_water_m(),
        mobile_don0,
        "donor mobile unchanged (no lower-z path, receiver full)",
    );
    // Receiver must not have gained pond from a blocked soil push.
    assert!(
        world.surface_water_m_at(1, 0) <= MASS_EPSILON,
        "blocked soil flux must not become pond on receiver"
    );
}

/// D41: 1×3 valley equal z, saturated, pond ~2 m with ΔH < 0.05;
/// after ≤5 ticks all H equal within 1e-9 and grid_at_rest.
#[test]
fn d41_valley_lake_snaps() {
    let z = 2.0;
    // Pond depths ~2 m with pairwise ΔH < 0.05 (within a few pond ticks of H_REST band).
    let cols = vec![
        saturated_column(z, 2.00),
        saturated_column(z, 2.03),
        saturated_column(z, 1.99),
    ];
    let mut world = World::grid(412, 3, 1, cols);
    let dh01 = (world.column_at(0, 0).head() - world.column_at(1, 0).head()).abs();
    let dh12 = (world.column_at(1, 0).head() - world.column_at(2, 0).head()).abs();
    let dh02 = (world.column_at(0, 0).head() - world.column_at(2, 0).head()).abs();
    assert!(dh01 < 0.05 && dh12 < 0.05 && dh02 < 0.05, "ΔH < 0.05 precondition");

    let mut ok = false;
    for _ in 1..=5 {
        world.tick();
        let h0 = world.column_at(0, 0).head();
        let h1 = world.column_at(1, 0).head();
        let h2 = world.column_at(2, 0).head();
        if (h0 - h1).abs() <= 1e-9
            && (h1 - h2).abs() <= 1e-9
            && (h0 - h2).abs() <= 1e-9
            && world.grid_at_rest()
        {
            ok = true;
            break;
        }
    }
    assert!(
        ok,
        "after ≤5 ticks heads must match within 1e-9 and grid_at_rest; got H={:?} rest={}",
        [
            world.column_at(0, 0).head(),
            world.column_at(1, 0).head(),
            world.column_at(2, 0).head()
        ],
        world.grid_at_rest()
    );
}

/// D41: same valley script; M after = M before.
#[test]
fn d41_snap_conserves_mass() {
    let z = 2.0;
    let cols = vec![
        saturated_column(z, 2.00),
        saturated_column(z, 2.03),
        saturated_column(z, 1.99),
    ];
    let mut world = World::grid(413, 3, 1, cols);
    let m0 = world.grid_water_mass();
    for _ in 0..5 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    assert_mass_close(closed_mass(&world), m0);
}

/// D41: z=8 dry high next to z=2 lake; high h stays 0 (not pulled into valley component).
#[test]
fn d41_high_dry_not_in_valley_component() {
    // 1×2: valley lake + dry highland. Valley has pond; high is dry (h=0).
    let cols = vec![
        saturated_column(2.0, 2.01), // lake
        saturated_column(8.0, 0.0),  // dry high (soil sat, no pond)
    ];
    let mut world = World::grid(414, 2, 1, cols);
    assert!(world.surface_water_m_at(1, 0) <= MASS_EPSILON);
    let m0 = world.grid_water_mass();

    for _ in 0..5 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
        assert!(
            world.surface_water_m_at(1, 0) <= MASS_EPSILON,
            "high dry cell must keep h=0, got {}",
            world.surface_water_m_at(1, 0)
        );
    }
}

// ---------------------------------------------------------------------------
// Sprint 5 — batched evaporation (closed basin)
// ET when clock.tick % N_ET == 0 after advance (ticks 10,20,…). E*1 per ET-step.
// ---------------------------------------------------------------------------

/// Advance until the next ET-step completes (clock.tick % N_ET == 0 after tick).
fn tick_through_next_et(world: &mut World) {
    loop {
        world.tick();
        if world.tick_count() % N_ET == 0 {
            break;
        }
    }
}

/// D5: pond against map edge; many ticks; M + et_lost conserved; no off-grid dump.
#[test]
fn d5_no_edge_leak() {
    // Corner/edge pond on a 2×2 map — missing neighbors are glass.
    let cols = vec![
        saturated_column(0.0, 0.5), // (0,0) edge pond
        saturated_column(0.0, 0.0),
        saturated_column(0.0, 0.0),
        saturated_column(0.0, 0.0),
    ];
    let mut world = World::grid(501, 2, 2, cols);
    let m0 = world.grid_water_mass();
    assert_approx_eq(world.et_lost(), 0.0, "et starts at 0");

    for _ in 0..50 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    assert!(world.et_lost() > MASS_EPSILON, "ET should have removed some water");
    assert_mass_close(closed_mass(&world), m0);
}

/// D5: h>0 and wet top; one ET-step; h dropped, θ_top unchanged.
#[test]
fn d5_pond_evaps_before_soil() {
    let h0 = 0.10;
    // Saturated soil so pond stays on surface (no infiltrate room).
    let mut world = World::new(502, saturated_column(0.0, h0));
    let theta_top0 = world.column_at(0, 0).layers[0].theta;
    let m0 = world.grid_water_mass();

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    assert!(
        (h0 - h1 - E_OPEN).abs() <= 1e-9 || (h1 < h0 - MASS_EPSILON),
        "pond should drop by ~E_OPEN: h0={h0} h1={h1} E_OPEN={E_OPEN}"
    );
    assert!(
        (h0 - h1 - E_OPEN).abs() <= MASS_EPSILON.max(1e-9),
        "exact pond take: expected drop {E_OPEN}, got {}",
        h0 - h1
    );
    assert_approx_eq(
        world.column_at(0, 0).layers[0].theta,
        theta_top0,
        "θ_top unchanged when pond evaporates",
    );
    assert_mass_close(closed_mass(&world), m0);
    assert!(world.et_lost() > V_REST);
}

/// D5: h=0, top at θ_fc; enough ET-steps; θ_top < θ_fc.
#[test]
fn d5_soil_evaps_below_fc() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let cols = vec![field_capacity_column(0.0, 0.0)];
    let mut world = World::grid(503, 1, 1, cols);
    assert!(world.surface_water_m() <= MASS_EPSILON);
    let m0 = world.grid_water_mass();

    // Each ET takes E_SOIL from top (L=0.3) → Δθ = E_SOIL/0.3 per ET.
    assert!(E_SOIL > V_REST, "E_SOIL must exceed V_REST to actually evaporate");
    // A few ET-steps push θ below fc.
    for _ in 0..30 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let theta_top = world.column_at(0, 0).layers[0].theta;
    assert!(
        theta_top < fc - MASS_EPSILON,
        "θ_top should fall below θ_fc={fc}, got {theta_top}"
    );
    assert!(world.et_lost() > MASS_EPSILON);
}

/// D5: top dries; bot stays.
#[test]
fn d5_bot_untouched() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let col = Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, fc, tex),
            SoilLayer::new(0.5, fc, tex),
        ],
    );
    let mut world = World::new(504, col);
    let theta_bot0 = world.column_at(0, 0).layers[1].theta;
    let m0 = world.grid_water_mass();

    // Enough ET to dry the top layer (θ*L = fc*0.3 = 0.06; E_SOIL=0.005 → 12 ET-steps).
    for _ in 0..(N_ET as usize * 15) {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    let top = &world.column_at(0, 0).layers[0];
    let bot = &world.column_at(0, 0).layers[1];
    assert!(
        top.theta * top.thickness_m <= V_REST + MASS_EPSILON
            || top.theta < fc * 0.5,
        "top should be substantially dried, θ={}",
        top.theta
    );
    assert_approx_eq(bot.theta, theta_bot0, "bot layer untouched by ET");
}

/// D5: N_et hydro ticks with no ET-step in between; M unchanged until the ET tick.
#[test]
fn d5_batch_not_every_tick() {
    let mut world = World::new(505, saturated_column(0.0, 0.25));
    let m0 = world.grid_water_mass();
    assert_eq!(world.tick_count(), 0);
    assert_approx_eq(world.et_lost(), 0.0, "no ET yet");

    // Ticks 1..N_ET-1: hydro only, no ET.
    for i in 1..N_ET {
        world.tick();
        assert_eq!(world.tick_count(), i);
        assert_approx_eq(world.et_lost(), 0.0, &format!("no ET before tick {N_ET}"));
        assert_mass_close(world.grid_water_mass(), m0);
        assert_mass_close(closed_mass(&world), m0);
    }

    // Tick N_ET: ET fires.
    world.tick();
    assert_eq!(world.tick_count(), N_ET);
    assert!(
        world.et_lost() > V_REST,
        "ET-step at tick {N_ET} should evaporate, et_lost={}",
        world.et_lost()
    );
    assert!(
        world.grid_water_mass() < m0 - V_REST,
        "on-grid M should drop on the ET tick"
    );
    assert_mass_close(closed_mass(&world), m0);
    assert_approx_eq(world.et_lost(), E_OPEN, "one pond ET-step takes E_OPEN");
}

/// D5: rain then ET; M + et_lost == initial + rain.
#[test]
fn d5_mass_et_accounts() {
    let mut world = World::new(506, saturated_column(1.0, 0.0));
    let m_initial = world.grid_water_mass();
    let rain = 0.30;
    world.add_rain(rain);
    let m0 = world.grid_water_mass(); // initial + rain
    assert_mass_close(m0, m_initial + rain);

    for _ in 0..40 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }

    assert!(world.et_lost() > MASS_EPSILON, "rain then ET should lose some water");
    assert_mass_close(world.grid_water_mass() + world.et_lost() + world.extract_lost(), m_initial + rain);
}

// ---------------------------------------------------------------------------
// Sprint 6 — occupants + plant stub (ε = MASS_EPSILON = 1e-9)
// Sink cadence same as ET: after advance when tick % N_ET == 0; ET then occupants.
// ---------------------------------------------------------------------------

/// D6: one PlantStub, wet top; after one sink-step θ_top fell, bot unchanged.
#[test]
fn d6_plant_drinks_top() {
    let tex = Texture::Sand;
    // Both layers at θ_fc so no percolate/lateral before the sink-step; plant + ET
    // pull top (below fc OK); bot stays put with top-only root.
    let fc = tex.theta_fc();
    let col = Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, fc, tex),
            SoilLayer::new(0.5, fc, tex),
        ],
    );
    let mut world = World::new(601, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert!(world.plant_at(0, 0));
    assert_eq!(world.occupant_count(0, 0), 1);

    let theta_top0 = world.layer_theta_at(0, 0, 0);
    let theta_bot0 = world.layer_theta_at(0, 0, 1);
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let theta_top1 = world.layer_theta_at(0, 0, 0);
    let theta_bot1 = world.layer_theta_at(0, 0, 1);
    assert!(
        theta_top1 < theta_top0 - MASS_EPSILON,
        "θ_top should fall: {theta_top0} -> {theta_top1}"
    );
    assert_approx_eq(theta_bot1, theta_bot0, "bot unchanged with top-only root");
    assert!(world.extract_lost() > MASS_EPSILON, "plant should extract");
    assert_mass_close(closed_mass(&world), m0);
}

/// D6: root [0,1]; θ_bot falls, θ_top unchanged (pond shields top from ET).
#[test]
fn d6_mask_can_be_bot() {
    let tex = Texture::Sand;
    // Both layers saturated + pond ⇒ no percolate room; ET takes pond; bot-root drinks bot.
    let phi = tex.porosity();
    let col = Column::new(
        0.0,
        0.10, // pond
        vec![
            SoilLayer::new(0.3, phi, tex),
            SoilLayer::new(0.5, phi, tex),
        ],
    );
    let mut world = World::new(602, col);
    world
        .add_occupant(0, 0, Occupant::plant_stub_with_root([0.0, 1.0]))
        .unwrap();

    let theta_top0 = world.layer_theta_at(0, 0, 0);
    let theta_bot0 = world.layer_theta_at(0, 0, 1);
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let theta_top1 = world.layer_theta_at(0, 0, 0);
    let theta_bot1 = world.layer_theta_at(0, 0, 1);
    assert_approx_eq(theta_top1, theta_top0, "θ_top unchanged with bot root + pond ET");
    assert!(
        theta_bot1 < theta_bot0 - MASS_EPSILON,
        "θ_bot should fall: {theta_bot0} -> {theta_bot1}"
    );
    assert_approx_eq(world.extract_lost(), P_MAX, "one bot-root plant takes P_MAX");
    assert_mass_close(closed_mass(&world), m0);
    let _ = N_LAYERS; // used by Occupant root type
}

/// D6: two PlantStubs; extract ≈ 2*P_MAX (capped by water).
#[test]
fn d6_two_occupants_sum() {
    let tex = Texture::Sand;
    // Plenty of top water; pond so ET does not compete for soil mass budget.
    let col = Column::new(
        0.0,
        0.10,
        vec![
            SoilLayer::new(0.3, tex.porosity(), tex),
            SoilLayer::new(0.5, tex.porosity(), tex),
        ],
    );
    let mut world = World::new(603, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert_eq!(world.occupant_count(0, 0), 2);
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    assert_approx_eq(world.extract_lost(), 2.0 * P_MAX, "two plants extract ~2*P_MAX");
    assert_mass_close(closed_mass(&world), m0);
}

/// D6: dry top; after 3 sink-steps alive=false; further steps θ unchanged.
#[test]
fn d6_wilt_stops_uptake() {
    let tex = Texture::Sand;
    // Dry top (θ≈0) so plant takes nothing each sink-step; tiny bot water that
    // plant does not reach (root top-only). No pond ⇒ ET soil take also ~0.
    let col = Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, 0.0, tex),
            SoilLayer::new(0.5, 0.25, tex),
        ],
    );
    let mut world = World::new(604, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert!(world.plant_at(0, 0));

    // Three sink-steps → wilt.
    for step in 1..=3 {
        tick_through_next_et(&mut world);
        if step < 3 {
            assert!(
                world.plant_at(0, 0),
                "should still be alive after {step} dry sink-steps (T_WILT={T_WILT})"
            );
        }
    }
    assert!(
        !world.plant_at(0, 0),
        "plant should wilt after {T_WILT} dry sink-steps"
    );
    assert_approx_eq(world.extract_lost(), 0.0, "no water to extract while drying");

    let theta_top = world.layer_theta_at(0, 0, 0);
    let theta_bot = world.layer_theta_at(0, 0, 1);
    let extract0 = world.extract_lost();

    // Further sink-steps: θ unchanged, extract unchanged.
    for _ in 0..3 {
        tick_through_next_et(&mut world);
        assert_approx_eq(world.layer_theta_at(0, 0, 0), theta_top, "top frozen after wilt");
        assert_approx_eq(world.layer_theta_at(0, 0, 1), theta_bot, "bot frozen after wilt");
        assert_approx_eq(world.extract_lost(), extract0, "no further extract after wilt");
        assert!(!world.plant_at(0, 0));
    }
}

/// D6: M + et_lost + extract_lost conserved with occupants present.
#[test]
fn d6_extract_in_ledger() {
    let tex = Texture::Sand;
    let col = Column::new(
        0.0,
        0.20,
        vec![
            SoilLayer::new(0.3, tex.porosity(), tex),
            SoilLayer::new(0.5, 0.25, tex),
        ],
    );
    let mut world = World::new(605, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    let rain = 0.15;
    world.add_rain(rain);
    let m0 = closed_mass(&world);

    for _ in 0..40 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    assert!(world.et_lost() > MASS_EPSILON || world.extract_lost() > MASS_EPSILON);
    assert!(world.extract_lost() > MASS_EPSILON, "plant should have extracted");
    assert_mass_close(
        world.grid_water_mass() + world.et_lost() + world.extract_lost(),
        m0,
    );
}

/// D6: no occupants ⇒ extract_lost == 0.
#[test]
fn d6_bare_cell_no_extract() {
    let mut world = World::new(606, saturated_column(0.0, 0.25));
    assert_eq!(world.occupant_count(0, 0), 0);
    assert!(!world.plant_at(0, 0));
    let m0 = closed_mass(&world);

    for _ in 0..30 {
        world.tick();
        assert_approx_eq(world.extract_lost(), 0.0, "bare cell extract_lost");
        assert_mass_close(closed_mass(&world), m0);
    }
    assert!(world.et_lost() > MASS_EPSILON, "ET still runs on bare cells");
}

/// D6: 9th add_occupant fails.
#[test]
fn d6_cap_eight() {
    let mut world = World::new(607, default_column(0.2, 0.4));
    for i in 0..MAX_OCCUPANTS {
        world
            .add_occupant(0, 0, Occupant::plant_stub())
            .unwrap_or_else(|_| panic!("add {i} should succeed"));
    }
    assert_eq!(world.occupant_count(0, 0), MAX_OCCUPANTS);
    let err = world.add_occupant(0, 0, Occupant::plant_stub());
    assert!(err.is_err(), "9th occupant must fail");
    assert_eq!(world.occupant_count(0, 0), MAX_OCCUPANTS);

    world.clear_occupants(0, 0);
    assert_eq!(world.occupant_count(0, 0), 0);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert_eq!(world.occupant_count(0, 0), 1);
}


// ---------------------------------------------------------------------------
// Sprint 7 — shade scales ET (ε = MASS_EPSILON = 1e-9)
// S = clamp(sum alive shade, 0, 1); E_eff = E*(1-S). PlantStub shade default 0.25.
// ---------------------------------------------------------------------------

/// D7: no occupants; one ET-step; pond loss == E_OPEN.
#[test]
fn d7_bare_et_unchanged() {
    let h0 = 0.10;
    let mut world = World::new(701, saturated_column(0.0, h0));
    assert_eq!(world.occupant_count(0, 0), 0);
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    assert_approx_eq(h0 - h1, E_OPEN, "bare pond drop == E_OPEN");
    assert_approx_eq(world.et_lost(), E_OPEN, "et_lost == E_OPEN");
    assert_approx_eq(world.extract_lost(), 0.0, "no extract on bare cell");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: one alive PlantStub, h>0; pond loss == 0.75 * E_OPEN.
#[test]
fn d7_one_plant_cuts_pond_et() {
    let h0 = 0.10;
    let mut world = World::new(702, saturated_column(0.0, h0));
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert_approx_eq(
        world.column_at(0, 0).occupants[0].shade,
        0.25,
        "PlantStub default shade",
    );
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    let expected = 0.75 * E_OPEN;
    assert_approx_eq(h0 - h1, expected, "one plant pond drop == 0.75*E_OPEN");
    assert_approx_eq(world.et_lost(), expected, "et_lost == 0.75*E_OPEN");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: two alive; pond loss == 0.50 * E_OPEN.
#[test]
fn d7_two_sum() {
    let h0 = 0.10;
    let mut world = World::new(703, saturated_column(0.0, h0));
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    let expected = 0.50 * E_OPEN;
    assert_approx_eq(h0 - h1, expected, "two plants pond drop == 0.50*E_OPEN");
    assert_approx_eq(world.et_lost(), expected, "et_lost == 0.50*E_OPEN");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: four alive; pond loss == 0 (S capped at 1).
#[test]
fn d7_shade_cap_one() {
    let h0 = 0.10;
    let mut world = World::new(704, saturated_column(0.0, h0));
    for _ in 0..4 {
        world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    }
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    assert_approx_eq(h0 - h1, 0.0, "four plants: pond ET fully shaded");
    assert_approx_eq(world.et_lost(), 0.0, "et_lost == 0 when S=1");
    // Uptake still runs (plants drink soil, not pond).
    assert!(world.extract_lost() > MASS_EPSILON, "uptake still runs under full shade");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: wilted occupant contributes 0 shade; pond loss == E_OPEN.
#[test]
fn d7_wilt_no_shade() {
    let h0 = 0.10;
    let mut world = World::new(705, saturated_column(0.0, h0));
    let mut occ = Occupant::plant_stub();
    occ.alive = false;
    assert_approx_eq(occ.shade, 0.25, "wilted stub still has shade field 0.25");
    world.add_occupant(0, 0, occ).unwrap();
    assert!(!world.plant_at(0, 0));
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    let h1 = world.surface_water_m();
    assert_approx_eq(h0 - h1, E_OPEN, "wilted shade ignored: pond drop == E_OPEN");
    assert_approx_eq(world.et_lost(), E_OPEN, "et_lost == E_OPEN with wilted only");
    assert_approx_eq(world.extract_lost(), 0.0, "wilted does not extract");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: planted, wet top, h=0; extract still P_MAX (uptake unchanged by shade).
#[test]
fn d7_uptake_unchanged() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let col = Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, fc, tex),
            SoilLayer::new(0.5, fc, tex),
        ],
    );
    let mut world = World::new(706, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert!(world.surface_water_m() <= MASS_EPSILON);
    let m0 = closed_mass(&world);

    tick_through_next_et(&mut world);

    assert_approx_eq(world.extract_lost(), P_MAX, "shade does not change uptake");
    // Soil ET still runs at 0.75 * E_SOIL.
    assert_approx_eq(world.et_lost(), 0.75 * E_SOIL, "shaded soil ET");
    assert_mass_close(closed_mass(&world), m0);
}

/// D7: M + et_lost + extract_lost conserved with shade active.
#[test]
fn d7_ledger() {
    let tex = Texture::Sand;
    let col = Column::new(
        0.0,
        0.20,
        vec![
            SoilLayer::new(0.3, tex.porosity(), tex),
            SoilLayer::new(0.5, 0.25, tex),
        ],
    );
    let mut world = World::new(707, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    let rain = 0.15;
    world.add_rain(rain);
    let m0 = closed_mass(&world);

    for _ in 0..40 {
        world.tick();
        assert_mass_close(closed_mass(&world), m0);
    }
    assert!(world.et_lost() > MASS_EPSILON, "some ET under partial shade");
    assert!(world.extract_lost() > MASS_EPSILON, "plants extract");
    assert_mass_close(
        world.grid_water_mass() + world.et_lost() + world.extract_lost(),
        m0,
    );
}

// ---------------------------------------------------------------------------
// Sprint 8 — awake set + catch-up (ε = MASS_EPSILON = 1e-9; catch_up rain 5%)
// ---------------------------------------------------------------------------

/// D8: 16×16 all rest; tick; awake_count == 0.
#[test]
fn d8_desert_sleeps_zero_awake() {
    let w = 16usize;
    let h = 16usize;
    let tex = Texture::Sand;
    let cols: Vec<Column> = (0..w * h)
        .map(|_| {
            Column::new(
                0.0,
                0.0,
                vec![
                    SoilLayer::new(0.3, 0.0, tex),
                    SoilLayer::new(0.5, 0.0, tex),
                ],
            )
        })
        .collect();
    let mut world = World::grid(801, w, h, cols);
    // One hydro tick: no flux → all at_rest → prune leaves awake empty.
    world.tick();
    assert_eq!(world.awake_count(), 0, "desert should leave awake set");
    assert!(world.grid_at_rest());
    // Another tick still visits nobody.
    world.tick();
    assert_eq!(world.awake_count(), 0);
    let _ = T_SETTLE;
}

/// D8: rain one cell; it and its 4-neighbors are awake.
#[test]
fn d8_rain_wakes_neighbors() {
    let w = 5usize;
    let h = 5usize;
    let tex = Texture::Sand;
    let cols: Vec<Column> = (0..w * h)
        .map(|_| {
            Column::new(
                0.0,
                0.0,
                vec![
                    SoilLayer::new(0.3, 0.0, tex),
                    SoilLayer::new(0.5, 0.0, tex),
                ],
            )
        })
        .collect();
    let mut world = World::grid(802, w, h, cols);
    world.tick();
    assert_eq!(world.awake_count(), 0);

    let cx = 2usize;
    let cy = 2usize;
    world.add_rain_at(cx, cy, 0.05);
    assert!(!world.cell_at_rest(cx, cy));
    // Center + N,E,S,W
    let expected = [
        (cx, cy),
        (cx, cy + 1),
        (cx + 1, cy),
        (cx, cy - 1),
        (cx - 1, cy),
    ];
    assert_eq!(world.awake_count(), expected.len());
    for &(x, y) in &expected {
        assert!(!world.cell_at_rest(x, y), "woken cell ({x},{y}) must not be at_rest");
    }
    // Diagonal corners of the plus are not 4-neighbors — stay at rest.
    assert!(world.cell_at_rest(cx + 1, cy + 1));
    assert!(world.cell_at_rest(cx - 1, cy - 1));
}

/// D8: planted rest cell; catch_up(K=5, rain=0) wilts; extract and ET in ledger.
#[test]
fn d8_drought_wilt() {
    let tex = Texture::Sand;
    // Some top water so ET books a loss; plant root top-only will drink then go dry.
    // Use tiny top mass so K=5 sink-steps exhaust it and wilt.
    let col = Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, 0.02, tex), // ~0.006 m water — few plant drinks
            SoilLayer::new(0.5, 0.0, tex),
        ],
    );
    let mut world = World::new(803, col);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    // Rest the cell.
    for _ in 0..4 {
        world.tick();
    }
    // Advance off an ET boundary if needed so live cadence does not interfere;
    // catch_up applies its own sinks.
    assert!(world.plant_at(0, 0), "precondition: alive before drought catch_up");

    let m0 = closed_mass(&world);
    world.catch_up(5, 0.0);

    assert!(
        !world.plant_at(0, 0),
        "plant should wilt after catch_up(K=5>=T_WILT) drought"
    );
    assert!(world.extract_lost() > MASS_EPSILON, "extract in ledger");
    assert!(world.et_lost() > MASS_EPSILON, "ET in ledger");
    assert_mass_close(closed_mass(&world), m0);
}

/// D8: rest valley + plant; catch_up(K=3, rain>0); water rose then sinks; books close.
#[test]
fn d8_rain_fills_then_plants() {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    // 1×3: high, mid, valley. Plant in valley.
    let cols = vec![
        Column::new(2.0, 0.0, vec![SoilLayer::new(0.3, fc, tex), SoilLayer::new(0.5, fc, tex)]),
        Column::new(1.0, 0.0, vec![SoilLayer::new(0.3, fc, tex), SoilLayer::new(0.5, fc, tex)]),
        Column::new(0.0, 0.0, vec![SoilLayer::new(0.3, fc, tex), SoilLayer::new(0.5, fc, tex)]),
    ];
    let mut world = World::grid(804, 3, 1, cols);
    world.add_occupant(2, 0, Occupant::plant_stub()).unwrap();
    for _ in 0..8 {
        world.tick();
    }
    let h0 = world.surface_water_m_at(2, 0);
    let theta0 = world.layer_theta_at(2, 0, 0);
    let m_before = world.grid_water_mass();
    let et0 = world.et_lost();
    let ex0 = world.extract_lost();

    let k = 3u32;
    let rain = 0.05;
    world.catch_up(k, rain);

    let added = (k as f64) * rain * 3.0; // 3 cells
    let m_books = world.grid_water_mass() + world.et_lost() + world.extract_lost();
    assert_mass_close(m_books, m_before + added);

    let h1 = world.surface_water_m_at(2, 0);
    let theta1 = world.layer_theta_at(2, 0, 0);
    assert!(
        h1 > h0 + MASS_EPSILON || theta1 > theta0 + MASS_EPSILON
            || world.et_lost() > et0 + MASS_EPSILON
            || world.extract_lost() > ex0 + MASS_EPSILON,
        "catch_up should move water or apply sinks: h {h0}->{h1} θ {theta0}->{theta1}"
    );
    // Sinks applied (K>0).
    assert!(
        world.et_lost() >= et0 - MASS_EPSILON,
        "ET ledger non-decreasing"
    );
}

/// D8: rain=0; catch_up(K) vs K live sink-steps; θ,h,et,extract within 1e-9.
#[test]
fn d8_drought_matches_live_sinks() {
    let tex = Texture::Sand;
    let mk = || {
        let col = Column::new(
            0.0,
            0.08,
            vec![
                SoilLayer::new(0.3, tex.porosity(), tex),
                SoilLayer::new(0.5, tex.theta_fc(), tex),
            ],
        );
        let mut w = World::new(805, col);
        w.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
        // Rest hydro without hitting many ET steps: tick a few non-ET.
        for _ in 0..3 {
            w.tick();
        }
        w
    };

    let k = 4u32;
    let mut live = mk();
    let mut caught = mk();

    // Align clocks / state: clone-equivalent via same seed script.
    assert_eq!(live.tick_count(), caught.tick_count());
    assert_approx_eq(live.grid_water_mass(), caught.grid_water_mass(), "pre mass");

    // Live: K sink-steps (each N_ET hydro ticks ending on ET).
    for _ in 0..k {
        tick_through_next_et(&mut live);
    }

    caught.catch_up(k, 0.0);

    assert_approx_eq(
        caught.surface_water_m(),
        live.surface_water_m(),
        "h match drought",
    );
    assert_approx_eq(
        caught.layer_theta_at(0, 0, 0),
        live.layer_theta_at(0, 0, 0),
        "θ_top match",
    );
    assert_approx_eq(
        caught.layer_theta_at(0, 0, 1),
        live.layer_theta_at(0, 0, 1),
        "θ_bot match",
    );
    assert_approx_eq(caught.et_lost(), live.et_lost(), "et match");
    assert_approx_eq(caught.extract_lost(), live.extract_lost(), "extract match");
}

/// D8: rain only on high; after catch_up valley has water (settle routed it).
#[test]
fn d8_no_teleport_uphill() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    // Saturated so rain stays as pond and runs downhill.
    let cols = vec![
        Column::new(5.0, 0.0, vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)]),
        Column::new(0.0, 0.0, vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)]),
    ];
    let mut world = World::grid(806, 2, 1, cols);
    for _ in 0..4 {
        world.tick();
    }
    assert!(world.surface_water_m_at(1, 0) <= MASS_EPSILON);

    // Source only on high via catch_up rain on all cells — but test says rain only on high.
    // Use manual rain on high then catch_up settle path: catch_up adds rain everywhere.
    // Spec test: "rain only on high; after catch_up valley has water".
    // Interpret: add rain on high, then catch_up(K, 0) to settle; or catch_up with rain
    // then verify valley got water from settle (not teleport uphill from valley).
    // Stronger reading: rain on high only, catch_up drought settle moves it down.
    world.add_rain_at(0, 0, 0.40);
    let m0 = closed_mass(&world);
    world.catch_up(1, 0.0); // settle + one sink-step

    let valley_h = world.surface_water_m_at(1, 0);
    let valley_soil = world.column_at(1, 0).soil_water_mass();
    assert!(
        valley_h > MASS_EPSILON || valley_soil > phi * 0.3 + phi * 0.5 + MASS_EPSILON,
        "valley should receive water via settle, h={valley_h} soil={valley_soil}"
    );
    // High should not have gained from valley (no uphill teleport).
    assert_mass_close(closed_mass(&world), m0);
}

// ---------------------------------------------------------------------------
// Sprint 9 — region observe + scoped catch-up (CHUNK=8)
// ---------------------------------------------------------------------------

fn desert_grid(seed: u64, w: usize, h: usize) -> World {
    let tex = Texture::Sand;
    let cols: Vec<Column> = (0..w * h)
        .map(|_| {
            Column::new(
                0.0,
                0.0,
                vec![
                    SoilLayer::new(0.3, 0.0, tex),
                    SoilLayer::new(0.5, 0.0, tex),
                ],
            )
        })
        .collect();
    World::grid(seed, w, h, cols)
}

fn rest_all(world: &mut World) {
    // Drive hydro to rest; large deserts need a couple ticks.
    for _ in 0..4 {
        world.tick();
    }
    assert!(world.grid_at_rest(), "precondition: grid at rest");
}

fn chunk_xy(x: usize, y: usize) -> (usize, usize) {
    (x / CHUNK, y / CHUNK)
}

fn ignore_all_chunks(world: &mut World) {
    let ncx = (world.width() + CHUNK - 1) / CHUNK;
    let ncy = (world.height() + CHUNK - 1) / CHUNK;
    for cy in 0..ncy {
        for cx in 0..ncx {
            world.ignore_chunk(cx, cy).expect("rest chunk should ignore");
        }
    }
}

/// D9: 16×16 rest; ignore all; tick; hydro cell visits == 0.
#[test]
fn d9_ignore_zero_visits() {
    let mut world = desert_grid(901, 16, 16);
    rest_all(&mut world);
    assert_eq!(world.awake_count(), 0);
    ignore_all_chunks(&mut world);
    assert_eq!(world.observed_chunk_count(), 0);

    world.tick();
    assert_eq!(
        world.last_hydro_visits(),
        0,
        "ignored resting world must visit no hydro cells"
    );
    assert_eq!(world.awake_count(), 0);
}

/// D9: ignore all but (0,0); rain inside (0,0); only that chunk + halo awake.
#[test]
fn d9_observe_one_chunk_only() {
    let mut world = desert_grid(902, 16, 16);
    rest_all(&mut world);
    ignore_all_chunks(&mut world);
    world.observe_chunk(0, 0);
    assert_eq!(world.observed_chunk_count(), 1);

    // Rain near chunk edge so 4-neighbors include halo cells outside the chunk.
    let rx = CHUNK - 1;
    let ry = CHUNK - 1;
    assert_eq!(chunk_xy(rx, ry), (0, 0));
    world.add_rain_at(rx, ry, 0.05);

    // Awake = rain cell + 4-neighbors (includes (CHUNK, ry) and (rx, CHUNK) halo).
    let ncx = (world.width() + CHUNK - 1) / CHUNK;
    let ncy = (world.height() + CHUNK - 1) / CHUNK;
    for y in 0..world.height() {
        for x in 0..world.width() {
            let i_awake = !world.cell_at_rest(x, y);
            // Approximate awake via at_rest cleared by wake; also check awake_count path.
            let (cx, cy) = chunk_xy(x, y);
            let in_chunk0 = cx == 0 && cy == 0;
            let in_halo = {
                // 4-neighbor of any cell in chunk (0,0)
                let x0 = 0;
                let y0 = 0;
                let x1 = CHUNK.min(world.width());
                let y1 = CHUNK.min(world.height());
                let mut h = in_chunk0;
                if !h {
                    for yy in y0..y1 {
                        for xx in x0..x1 {
                            let man = (x as i32 - xx as i32).abs() + (y as i32 - yy as i32).abs();
                            if man == 1 {
                                h = true;
                                break;
                            }
                        }
                        if h {
                            break;
                        }
                    }
                }
                h
            };
            if i_awake {
                assert!(
                    in_halo,
                    "awake cell ({x},{y}) must lie in chunk (0,0) + halo"
                );
            }
            let _ = (ncx, ncy);
        }
    }
    assert!(world.awake_count() >= 1);
    // Halo cells outside chunk that are 4-neighbors of rain cell must be awake.
    assert!(!world.cell_at_rest(CHUNK, ry), "east halo awake");
    assert!(!world.cell_at_rest(rx, CHUNK), "south halo awake");
}

/// D9/S09.1: pond still flowing; ignore_chunk succeeds (chunk not observed).
/// Come-back catch_up owns the unfinished cascade.
#[test]
fn d9_cannot_ignore_moving() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    // 2×1 steep face with pond on high — will keep flowing for several ticks.
    let cols = vec![
        Column::new(
            5.0,
            0.5,
            vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)],
        ),
        Column::new(
            0.0,
            0.0,
            vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)],
        ),
    ];
    let mut world = World::grid(903, 2, 1, cols);
    // One tick so flux is happening / cells not all at_rest.
    world.tick();
    assert!(
        !world.grid_at_rest(),
        "precondition: pond still moving"
    );
    let before = world.observed_chunk_count();
    assert!(before >= 1);
    let res = world.ignore_chunk(0, 0);
    assert!(res.is_ok(), "S09.1: ignore while moving must succeed");
    assert_eq!(
        world.observed_chunk_count(),
        before - 1,
        "ignored chunk must leave the observed set"
    );
    let _ = ChunkBusy; // type retained in public API
}

/// D9: two chunks; catch_up_chunk A; B mass unchanged.
#[test]
fn d9_catchup_other_chunk_untouched() {
    // 16×8 → two chunks along x: (0,0) and (1,0).
    let w = 16usize;
    let h = 8usize;
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    let cols: Vec<Column> = (0..w * h)
        .map(|i| {
            let x = i % w;
            let elev = if x < CHUNK { 1.0 } else { 0.0 };
            Column::new(
                elev,
                0.0,
                vec![
                    SoilLayer::new(0.3, fc, tex),
                    SoilLayer::new(0.5, fc, tex),
                ],
            )
        })
        .collect();
    let mut world = World::grid(904, w, h, cols);
    rest_all(&mut world);

    // Cells strictly outside A + 1-cell halo (x >= CHUNK+1) must be untouched.
    let mut far_b0 = 0.0;
    for y in 0..h {
        for x in (CHUNK + 1)..w {
            far_b0 += world.water_mass_at(x, y);
        }
    }

    world.catch_up_chunk(0, 0, 2, 0.05);

    let mut far_b1 = 0.0;
    for y in 0..h {
        for x in (CHUNK + 1)..w {
            far_b1 += world.water_mass_at(x, y);
        }
    }
    assert_approx_eq(far_b1, far_b0, "far chunk-B mass unchanged");
}

/// D9: plant in A, ignore A, catch_up_chunk(A,K=5,rain=0); plant wilted; B intact.
#[test]
fn d9_drought_chunk_wilts() {
    let w = 16usize;
    let h = 8usize;
    let tex = Texture::Sand;
    // Tiny top water so drought K=5 wilts; B has more water so a mistaken sink would show.
    let cols: Vec<Column> = (0..w * h)
        .map(|i| {
            let x = i % w;
            if x < CHUNK {
                Column::new(
                    0.0,
                    0.0,
                    vec![
                        SoilLayer::new(0.3, 0.02, tex),
                        SoilLayer::new(0.5, 0.0, tex),
                    ],
                )
            } else {
                Column::new(
                    0.0,
                    0.0,
                    vec![
                        SoilLayer::new(0.3, 0.15, tex),
                        SoilLayer::new(0.5, 0.10, tex),
                    ],
                )
            }
        })
        .collect();
    let mut world = World::grid(905, w, h, cols);
    world.add_occupant(1, 1, Occupant::plant_stub()).unwrap();
    world.add_occupant(CHUNK + 1, 1, Occupant::plant_stub()).unwrap();
    rest_all(&mut world);
    assert!(world.plant_at(1, 1));
    assert!(world.plant_at(CHUNK + 1, 1));

    let theta_b0 = world.layer_theta_at(CHUNK + 1, 1, 0);
    let mass_b0 = world.water_mass_at(CHUNK + 1, 1);

    world.ignore_chunk(0, 0).expect("A at rest");
    world.catch_up_chunk(0, 0, 5, 0.0);

    assert!(!world.plant_at(1, 1), "plant in A should wilt");
    assert!(world.plant_at(CHUNK + 1, 1), "plant in B stays alive");
    assert_approx_eq(
        world.layer_theta_at(CHUNK + 1, 1, 0),
        theta_b0,
        "B theta intact",
    );
    assert_approx_eq(world.water_mass_at(CHUNK + 1, 1), mass_b0, "B mass intact");
}

/// D9: rain catch-up on high chunk; adjacent valley chunk gains water via halo.
#[test]
fn d9_halo_can_export_pond() {
    let w = 16usize;
    let h = 8usize;
    let tex = Texture::Sand;
    let phi = tex.porosity();
    // Chunk A (x<8) high saturated; chunk B valley saturated (room for pond).
    let cols: Vec<Column> = (0..w * h)
        .map(|i| {
            let x = i % w;
            if x < CHUNK {
                Column::new(
                    2.0,
                    0.0,
                    vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)],
                )
            } else {
                Column::new(
                    0.0,
                    0.0,
                    vec![SoilLayer::new(0.3, phi, tex), SoilLayer::new(0.5, phi, tex)],
                )
            }
        })
        .collect();
    let mut world = World::grid(906, w, h, cols);
    rest_all(&mut world);

    let mut valley0 = 0.0;
    for y in 0..h {
        for x in CHUNK..w {
            valley0 += world.water_mass_at(x, y);
        }
    }

    world.catch_up_chunk(0, 0, 3, 0.10);

    let mut valley1 = 0.0;
    for y in 0..h {
        for x in CHUNK..w {
            valley1 += world.water_mass_at(x, y);
        }
    }
    assert!(
        valley1 > valley0 + MASS_EPSILON,
        "valley chunk should gain water via halo export: {valley0} -> {valley1}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 9.1 — catch_up calendar blocks
// ---------------------------------------------------------------------------

/// D91: plant + pond on highs; catch_up(K=10,R=0); plant alive; top not forced to 0 while bot full.
#[test]
fn d91_storm_then_catchup_plant_lives() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    // Equal-elevation "highs" with pond so water stays local (no valley steal).
    // Top starts moist, bot has room — percolation fills bot while calendar sinks
    // let the plant sip the top (settle-first would drain the top before any drink).
    let cols = vec![
        Column::new(
            2.0,
            0.30,
            vec![
                SoilLayer::new(0.3, phi, tex),
                SoilLayer::new(0.5, 0.05, tex),
            ],
        ),
        Column::new(
            2.0,
            0.30,
            vec![
                SoilLayer::new(0.3, phi, tex),
                SoilLayer::new(0.5, 0.05, tex),
            ],
        ),
    ];
    let mut world = World::grid(910, 2, 1, cols);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert!(world.plant_at(0, 0));

    let m0 = closed_mass(&world);
    world.catch_up(10, 0.0);

    assert!(
        world.plant_at(0, 0),
        "plant should survive calendar catch_up while water is still moving"
    );
    let theta_top = world.layer_theta_at(0, 0, 0);
    let theta_bot = world.layer_theta_at(0, 0, 1);
    // Bot fills from percolation; top must not be forced dry while bot is full.
    assert!(
        (theta_bot - phi).abs() <= 1e-3,
        "precondition of assertion: bot should be (near) full, θ_bot={theta_bot}"
    );
    assert!(
        theta_top > 1e-6,
        "top must not be forced to 0 while bot full (θ_top={theta_top}, θ_bot={theta_bot})"
    );
    assert_mass_close(closed_mass(&world), m0);
}

/// D91: same seed — 100 live ticks vs catch_up(K=10,R=0); θ,h,alive within 1e-6; extract within 20%.
#[test]
fn d91_matches_live_100() {
    let tex = Texture::Sand;
    let phi = tex.porosity();
    let mk = || {
        let cols = vec![
            Column::new(
                2.0,
                0.25,
                vec![
                    SoilLayer::new(0.3, 0.08, tex),
                    SoilLayer::new(0.5, 0.20, tex),
                ],
            ),
            Column::new(
                0.0,
                0.0,
                vec![
                    SoilLayer::new(0.3, phi * 0.5, tex),
                    SoilLayer::new(0.5, phi * 0.5, tex),
                ],
            ),
        ];
        let mut w = World::grid(911, 2, 1, cols);
        w.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
        w
    };

    let mut live = mk();
    let mut caught = mk();
    assert_eq!(live.tick_count(), caught.tick_count());

    for _ in 0..100 {
        live.tick();
    }
    caught.catch_up(10, 0.0);

    assert_eq!(live.tick_count(), caught.tick_count(), "calendar advances K*N_ET");
    let tol = 1e-6;
    assert!(
        (live.surface_water_m_at(0, 0) - caught.surface_water_m_at(0, 0)).abs() <= tol,
        "h high: live={} caught={}",
        live.surface_water_m_at(0, 0),
        caught.surface_water_m_at(0, 0)
    );
    assert!(
        (live.surface_water_m_at(1, 0) - caught.surface_water_m_at(1, 0)).abs() <= tol,
        "h valley: live={} caught={}",
        live.surface_water_m_at(1, 0),
        caught.surface_water_m_at(1, 0)
    );
    for layer in 0..2 {
        for x in 0..2 {
            let a = live.layer_theta_at(x, 0, layer);
            let b = caught.layer_theta_at(x, 0, layer);
            assert!(
                (a - b).abs() <= tol,
                "θ({x},0,{layer}): live={a} caught={b}"
            );
        }
    }
    assert_eq!(
        live.plant_at(0, 0),
        caught.plant_at(0, 0),
        "alive mismatch live={} caught={}",
        live.plant_at(0, 0),
        caught.plant_at(0, 0)
    );
    let le = live.extract_lost();
    let ce = caught.extract_lost();
    let scale = le.abs().max(ce.abs()).max(1e-9);
    assert!(
        (le - ce).abs() <= 0.20 * scale,
        "extract within 20%: live={le} caught={ce}"
    );
}

/// D91: at rest, R=0; catch_up(K) vs K live sinks; exact (inner hydro loop is 0).
#[test]
fn d91_rest_drought_still_exact() {
    let tex = Texture::Sand;
    let mk = || {
        let col = Column::new(
            0.0,
            0.06,
            vec![
                SoilLayer::new(0.3, tex.porosity(), tex),
                SoilLayer::new(0.5, tex.theta_fc(), tex),
            ],
        );
        let mut w = World::new(912, col);
        w.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
        // Drive to rest without burning many ET sinks.
        for _ in 0..5 {
            w.tick();
        }
        assert!(w.grid_at_rest(), "precondition: at rest");
        w
    };

    let k = 4u32;
    let mut live = mk();
    let mut caught = mk();
    assert_eq!(live.tick_count(), caught.tick_count());
    assert_approx_eq(live.grid_water_mass(), caught.grid_water_mass(), "pre mass");

    for _ in 0..k {
        tick_through_next_et(&mut live);
    }
    caught.catch_up(k, 0.0);

    assert_approx_eq(caught.surface_water_m(), live.surface_water_m(), "h");
    assert_approx_eq(
        caught.layer_theta_at(0, 0, 0),
        live.layer_theta_at(0, 0, 0),
        "θ_top",
    );
    assert_approx_eq(
        caught.layer_theta_at(0, 0, 1),
        live.layer_theta_at(0, 0, 1),
        "θ_bot",
    );
    assert_approx_eq(caught.et_lost(), live.et_lost(), "et");
    assert_approx_eq(caught.extract_lost(), live.extract_lost(), "extract");
    assert_eq!(live.plant_at(0, 0), caught.plant_at(0, 0), "alive");
}

// ---------------------------------------------------------------------------
// Sprint 10 — flyover catch_up budget (1024×1024)
// ---------------------------------------------------------------------------

use std::time::Instant;

const S10_W: usize = 1024;
const S10_H: usize = 1024;
const S10_OX: usize = 16;
const S10_OY: usize = 496;

/// Ridge z=8 on x∈[128,144), else z=2; Loam at θ_fc; plant in starting view; asleep+ready.
fn s10_flyover_world(seed: u64) -> World {
    let tex = Texture::Loam;
    let fc = tex.theta_fc();
    let mut cols = Vec::with_capacity(S10_W * S10_H);
    for y in 0..S10_H {
        for x in 0..S10_W {
            let z = if (128..144).contains(&x) { 8.0 } else { 2.0 };
            let _ = y;
            cols.push(Column::new(
                z,
                0.0,
                vec![
                    SoilLayer::new(0.3, fc, tex),
                    SoilLayer::new(0.5, fc, tex),
                ],
            ));
        }
    }
    let mut world = World::grid(seed, S10_W, S10_H, cols);
    // Rest desert: no hydro activity until observe/rain/catch_up.
    world.force_sleep_all();
    // PlantStub in starting view [ox,ox+32)×[oy,oy+32).
    world
        .add_occupant(S10_OX + 8, S10_OY + 8, Occupant::plant_stub())
        .expect("plant");
    // add_occupant wakes; return to sleep for budget tests that ignore explicitly.
    world.force_sleep_all();
    world
}

fn s10_ignore_all(world: &mut World) {
    let ncx = (world.width() + CHUNK - 1) / CHUNK;
    let ncy = (world.height() + CHUNK - 1) / CHUNK;
    for cy in 0..ncy {
        for cx in 0..ncx {
            world.ignore_chunk(cx, cy).expect("ignore");
        }
    }
}

fn s10_work_set_chunks() -> Vec<(usize, usize)> {
    // view ∪ look-ahead = [ox, ox+64) × [oy, oy+32)
    let x0 = S10_OX;
    let x1 = S10_OX + 64;
    let y0 = S10_OY;
    let y1 = S10_OY + 32;
    let mut chunks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let cx = x / CHUNK;
            let cy = y / CHUNK;
            if seen.insert((cx, cy)) {
                chunks.push((cx, cy));
            }
        }
    }
    chunks
}

fn s10_stripe_chunks() -> Vec<(usize, usize)> {
    // Incoming stripe: first 8 m of look-ahead = [ox+32, ox+40) × [oy, oy+32)
    let x0 = S10_OX + 32;
    let x1 = S10_OX + 40;
    let y0 = S10_OY;
    let y1 = S10_OY + 32;
    let mut chunks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let cx = x / CHUNK;
            let cy = y / CHUNK;
            if seen.insert((cx, cy)) {
                chunks.push((cx, cy));
            }
        }
    }
    chunks
}

fn s10_work_set_cell_count_with_halo(world: &World, chunks: &[(usize, usize)]) -> usize {
    let mut cells = std::collections::HashSet::new();
    for &(cx, cy) in chunks {
        let x0 = cx * CHUNK;
        let y0 = cy * CHUNK;
        let x1 = ((cx + 1) * CHUNK).min(world.width());
        let y1 = ((cy + 1) * CHUNK).min(world.height());
        for y in y0..y1 {
            for x in x0..x1 {
                cells.insert((x, y));
                for (dx, dy) in [(0i32, 1), (1, 0), (0, -1), (-1, 0)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let ux = nx as usize;
                    let uy = ny as usize;
                    if ux < world.width() && uy < world.height() {
                        cells.insert((ux, uy));
                    }
                }
            }
        }
    }
    cells.len()
}

/// D10: World 1024×1024 constructs; print cell count (and RSS if easy).
#[test]
fn d10_map_constructs() {
    let t0 = Instant::now();
    let world = match std::panic::catch_unwind(|| s10_flyover_world(1001)) {
        Ok(w) => w,
        Err(e) => panic!("d10_map_constructs: alloc/construct failed: {e:?}"),
    };
    let cells = world.width() * world.height();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "d10_map_constructs: cells={cells} width={} height={} construct_ms={ms:.2}",
        world.width(),
        world.height()
    );
    // Best-effort RSS (Linux).
    if let Ok(statm) = std::fs::read_to_string("/proc/self/status") {
        for line in statm.lines() {
            if line.starts_with("VmRSS:") {
                eprintln!("d10_map_constructs: {line}");
                break;
            }
        }
    }
    assert_eq!(cells, S10_W * S10_H);
    assert_eq!(world.occupant_count(S10_OX + 8, S10_OY + 8), 1);
}

/// D10: all ignored + rest; 1 tick; visits==0; wall < 5 ms.
#[test]
fn d10_ignored_tick_zero_visits() {
    let mut world = s10_flyover_world(1002);
    s10_ignore_all(&mut world);
    assert_eq!(world.observed_chunk_count(), 0);
    assert_eq!(world.awake_count(), 0);

    let t0 = Instant::now();
    world.tick();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("d10_ignored_tick_zero_visits: {ms:.3} ms");
    assert_eq!(world.last_hydro_visits(), 0);
    assert!(
        ms < 5.0,
        "ignored tick wall {ms:.3} ms exceeds 5 ms budget"
    );
}

/// D10: observe work set only; rain in view; visits ≤ work-set+halo, not 1024².
#[test]
fn d10_frustum_not_world() {
    let mut world = s10_flyover_world(1003);
    s10_ignore_all(&mut world);
    let work = s10_work_set_chunks();
    eprintln!("d10_frustum_not_world: work_set_chunks={}", work.len());
    // 64×32 cells → 8×4 chunk tiles = 32 (sprint "~8" was underspecified).
    assert_eq!(work.len(), 32, "expect 32 chunks for 64×32 view∪look-ahead");
    for &(cx, cy) in &work {
        world.observe_chunk(cx, cy);
    }
    let cap = s10_work_set_cell_count_with_halo(&world, &work);
    world.add_rain_at(S10_OX + 4, S10_OY + 4, 0.05);

    let t0 = Instant::now();
    world.tick();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let visits = world.last_hydro_visits();
    eprintln!("d10_frustum_not_world: visits={visits} cap={cap} {ms:.3} ms");
    assert!(
        visits <= cap,
        "visits {visits} exceed work-set+halo {cap}"
    );
    assert!(
        visits < S10_W * S10_H / 4,
        "visits {visits} look like whole-world hydro"
    );
}

/// D10: catch_up_chunk each of 4 stripe chunks, K=30, R=0; wall < 50 ms.
#[test]
fn d10_stripe_k30() {
    let mut world = s10_flyover_world(1004);
    s10_ignore_all(&mut world);
    let stripe = s10_stripe_chunks();
    eprintln!("d10_stripe_k30: stripe_chunks={} {:?}", stripe.len(), stripe);
    assert_eq!(stripe.len(), 4, "incoming stripe must be 4 chunks");

    let t0 = Instant::now();
    for &(cx, cy) in &stripe {
        world.catch_up_chunk(cx, cy, 30, 0.0);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("d10_stripe_k30: {ms:.3} ms");
    assert!(ms < 50.0, "stripe K=30 wall {ms:.3} ms exceeds 50 ms");
}

/// D10: catch_up all work-set chunks, K=100, R=0; wall < 500 ms.
#[test]
fn d10_workset_k100() {
    let mut world = s10_flyover_world(1005);
    s10_ignore_all(&mut world);
    let work = s10_work_set_chunks();
    assert_eq!(work.len(), 32);

    let t0 = Instant::now();
    world.catch_up_chunks(&work, 100, 0.0);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("d10_workset_k100: {ms:.3} ms");
    assert!(ms < 500.0, "workset K=100 wall {ms:.3} ms exceeds 500 ms");
}

/// D10: 0.3 m pond on ridge cells in look-ahead; catch_up those chunks K=30; wall < 500 ms.
#[test]
fn d10_storm_k30_still_under_fly() {
    let mut world = s10_flyover_world(1006);
    s10_ignore_all(&mut world);

    // Spec: 0.3 m pond on ridge cells in look-ahead. ox=16 look-ahead x∈[48,80)
    // misses ridge x∈[128,144). Stress the incoming stripe (32×8) with 0.3 m pond
    // (and ridge z where stripe overlaps elevation jump is N/A); catch_up those
    // 4 chunks at K=30 under the fly budget.
    let chunks = s10_stripe_chunks();
    assert_eq!(chunks.len(), 4);
    for y in S10_OY..(S10_OY + 32) {
        for x in (S10_OX + 32)..(S10_OX + 40) {
            // Direct write + later catch_up wakes wet cells (avoid waking the world).
            let i = y * world.width() + x;
            let _ = i;
            let col = world.column_at_mut(x, y);
            col.surface_water_m += 0.3;
        }
    }
    // Clear the accidental wakes from column_at_mut? set_column wakes — we used
    // column_at_mut which does NOT wake. Good. Ensure asleep before timed catch_up.
    world.force_sleep_all();
    eprintln!("d10_storm_k30_still_under_fly: chunks={}", chunks.len());

    let t0 = Instant::now();
    world.catch_up_chunks(&chunks, 30, 0.0);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("d10_storm_k30_still_under_fly: {ms:.3} ms");
    assert!(ms < 500.0, "storm K=30 wall {ms:.3} ms exceeds 500 ms");
}


// --- Sprint 11 — kernel catalog -------------------------------------------------

/// D11: embedded CSV loads; at least ten demo=1 taxa (fixture has exactly 10).
#[test]
fn d11_catalog_loads_demo_ten() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    assert!(
        cat.demo_count() >= 10,
        "expected ≥10 demo taxa, got {}",
        cat.demo_count()
    );
    assert_eq!(
        cat.demo_count(),
        10,
        "demo fixture should be exactly 10 rows"
    );
    assert_eq!(cat.len(), 10);
}

/// D11: unknown taxon_id is Err.
#[test]
fn d11_unknown_taxon_is_err() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    assert!(cat.get("not_a_taxon").is_err());
}

/// D11: rice (oryza_sativa) empty root_depth → herb form default → [1,0].
#[test]
fn d11_rice_root_null_uses_herb_mask() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let rice = cat.get("oryza_sativa").expect("oryza_sativa");
    assert!(rice.root_depth_m.is_none());
    assert_eq!(rice.root_mask, [1.0, 0.0]);
    // Helper builds PlantStub with catalog mask; uptake stays P_MAX.
    let stub = rice.to_plant_stub();
    assert_eq!(stub.root, [1.0, 0.0]);
    assert!((stub.uptake_max - P_MAX).abs() < MASS_EPSILON);
}

/// D11: oak (quercus_alba) deeper root → [1,1], deeper than rice default.
#[test]
fn d11_oak_deeper_than_rice_default_mask() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let oak = cat.get("quercus_alba").expect("quercus_alba");
    let rice = cat.get("oryza_sativa").expect("oryza_sativa");
    assert!(oak.root_depth_m.is_some());
    assert!(oak.root_depth_m.unwrap() > 0.20);
    assert_eq!(oak.root_mask, [1.0, 1.0]);
    assert_eq!(rice.root_mask, [1.0, 0.0]);
    // Oak reaches bot layer; rice default does not.
    assert!(oak.root_mask[1] > rice.root_mask[1]);
}

/// D11: kernel resolves taxa by taxon_id only — no species-name string branching.
#[test]
fn d11_no_species_name_match_in_sim_core() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    // API-only: oak and rice resolve via id, not display name.
    let oak = cat.get("quercus_alba").expect("id quercus_alba");
    let rice = cat.get("oryza_sativa").expect("id oryza_sativa");
    assert_ne!(oak.taxon_id, rice.taxon_id);
    assert_eq!(oak.name_norm, "quercus alba");
    assert_eq!(rice.name_norm, "oryza sativa");
    // Lookup by common English name must fail (no name matching in kernel).
    assert!(cat.get("oak").is_err());
    assert!(cat.get("rice").is_err());
    assert!(cat.get("quercus alba").is_err());

    // Source scan: production sim-core must not branch physics on species name strings.
    let lib = include_str!("../src/lib.rs");
    let catalog = include_str!("../src/catalog.rs");
    for (label, src) in [("lib.rs", lib), ("catalog.rs", catalog)] {
        for pat in [
            r#"name == "oak""#,
            r#"name == "rice""#,
            r#"== "oak""#,
            r#"== "Rice""#,
            "if name ==",
            "match name",
        ] {
            assert!(
                !src.contains(pat),
                "{label} contains forbidden species-name pattern: {pat}"
            );
        }
    }
    let _params: &OccupantParams = oak;
}

/// D11: two taxa expose queryable rain_min/max envelopes (no filter applied).
#[test]
fn d11_same_rain_two_taxa_queryable() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let a = cat.get("oryza_sativa").expect("rice");
    let b = cat.get("quercus_alba").expect("oak");
    let a_min = a.rain_min_mm.expect("rice rain_min");
    let a_max = a.rain_max_mm.expect("rice rain_max");
    let b_min = b.rain_min_mm.expect("oak rain_min");
    let b_max = b.rain_max_mm.expect("oak rain_max");
    assert!(a_min < a_max);
    assert!(b_min < b_max);
    // Overlapping mid-range queryable on both (e.g. 900 mm).
    let q = 900.0;
    assert!(a_min <= q && q <= a_max, "rice envelope should cover {q}");
    assert!(b_min <= q && q <= b_max, "oak envelope should cover {q}");
    // Envelopes differ (not a single hardcoded rain table).
    assert!(
        (a_min, a_max) != (b_min, b_max),
        "two taxa should carry distinct rain envelopes"
    );
}


// --- Sprint 12 — climate envelope -------------------------------------------------

fn climate_sand_column() -> Column {
    let tex = Texture::Sand;
    let fc = tex.theta_fc();
    Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, fc, tex),
            SoilLayer::new(0.5, fc, tex),
        ],
    )
}

/// D12: rice dies / fails to plant when T_air < catalog t_min (16°C).
#[test]
fn d12_rice_dies_below_tmin() {
    let mut world = World::new(1201, climate_sand_column());
    world.set_climate(10.0, 1000.0); // below rice t_min=16; rain OK
    let err = world.plant_taxon(0, 0, "oryza_sativa");
    assert!(
        matches!(err, Err(PlantTaxonError::Climate(ClimateReject::TMin))),
        "expected TMin reject, got {err:?}"
    );
    assert!(!world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::TMin);
    // Tick path: plant under good climate then chill.
    world.set_climate(25.0, 1000.0);
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));
    world.set_climate(10.0, 1000.0);
    world.tick();
    assert!(!world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::TMin);
}

/// D12: wheat lives at the same cold T that kills rice.
#[test]
fn d12_wheat_lives_at_same_t() {
    let mut world = World::new(1202, climate_sand_column());
    world.set_climate(10.0, 1000.0); // rice dies; wheat t_min=5
    world.plant_taxon(0, 0, "triticum_aestivum").unwrap();
    assert!(world.plant_at(0, 0));
    world.tick();
    assert!(world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
}

/// D12: rice lives inside its T/rain envelope.
#[test]
fn d12_rice_lives_in_envelope() {
    let mut world = World::new(1203, climate_sand_column());
    world.set_climate(25.0, 1200.0); // inside 16–38 / 800–4000
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));
    world.tick();
    assert!(world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
}

/// D12: wheat dies when T_air > catalog t_max (27°C).
#[test]
fn d12_wheat_dies_above_tmax() {
    let mut world = World::new(1204, climate_sand_column());
    world.set_climate(30.0, 1000.0); // above wheat t_max=27; rain OK
    let err = world.plant_taxon(0, 0, "triticum_aestivum");
    assert!(
        matches!(err, Err(PlantTaxonError::Climate(ClimateReject::TMax))),
        "expected TMax reject, got {err:?}"
    );
    assert!(!world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::TMax);
}

/// D12/S12.1: dry rain_year is not a live kill; rice remains at T=22 / 200 mm.
#[test]
fn d12_rice_dies_dry_year() {
    let mut world = World::new(1205, climate_sand_column());
    world.set_climate(22.0, 200.0); // warm; dry year
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0), "live plant must ignore rain_year");
    world.tick();
    assert!(
        world.plant_at(0, 0),
        "live tick must not kill on dry rain_year when T in envelope"
    );
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
}

/// D12/S12.1: 0.05 m rain must not change rain_year_mm; rice remains if T in envelope.
#[test]
fn d12_bucket_is_not_climate() {
    let mut world = World::new(1206, climate_sand_column());
    world.set_climate(25.0, 500.0); // dry year — rain_year still orthogonal to bucket
    assert!((world.rain_year_mm() - 500.0).abs() < 1e-12);
    world.add_rain(0.05);
    assert!(
        (world.rain_year_mm() - 500.0).abs() < 1e-12,
        "0.05 m add_rain must not flip rain_year_mm"
    );
    // Live path: T in envelope → rice plants; rain_year is not a live kill.
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));
    world.tick();
    assert!(world.plant_at(0, 0), "rice remains when T in envelope");
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
    assert!((world.rain_year_mm() - 500.0).abs() < 1e-12);
}

/// D12: empty envelope / PlantStub without taxon — no climate veto at extreme set_climate.
#[test]
fn d12_empty_envelope_no_veto() {
    let mut world = World::new(1207, climate_sand_column());
    world.set_climate(-40.0, 0.0);
    world.add_occupant(0, 0, Occupant::plant_stub()).unwrap();
    assert!(world.plant_at(0, 0));
    world.tick();
    assert!(world.plant_at(0, 0), "taxon-less PlantStub must survive extreme climate");
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
}

/// D12: filter uses catalog ids/fields — no species-name branching in the kernel.
#[test]
fn d12_no_species_name_match() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let rice = cat.get("oryza_sativa").expect("oryza_sativa");
    let wheat = cat.get("triticum_aestivum").expect("triticum_aestivum");
    assert_ne!(rice.taxon_id, wheat.taxon_id);
    // Common English names are not taxon keys.
    assert!(cat.get("rice").is_err());
    assert!(cat.get("wheat").is_err());

    // API: envelopes differ via catalog fields, not hardcoded name tables.
    assert_eq!(rice.t_min_c, Some(16.0));
    assert_eq!(wheat.t_min_c, Some(5.0));
    assert_ne!(rice.t_max_c, wheat.t_max_c);

    let mut world = World::new(1208, climate_sand_column());
    world.set_climate(10.0, 1000.0);
    assert!(matches!(
        world.plant_taxon(0, 0, "oryza_sativa"),
        Err(PlantTaxonError::Climate(ClimateReject::TMin))
    ));
    world.plant_taxon(0, 0, "triticum_aestivum").unwrap();
    assert!(world.plant_at(0, 0));

    let lib = include_str!("../src/lib.rs");
    let catalog = include_str!("../src/catalog.rs");
    for (label, src) in [("lib.rs", lib), ("catalog.rs", catalog)] {
        for pat in [
            r#"== "rice""#,
            r#"== "wheat""#,
            r#"name == "rice""#,
            r#"name == "wheat""#,
            "if name ==",
            "match name",
        ] {
            assert!(
                !src.contains(pat),
                "{label} contains forbidden species-name pattern: {pat}"
            );
        }
    }
    let _params: &OccupantParams = rice;
}


// --- Sprint 12.1 — event hydro / current conditions vs year envelope -------------

fn hydro_pond_column(h_surf: f64) -> Column {
    // Saturated soil so pond stays on the surface (no infiltrate room).
    saturated_column(0.0, h_surf)
}

fn hydro_dry_column() -> Column {
    let tex = Texture::Sand;
    Column::new(
        0.0,
        0.0,
        vec![
            SoilLayer::new(0.3, 0.0, tex),
            SoilLayer::new(0.5, 0.0, tex),
        ],
    )
}

/// D121: plant rice under dry rain_year when air is warm (live T-only gate).
#[test]
fn d121_plant_rice_in_dry_year_if_warm() {
    let mut world = World::new(12101, climate_sand_column());
    world.set_climate(22.0, 200.0); // below rice rain_min=800
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
}

/// D121: rice planted warm then air turns cold → climate TMin remove.
#[test]
fn d121_rice_dies_when_air_turns_cold() {
    let mut world = World::new(12102, climate_sand_column());
    world.set_climate(22.0, 200.0);
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));
    world.set_climate(10.0, 200.0); // below rice t_min=16
    world.tick();
    assert!(!world.plant_at(0, 0));
    assert_eq!(world.climate_reject(0, 0), ClimateReject::TMin);
}

/// D121: wheat (drain_ok=MD) waterlogs in pond after T_WL live ticks.
#[test]
fn d121_wheat_waterlogs_in_pond() {
    let mut world = World::new(12103, hydro_pond_column(0.05));
    world.set_climate(20.0, 1000.0); // inside wheat T 5–27
    world.plant_taxon(0, 0, "triticum_aestivum").unwrap();
    assert!(world.plant_at(0, 0));
    assert!(world.surface_water_m() >= H_POND);

    for t in 1..=T_WL {
        if world.surface_water_m() < H_POND {
            world.add_rain(H_POND - world.surface_water_m() + 0.01);
        }
        world.tick();
        if t < T_WL {
            assert!(
                world.plant_at(0, 0),
                "wheat should still be alive after {t} waterlog ticks (T_WL={T_WL})"
            );
            assert_eq!(world.hydro_reject(0, 0), HydroReject::None);
        }
    }
    assert!(!world.plant_at(0, 0), "wheat should waterlog after T_WL={T_WL}");
    assert_eq!(world.hydro_reject(0, 0), HydroReject::Waterlog);
}

/// D121: rice (drain_ok=W) survives pond — no waterlog kill.
#[test]
fn d121_rice_pond_no_waterlog() {
    let mut world = World::new(12104, hydro_pond_column(0.05));
    world.set_climate(22.0, 1000.0);
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    for _ in 0..(T_WL + 3) {
        if world.surface_water_m() < H_POND {
            world.add_rain(H_POND - world.surface_water_m() + 0.01);
        }
        world.tick();
    }
    assert!(world.plant_at(0, 0), "rice drain_ok=W must not waterlog in pond");
    assert_eq!(world.hydro_reject(0, 0), HydroReject::None);
}

/// D121: wheat height 0.8 m — pond over height for T_SUB → Submerged.
#[test]
fn d121_wheat_submerged_over_height() {
    let wheat_h = 0.8;
    let mut world = World::new(12105, hydro_pond_column(wheat_h + 0.10));
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "triticum_aestivum").unwrap();
    assert!(world.surface_water_m() >= wheat_h);

    for t in 1..=T_SUB {
        if world.surface_water_m() < wheat_h {
            world.add_rain(wheat_h - world.surface_water_m() + 0.10);
        }
        world.tick();
        if t < T_SUB {
            assert!(
                world.plant_at(0, 0),
                "wheat should survive {t} submerge ticks (T_SUB={T_SUB})"
            );
        }
    }
    assert!(!world.plant_at(0, 0), "wheat submerged after T_SUB");
    assert_eq!(world.hydro_reject(0, 0), HydroReject::Submerged);
}

/// D121: oak height 20.1 — shallow pond is not a submerge kill.
#[test]
fn d121_oak_not_submerged_in_shallow_pond() {
    let mut world = World::new(12106, hydro_pond_column(0.05));
    world.set_climate(20.0, 1000.0); // oak t_min=7 t_max=32
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    let oak_h = world
        .catalog()
        .get("quercus_alba")
        .unwrap()
        .height_m_mature
        .unwrap();
    assert!((oak_h - 20.1).abs() < 1e-9);
    assert!(0.05 < oak_h);

    for _ in 0..T_SUB {
        if world.surface_water_m() < H_POND {
            world.add_rain(0.05);
        }
        world.tick();
        // Never a Submerged reject (oak is tall). May waterlog at T_WL==T_SUB.
        assert_ne!(
            world.hydro_reject(0, 0),
            HydroReject::Submerged,
            "shallow pond must not report Submerged for tall oak"
        );
    }
}

/// D121: drought wilt with good rain_year — rain_year is not the drought axis.
#[test]
fn d121_drought_kills_at_good_rain_year() {
    let mut world = World::new(12107, hydro_dry_column());
    world.set_climate(22.0, 1200.0); // good rain_year; dry soil
    world.plant_taxon(0, 0, "oryza_sativa").unwrap();
    assert!(world.plant_at(0, 0));

    for step in 1..=T_WILT {
        tick_through_next_et(&mut world);
        if step < T_WILT {
            assert!(
                world.plant_at(0, 0),
                "alive after {step} dry sinks (T_WILT={T_WILT})"
            );
        }
    }
    assert!(
        !world.plant_at(0, 0),
        "plant should wilt after T_WILT dry sinks despite good rain_year"
    );
    // Wilt leaves occupant (alive=false); not a climate/hydro remove.
    assert_eq!(world.climate_reject(0, 0), ClimateReject::None);
    assert_eq!(world.hydro_reject(0, 0), HydroReject::None);
    assert!(world.occupant_count(0, 0) >= 1, "wilt keeps occupant slot");
}

/// D121: hydro uses catalog drain_ok + height — no species-name branching.
#[test]
fn d121_no_species_name_match() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let rice = cat.get("oryza_sativa").expect("oryza_sativa");
    let wheat = cat.get("triticum_aestivum").expect("triticum_aestivum");
    let oak = cat.get("quercus_alba").expect("quercus_alba");

    assert_eq!(rice.drain_ok.as_deref(), Some("W"));
    assert_eq!(wheat.drain_ok.as_deref(), Some("MD"));
    assert_eq!(oak.height_m_mature, Some(20.1));
    assert_eq!(wheat.height_m_mature, Some(0.8));

    assert!(cat.get("rice").is_err());
    assert!(cat.get("wheat").is_err());
    assert!(cat.get("oak").is_err());

    let lib = include_str!("../src/lib.rs");
    let catalog = include_str!("../src/catalog.rs");
    for (label, src) in [("lib.rs", lib), ("catalog.rs", catalog)] {
        for pat in [
            r#"== "rice""#,
            r#"== "wheat""#,
            r#"== "oak""#,
            r#"name == "rice""#,
            r#"name == "wheat""#,
            "if name ==",
            "match name",
        ] {
            assert!(
                !src.contains(pat),
                "{label} contains forbidden species-name pattern: {pat}"
            );
        }
    }
    let _params: &OccupantParams = rice;
}

// --- Sprint 13 — height-layered light -------------------------------------------------

fn light_sand_column() -> Column {
    climate_sand_column()
}

fn alpha_sla(s: f64) -> f64 {
    (0.55 - 0.01 * (s - 20.0)).clamp(0.15, 0.85)
}

/// D13: grass alone receives L0=1 and survives well past T_DARK.
#[test]
fn d13_grass_alone_full_light() {
    let mut world = World::new(1301, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    assert!(world.plant_at(0, 0));

    for _ in 0..(T_DARK + 3) {
        world.tick();
    }
    assert!(world.plant_at(0, 0), "grass alone must not dark-kill");
    assert_eq!(world.light_reject(0, 0), LightReject::None);
    let occ = &world.column_at(0, 0).occupants[0];
    assert_eq!(occ.taxon_id.as_deref(), Some("lolium_perenne"));
    let light = occ.last_light.expect("light pass sets last_light");
    assert!(
        (light - L0).abs() < 1e-12,
        "grass alone must see full L0, got {light}"
    );
    assert_eq!(occ.dark_steps, 0);
}

/// D13: oak over grass reduces grass last_light below L0; oak keeps L0.
#[test]
fn d13_oak_over_grass_shades() {
    let mut world = World::new(1302, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    world.plant_taxon(0, 0, "quercus_alba").unwrap();

    world.tick();
    let col = world.column_at(0, 0);
    assert_eq!(col.occupants.len(), 2);
    let mut grass_l = None;
    let mut oak_l = None;
    let mut oak_alpha = None;
    for o in &col.occupants {
        match o.taxon_id.as_deref() {
            Some("lolium_perenne") => grass_l = o.last_light,
            Some("quercus_alba") => {
                oak_l = o.last_light;
                oak_alpha = Some(alpha_for_occupant(world.catalog(), o));
            }
            _ => {}
        }
    }
    let grass_l = grass_l.expect("grass light");
    let oak_l = oak_l.expect("oak light");
    let oak_alpha = oak_alpha.unwrap();
    assert!((oak_l - L0).abs() < 1e-12, "oak tallest sees L0, got {oak_l}");
    let expect = L0 * (1.0 - oak_alpha);
    assert!(
        (grass_l - expect).abs() < 1e-9,
        "grass last_light={grass_l} expect {expect} (oak alpha={oak_alpha})"
    );
    assert!(grass_l < L0 - 1e-9, "oak must shade grass");
    assert!(world.plant_at(0, 0));
    assert_eq!(world.light_reject(0, 0), LightReject::None);
}

/// D13: deep oak shade (two oaks, same band) drives grass below herb L_min → Dark remove.
#[test]
fn d13_grass_dies_in_oak_shade() {
    let mut world = World::new(1303, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    // Two oaks same band: both see L0; grass gets L0*(1-min(1,2*alpha)).
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    world.plant_taxon(0, 0, "quercus_alba").unwrap();

    let oak = world.catalog().get("quercus_alba").unwrap();
    let a = alpha_from_params(oak);
    let l_grass = L0 * (1.0 - (2.0 * a).min(1.0));
    assert!(
        l_grass < 0.25,
        "precondition: two-oak band shade {l_grass} must be < herb L_min"
    );

    world.tick();
    let col = world.column_at(0, 0);
    let mut oak_lights = Vec::new();
    let mut grass_l = None;
    for o in &col.occupants {
        match o.taxon_id.as_deref() {
            Some("quercus_alba") => oak_lights.push(o.last_light.expect("oak light")),
            Some("lolium_perenne") => grass_l = o.last_light,
            _ => {}
        }
    }
    assert_eq!(oak_lights.len(), 2);
    for ol in &oak_lights {
        assert!((ol - L0).abs() < 1e-12, "same-band oaks both see L0, got {ol}");
    }
    let grass_l = grass_l.expect("grass light");
    assert!(
        (grass_l - l_grass).abs() < 1e-9,
        "grass last_light={grass_l} expect band transmit {l_grass}"
    );

    for t in 1..=T_DARK {
        if t > 1 {
            world.tick();
        }
        if t < T_DARK {
            assert!(
                world
                    .column_at(0, 0)
                    .occupants
                    .iter()
                    .any(|o| o.taxon_id.as_deref() == Some("lolium_perenne")),
                "grass should remain through tick {t}/{T_DARK}"
            );
            assert_eq!(world.light_reject(0, 0), LightReject::None);
        }
    }
    assert!(
        world
            .column_at(0, 0)
            .occupants
            .iter()
            .all(|o| o.taxon_id.as_deref() != Some("lolium_perenne")),
        "grass must be removed after T_DARK under deep oak shade"
    );
    assert_eq!(world.light_reject(0, 0), LightReject::Dark);
    // Oaks remain (tree L_min is low); both still same band at L0.
    assert!(
        world
            .column_at(0, 0)
            .occupants
            .iter()
            .filter(|o| o.taxon_id.as_deref() == Some("quercus_alba"))
            .count()
            >= 1,
        "oaks should survive their own canopy light"
    );
}

/// D13: remove oak canopy → grass stays lit and lives.
#[test]
fn d13_remove_oak_grass_lives() {
    let mut world = World::new(1304, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    // One tick records shade, then clear oaks only.
    world.tick();
    let grass = world
        .column_at(0, 0)
        .occupants
        .iter()
        .find(|o| o.taxon_id.as_deref() == Some("lolium_perenne"))
        .unwrap()
        .clone();
    world.clear_occupants(0, 0);
    world.add_occupant(0, 0, grass).unwrap();

    for _ in 0..(T_DARK + 3) {
        world.tick();
    }
    assert!(
        world
            .column_at(0, 0)
            .occupants
            .iter()
            .any(|o| o.alive && o.taxon_id.as_deref() == Some("lolium_perenne")),
        "grass lives once oak canopy is removed"
    );
    assert_eq!(world.light_reject(0, 0), LightReject::None);
    let light = world.column_at(0, 0).occupants[0]
        .last_light
        .expect("last_light");
    assert!((light - L0).abs() < 1e-12);
}

/// D13: two ryegrass same band — both receive L0; neither shades the other before receive.
#[test]
fn d13_two_herbs_neither_dark() {
    let mut world = World::new(1305, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();

    world.tick();
    assert_eq!(world.occupant_count(0, 0), 2);
    for o in &world.column_at(0, 0).occupants {
        assert_eq!(o.taxon_id.as_deref(), Some("lolium_perenne"));
        let light = o.last_light.expect("last_light");
        assert!(
            (light - L0).abs() < 1e-12,
            "same-band ryegrass must both see L0, got {light}"
        );
        assert_eq!(o.dark_steps, 0);
    }

    for _ in 0..(T_DARK + 2) {
        world.tick();
    }
    assert_eq!(world.occupant_count(0, 0), 2);
    assert!(world
        .column_at(0, 0)
        .occupants
        .iter()
        .all(|o| o.alive && o.dark_steps == 0));
    assert_eq!(world.light_reject(0, 0), LightReject::None);
}

/// D13: ET uses alpha_for (SLA/form), not the shade field alone.
#[test]
fn d13_et_uses_alpha() {
    let h0 = 0.10;
    let mut world = World::new(1306, saturated_column(0.0, h0));
    world.set_climate(20.0, 1000.0);
    // Reach the tick before the first ET without a taxon plant (pond waterlogs
    // MD herbs after T_WL live ticks). Plant just in time for the ET step.
    while world.tick_count() + 1 < N_ET {
        world.tick();
    }
    world.plant_taxon(0, 0, "lolium_perenne").unwrap();
    let grass = world.catalog().get("lolium_perenne").unwrap();
    let a = alpha_from_params(grass);
    assert!((a - alpha_sla(grass.sla_m2_kg.unwrap())).abs() < 1e-12);
    assert!((world.column_at(0, 0).occupants[0].shade - 0.25).abs() < 1e-12);
    assert!((a - 0.25).abs() > 1e-3, "SLA alpha should differ from shade field");

    let h_before = world.surface_water_m();
    world.tick(); // N_ET: ET then filters
    let h1 = world.surface_water_m();
    let expected = E_OPEN * (1.0 - a.clamp(0.0, 1.0));
    assert_approx_eq(h_before - h1, expected, "pond ET scaled by SLA alpha");
    assert_approx_eq(world.et_lost(), expected, "et_lost matches alpha shade");
}

/// D13: light uses form/SLA/height ids — no species-name branching.
#[test]
fn d13_no_species_name_match() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let grass = cat.get("lolium_perenne").expect("lolium_perenne");
    let oak = cat.get("quercus_alba").expect("quercus_alba");
    assert_eq!(grass.form, sim_core::Form::Herb);
    assert_eq!(oak.form, sim_core::Form::Tree);
    assert!(grass.sla_m2_kg.is_some());
    assert_eq!(oak.height_m_mature, Some(20.1));
    assert!(cat.get("grass").is_err());
    assert!(cat.get("oak").is_err());

    let lib = include_str!("../src/lib.rs");
    let catalog = include_str!("../src/catalog.rs");
    for (label, src) in [("lib.rs", lib), ("catalog.rs", catalog)] {
        for pat in [
            r#"== "grass""#,
            r#"== "oak""#,
            r#"== "lolium""#,
            r#"name == "oak""#,
            "if name ==",
            "match name",
        ] {
            assert!(
                !src.contains(pat),
                "{label} contains forbidden species-name pattern: {pat}"
            );
        }
    }
    let _a = alpha_from_params(grass);
    let _b = alpha_from_params(oak);
}

/// D131: two oaks same height band both receive L0.
#[test]
fn d131_two_oaks_same_light() {
    let mut world = World::new(1311, light_sand_column());
    world.set_climate(20.0, 1000.0);
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    world.plant_taxon(0, 0, "quercus_alba").unwrap();
    assert!((H_BAND - 0.05).abs() < 1e-15);

    world.tick();
    let lights: Vec<f64> = world
        .column_at(0, 0)
        .occupants
        .iter()
        .map(|o| o.last_light.expect("light"))
        .collect();
    assert_eq!(lights.len(), 2);
    for l in &lights {
        assert!((l - L0).abs() < 1e-12, "same-band oaks both L0, got {l}");
    }
    assert!((lights[0] - lights[1]).abs() < 1e-15);
}

/// D131: two oaks shade grass darker than one oak (band sum alpha).
#[test]
fn d131_two_oaks_darker_grass_than_one() {
    let oak = Catalog::load_embedded()
        .unwrap()
        .get("quercus_alba")
        .unwrap()
        .clone();
    let a = alpha_from_params(&oak);
    let one = L0 * (1.0 - a.min(1.0));
    let two = L0 * (1.0 - (2.0 * a).min(1.0));
    assert!(two < one - 1e-9, "precondition two-oak band darker than one");

    let mut one_oak = World::new(1312, light_sand_column());
    one_oak.set_climate(20.0, 1000.0);
    one_oak.plant_taxon(0, 0, "lolium_perenne").unwrap();
    one_oak.plant_taxon(0, 0, "quercus_alba").unwrap();
    one_oak.tick();
    let grass_one = one_oak
        .column_at(0, 0)
        .occupants
        .iter()
        .find(|o| o.taxon_id.as_deref() == Some("lolium_perenne"))
        .unwrap()
        .last_light
        .unwrap();
    assert!((grass_one - one).abs() < 1e-9, "one-oak grass {grass_one} vs {one}");

    let mut two_oaks = World::new(1313, light_sand_column());
    two_oaks.set_climate(20.0, 1000.0);
    two_oaks.plant_taxon(0, 0, "lolium_perenne").unwrap();
    two_oaks.plant_taxon(0, 0, "quercus_alba").unwrap();
    two_oaks.plant_taxon(0, 0, "quercus_alba").unwrap();
    two_oaks.tick();
    let grass_two = two_oaks
        .column_at(0, 0)
        .occupants
        .iter()
        .find(|o| o.taxon_id.as_deref() == Some("lolium_perenne"))
        .unwrap()
        .last_light
        .unwrap();
    assert!((grass_two - two).abs() < 1e-9, "two-oak grass {grass_two} vs {two}");
    assert!(
        grass_two < grass_one - 1e-9,
        "two oaks must darken grass more than one ({grass_two} vs {grass_one})"
    );
}

/// D131: shade / received L independent of plant id / insertion order within a band.
#[test]
fn d131_no_id_order_shade() {
    let mut a = World::new(1314, light_sand_column());
    a.set_climate(20.0, 1000.0);
    a.plant_taxon(0, 0, "lolium_perenne").unwrap();
    a.plant_taxon(0, 0, "quercus_alba").unwrap();
    a.tick();

    let mut b = World::new(1315, light_sand_column());
    b.set_climate(20.0, 1000.0);
    b.plant_taxon(0, 0, "quercus_alba").unwrap();
    b.plant_taxon(0, 0, "lolium_perenne").unwrap();
    b.tick();

    fn lights(w: &World) -> (f64, f64) {
        let mut grass = None;
        let mut oak = None;
        for o in &w.column_at(0, 0).occupants {
            match o.taxon_id.as_deref() {
                Some("lolium_perenne") => grass = o.last_light,
                Some("quercus_alba") => oak = o.last_light,
                _ => {}
            }
        }
        (grass.expect("grass"), oak.expect("oak"))
    }
    let (g_a, o_a) = lights(&a);
    let (g_b, o_b) = lights(&b);
    assert!((o_a - L0).abs() < 1e-12);
    assert!((o_b - L0).abs() < 1e-12);
    assert!((g_a - g_b).abs() < 1e-12, "order must not change grass light");
    assert!((o_a - o_b).abs() < 1e-12, "order must not change oak light");

    // Two same-band oaks: order of planting must not change either oak's L0.
    let mut c = World::new(1316, light_sand_column());
    c.set_climate(20.0, 1000.0);
    c.plant_taxon(0, 0, "quercus_alba").unwrap();
    c.plant_taxon(0, 0, "quercus_alba").unwrap();
    c.tick();
    let mut d = World::new(1317, light_sand_column());
    d.set_climate(20.0, 1000.0);
    // plant grass first then oaks in reverse relative slotting
    d.plant_taxon(0, 0, "lolium_perenne").unwrap();
    d.plant_taxon(0, 0, "quercus_alba").unwrap();
    d.plant_taxon(0, 0, "quercus_alba").unwrap();
    d.tick();
    for o in &c.column_at(0, 0).occupants {
        assert!((o.last_light.unwrap() - L0).abs() < 1e-12);
    }
    let oak_a = alpha_from_params(c.catalog().get("quercus_alba").unwrap());
    let expect_g = L0 * (1.0 - (2.0 * oak_a).min(1.0));
    let g = d
        .column_at(0, 0)
        .occupants
        .iter()
        .find(|o| o.taxon_id.as_deref() == Some("lolium_perenne"))
        .unwrap()
        .last_light
        .unwrap();
    assert!((g - expect_g).abs() < 1e-9);
    for o in &d.column_at(0, 0).occupants {
        if o.taxon_id.as_deref() == Some("quercus_alba") {
            assert!((o.last_light.unwrap() - L0).abs() < 1e-12);
        }
    }
}

/// D131: light bands use height/alpha ids — no species-name branching.
#[test]
fn d131_no_species_name_match() {
    let cat = Catalog::load_embedded().expect("embedded catalog");
    let grass = cat.get("lolium_perenne").expect("lolium_perenne");
    let oak = cat.get("quercus_alba").expect("quercus_alba");
    assert_eq!(grass.form, sim_core::Form::Herb);
    assert_eq!(oak.form, sim_core::Form::Tree);
    let hg = grass.height_m_mature.unwrap();
    let ho = oak.height_m_mature.unwrap();
    assert!(
        (ho - hg).abs() > H_BAND,
        "oak and grass must be in different bands for shade tests"
    );
    assert!(cat.get("grass").is_err());
    assert!(cat.get("oak").is_err());

    let lib = include_str!("../src/lib.rs");
    let catalog = include_str!("../src/catalog.rs");
    for (label, src) in [("lib.rs", lib), ("catalog.rs", catalog)] {
        for pat in [
            r#"== "grass""#,
            r#"== "oak""#,
            r#"== "lolium""#,
            r#"name == "oak""#,
            "if name ==",
            "match name",
        ] {
            assert!(
                !src.contains(pat),
                "{label} contains forbidden species-name pattern: {pat}"
            );
        }
    }
    assert!(lib.contains("H_BAND"));
    let _ = (alpha_from_params(grass), alpha_from_params(oak));
}
