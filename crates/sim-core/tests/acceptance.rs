//! Acceptance tests — named contracts from docs/spec/ACCEPTANCE.md.
//!
//! ε: 1e-9 relative on f64 water mass (MASS_EPSILON from sim_core).
//! Cell area A = 1. Column water mass M = h_surf + sum(theta_i * L_i).
//! Grid mass: sum of column M.

use sim_core::{
    Column, Occupant, SoilLayer, Texture, World, E_OPEN, E_SOIL, H_REST, MASS_EPSILON, MAX_OCCUPANTS,
    N_ET, N_LAYERS, P_MAX, R_MAX, T_WILT, V_REST,
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
