//! Sprint 1 acceptance tests — named contracts from docs/spec/ACCEPTANCE.md.
//!
//! ε: 1e-9 relative on f64 water mass (MASS_EPSILON from sim_core).
//! Cell area A = 1. Column water mass M = h_surf + sum(theta_i * L_i).

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
