//! S17 critic acceptance: tests call the same API the UI calls.

use critic::{Critic, LayerKind, TileOccupantView};
use sim_core::{
    alpha_eff, alpha_from_params, canopy_layers, same_canopy_layer, Catalog, H_BAND, T_CROWD,
};

const PINE: &str = "pinus_taeda";
const RYE: &str = "lolium_perenne";

fn mature(c: &Critic, taxon: &str) -> Vec<TileOccupantView> {
    c.query_tile()
        .occupants
        .into_iter()
        .filter(|o| o.alive && o.taxon.as_deref() == Some(taxon) && (o.height_frac - 1.0).abs() < 1e-9)
        .collect()
}

/// D17: two mature pines share a woody layer; after T_CROWD one remains.
#[test]
fn d17_two_mature_pines_thin() {
    let mut c = Critic::new();
    c.plant(PINE).unwrap();
    c.plant(PINE).unwrap();
    let q = mature(&c, PINE);
    assert_eq!(q.len(), 2);
    assert_eq!(q[0].layer_kind, LayerKind::Woody);
    assert_eq!(q[0].layer_id, q[1].layer_id);
    assert!(q[0].a > 1.0);
    c.tick_n(T_CROWD - 1);
    assert_eq!(mature(&c, PINE).len(), 2);
    c.tick();
    assert_eq!(mature(&c, PINE).len(), 1, "one mature pine after T_CROWD");
}

/// D17: 13.1 m and ~8.6 m pines get different layer_id; both remain after T_CROWD.
#[test]
fn d17_midstory_separate_layer() {
    let mut c = Critic::new();
    c.plant(PINE).unwrap();
    c.plant_height(PINE, 8.6).unwrap();
    let pines: Vec<_> = c
        .query_tile()
        .occupants
        .into_iter()
        .filter(|o| o.taxon.as_deref() == Some(PINE))
        .collect();
    assert_eq!(pines.len(), 2);
    assert!(pines.iter().all(|o| o.layer_kind == LayerKind::Woody));
    assert_ne!(pines[0].layer_id, pines[1].layer_id);
    c.tick_n(T_CROWD);
    let left: Vec<_> = c
        .query_tile()
        .occupants
        .into_iter()
        .filter(|o| o.alive && o.taxon.as_deref() == Some(PINE) && o.height_m > 8.0)
        .collect();
    assert_eq!(left.len(), 2, "dominant and midstory pine both remain");
}

/// D17: two mature rye share an herb band; after T_CROWD one remains.
#[test]
fn d17_herb_band_thins() {
    let mut c = Critic::new();
    c.plant(RYE).unwrap();
    c.plant(RYE).unwrap();
    let q = mature(&c, RYE);
    assert_eq!(q.len(), 2);
    assert_eq!(q[0].layer_kind, LayerKind::Herb);
    assert_eq!(q[0].layer_id, q[1].layer_id);
    assert!(q[0].a > 1.0);
    c.tick_n(T_CROWD - 1);
    assert_eq!(mature(&c, RYE).len(), 2);
    c.tick();
    assert_eq!(mature(&c, RYE).len(), 1, "one mature rye after T_CROWD");
}

/// D17: query layer_id and A match S16.3 (woody layers) and S15 (herb bands) on a fixed setup,
/// and agree with the kernel's own compress scale (band_scale = 1/A when A > 1).
#[test]
fn d17_query_matches_kernel() {
    let cat = Catalog::load_embedded().unwrap();
    let pine = cat.get(PINE).unwrap();
    let rye = cat.get(RYE).unwrap();
    let h = pine.height_m_mature.unwrap();
    let mut c = Critic::new();
    c.plant(PINE).unwrap();
    c.plant(PINE).unwrap();
    c.plant_height(PINE, 8.6).unwrap();
    c.plant(RYE).unwrap();
    c.plant(RYE).unwrap();
    let q = c.query_tile();
    assert_eq!(q.living, 5);
    assert_eq!(q.cap, sim_core::MAX_OCCUPANTS);

    // S16.3 expected layers / loads.
    let heights = [h, h, 8.6];
    let ids = canopy_layers(&heights);
    assert!(same_canopy_layer(h, h) && !same_canopy_layer(h, 8.6));
    let a_pine = alpha_from_params(pine);
    let a_top = 2.0 * alpha_eff(a_pine, 1.0);
    let a_mid = alpha_eff(a_pine, 8.6 / h);
    let woody: Vec<_> = q.occupants.iter().filter(|o| o.layer_kind == LayerKind::Woody).collect();
    assert_eq!(woody.len(), 3);
    for (o, &want_id) in woody.iter().zip(&ids) {
        let want_a = if want_id == ids[0] { a_top } else { a_mid };
        assert_eq!(o.layer_id, want_id);
        assert!((o.a - want_a).abs() < 1e-12, "woody A {} vs {}", o.a, want_a);
    }
    // S15 herb band: two equal mature rye in one H_BAND band.
    let herbs: Vec<_> = q.occupants.iter().filter(|o| o.layer_kind == LayerKind::Herb).collect();
    assert_eq!(herbs.len(), 2);
    assert!((herbs[0].height_m - herbs[1].height_m).abs() <= H_BAND);
    assert_eq!(herbs[0].layer_id, herbs[1].layer_id);
    let a_herb = 2.0 * alpha_eff(alpha_from_params(rye), 1.0);
    assert!((herbs[0].a - a_herb).abs() < 1e-12);

    // After one tick the kernel's crowd counters match "A > 1" from the query.
    c.tick();
    let q = c.query_tile();
    for o in q.occupants.iter().filter(|o| o.height_frac > 0.5) {
        assert_eq!(o.over > 0, o.a > 1.0, "slot {} over={} A={}", o.slot, o.over, o.a);
        assert!(o.light.is_some());
    }
}

/// D17: critic and query code carry no species-name branch and no physics constants.
#[test]
fn d17_no_species_name_match() {
    let critic_lib = include_str!("../src/lib.rs");
    let critic_main = include_str!("../src/main.rs");
    let core = include_str!("../../sim-core/src/lib.rs");
    for (label, src) in [("critic lib", critic_lib), ("critic main", critic_main), ("sim-core", core)] {
        for pat in [
            r#"== "grass""#, r#"== "oak""#, r#"== "pine""#, r#"== "pinus""#, r#"== "lolium""#,
            r#"== "rye""#, "if name ==", "match name", "pinus_taeda", "quercus_alba", "lolium_perenne",
        ] {
            assert!(!src.contains(pat), "{label} contains forbidden pattern: {pat}");
        }
    }
    for (label, src) in [("critic lib", critic_lib), ("critic main", critic_main)] {
        for pat in ["H_BAND", "T_CROWD", "LAYER_GAP", "canopy_layers", "same_canopy_layer", "alpha_eff"] {
            assert!(!src.contains(pat), "{label} must not compute physics: {pat}");
        }
    }
    assert!(core.contains("pub fn query_tile"));
}
