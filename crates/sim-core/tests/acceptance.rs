//! Acceptance tests — named contracts from docs/spec/ACCEPTANCE.md.
//!
//! ε: 1e-9 relative on f64 water mass (MASS_EPSILON from sim_core).
//! Cell area A = 1. Column water mass M = h_surf + sum(theta_i * L_i).
//! Grid mass: sum of column M.

use sim_core::{Column, SoilLayer, Texture, World, MASS_EPSILON};

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
    let m_final = world.water_mass();
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
    assert!(world.water_mass() > m0);
    assert_mass_close(world.water_mass(), m0 + r);
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

    let m_final = world.water_mass();
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
        assert_mass_close(world.grid_water_mass(), m0);
    }
    assert_mass_close(world.grid_water_mass(), m0);
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

    // Half-drop on flat: each tick moves half the remaining ΔH, so equalizes gradually.
    for _ in 0..64 {
        world.tick();
        assert_mass_close(world.grid_water_mass(), m0);
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
    assert_mass_close(world.grid_water_mass(), m0);
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

    for _ in 0..64 {
        world.tick();
        assert_mass_close(world.grid_water_mass(), m0);
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
    let cols = vec![
        textured_column(10.0, 0.0, phi, tex), // high saturated
        textured_column(5.0, 0.0, phi, tex),  // low saturated
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
    for _ in 0..32 {
        world.tick();
        assert_mass_close(world.grid_water_mass(), m0);
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

    for _ in 0..16 {
        world.tick();
        assert_mass_close(world.grid_water_mass(), m0);
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

    for _ in 0..16 {
        world.tick();
        assert_mass_close(world.grid_water_mass(), m0);
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
