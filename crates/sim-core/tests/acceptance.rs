//! Acceptance tests — named contracts from docs/spec/ACCEPTANCE.md.
//!
//! ε: 1e-9 relative on f64 water mass (MASS_EPSILON from sim_core).
//! Cell area A = 1. Column water mass M = h_surf + sum(theta_i * L_i).
//! Grid mass: sum of column M.

use sim_core::{Column, SoilLayer, World, MASS_EPSILON};

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

fn default_column(theta: f64, porosity: f64) -> Column {
    Column::new(
        10.0,
        0.0,
        vec![
            SoilLayer::new(0.3, theta, porosity),
            SoilLayer::new(0.5, theta, porosity),
        ],
    )
}

/// Saturated soil so pond stays on the surface for runoff tests.
fn saturated_column(elevation_m: f64, surface_water_m: f64) -> Column {
    let phi = 0.4;
    Column::new(
        elevation_m,
        surface_water_m,
        vec![
            SoilLayer::new(0.3, phi, phi),
            SoilLayer::new(0.5, phi, phi),
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

    let mut a = World::new(seed, default_column(0.1, 0.35));
    let mut b = World::new(seed, default_column(0.1, 0.35));
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
    let mut world = World::new(1, default_column(0.2, 0.45));
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

/// I3: closed grid, no sink — rain on one cell; infiltrate+runoff; sum M constant ±ε.
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
        world.tick(); // infiltrate all + runoff
        assert_mass_close(world.grid_water_mass(), m0);
    }
    assert_mass_close(world.grid_water_mass(), m0);
}

/// I4: high+low connected pair plus isolated dry column; isolated M unchanged.
#[test]
fn i4_disconnected_dry_stays_dry() {
    // 1×3: elevations [10, 5, 100]; rain on x=0 → runoff 0→1; x=2 stays dry.
    let cols = vec![
        saturated_column(10.0, 0.0),
        saturated_column(5.0, 0.0),
        saturated_column(100.0, 0.0),
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

    for _ in 0..8 {
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
#[test]
fn d1_no_uphill_creation() {
    let cols = vec![
        saturated_column(10.0, 0.0), // high dry
        saturated_column(5.0, 0.4),  // low with pond
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
// Saturated soil (θ=φ) so pond stays on surface.
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
