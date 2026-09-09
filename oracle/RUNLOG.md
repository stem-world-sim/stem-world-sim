# Oracle RUNLOG

```
sprint: S10
result: green
branch: feat/s10-flyover-budget
date: 2026-09-08 (PT)
epsilon: 1e-9 relative on f64 water mass; R_MAX=0.15; H_REST=0.02 m; V_REST=1e-4 m
constants: E_OPEN=0.02; E_SOIL=0.005; N_ET=10; MAX_OCCUPANTS=8; N_LAYERS=2; P_MAX=0.01; T_WILT=3; PlantStub.shade=0.25; T_SETTLE=64; CHUNK=8
invariants encoded: I1, I2, I3(+et_lost+extract_lost), I4, I5, I6 + D0 + D1 + D3 + D31 + D32 + D33 + D331 + D4 + D41 + D5 + D6 + D7 + D8 + D9 + D91 + D10
tests:
  - i1_moisture_and_surface_nonnegative
  - i2_isolated_column_mass_conserved
  - i3_closed_grid_mass_conserved
  - i4_disconnected_dry_stays_dry
  - i5_same_seed_same_script_same_state
  - i6_queries_are_kernel_methods
  - d0_rain_fits_in_pores_surface_dries
  - d0_rain_exceeds_pores_remainder_ponds
  - d0_novice_all_rain_vanishes_is_false
  - d1_pond_leaves_high_appears_low
  - d1_no_uphill_creation
  - d1_flat_equal_H_no_net_drain
  - d1_flat_pair_equalizes_no_oscillation
  - d1_same_z_three_share
  - d3_clay_ponds_before_sand
  - d3_infiltrate_respects_I_max
  - d3_hill_soil_drains_to_valley
  - d3_below_fc_does_not_drain
  - d3_no_soil_drain_uphill
  - d3_pond_moves_faster_than_soil
  - d31_equal_z_mobile_shares
  - d31_equal_z_below_fc_no_share
  - d31_downslope_beats_equal_z
  - d32_top_percolates_before_lateral
  - d32_no_percolate_below_fc
  - d32_downslope_fills_soil_before_pond
  - d32_contact_limited_by_slower_soil
  - d33_steep_face_does_not_empty_in_one_tick
  - d33_same_z_gets_pond_when_face_capped
  - d33_drowned_lake_heads_equalize_no_oscillation
  - d33_equal_H_no_flux
  - d331_multi_donor_no_head_inversion
  - d331_drowned_lake_no_period2
  - d4_pond_below_H_rest_no_flux
  - d4_drowned_lake_reaches_rest
  - d4_rain_wakes_rest
  - d4_closed_mass_still_conserved
  - d41_no_soil_into_full
  - d41_valley_lake_snaps
  - d41_snap_conserves_mass
  - d41_high_dry_not_in_valley_component
  - d5_no_edge_leak
  - d5_pond_evaps_before_soil
  - d5_soil_evaps_below_fc
  - d5_bot_untouched
  - d5_batch_not_every_tick
  - d5_mass_et_accounts
  - d6_plant_drinks_top
  - d6_mask_can_be_bot
  - d6_two_occupants_sum
  - d6_wilt_stops_uptake
  - d6_extract_in_ledger
  - d6_bare_cell_no_extract
  - d6_cap_eight
  - d7_bare_et_unchanged
  - d7_one_plant_cuts_pond_et
  - d7_two_sum
  - d7_shade_cap_one
  - d7_wilt_no_shade
  - d7_uptake_unchanged
  - d7_ledger
  - d8_desert_sleeps_zero_awake
  - d8_rain_wakes_neighbors
  - d8_drought_wilt
  - d8_rain_fills_then_plants
  - d8_drought_matches_live_sinks
  - d8_no_teleport_uphill
  - d9_ignore_zero_visits
  - d9_observe_one_chunk_only
  - d9_cannot_ignore_moving
  - d9_catchup_other_chunk_untouched
  - d9_drought_chunk_wilts
  - d9_halo_can_export_pond
  - d91_storm_then_catchup_plant_lives
  - d91_matches_live_100
  - d91_rest_drought_still_exact
  - d10_map_constructs
  - d10_ignored_tick_zero_visits
  - d10_frustum_not_world
  - d10_stripe_k30
  - d10_workset_k100
  - d10_storm_k30_still_under_fly
notes: S10 1024x1024 flyover catch_up budgets. Column layers/occupants densified via SmallVec; sparse hydro/catch_up (awake_list, visit_indices, HashSet region bounds, lake_snap_ids); force_sleep_all + catch_up_chunks; profile.test opt-level=3 for wall-clock. Topology: ridge z=8 on x in [128,144) else z=2; Loam theta_fc; plant in view; ox,oy=(16,496). Work set view+look-ahead 64x32 = 32 chunks (sprint "~8" underspecified). Incoming stripe 32x8 = 4 chunks. Storm: ox look-ahead misses ridge — stress 0.3 m pond on incoming stripe + K=30 catch_up. Measured (cargo test, opt-level=3): d10_map_constructs construct~123 ms cells=1048576 VmRSS~990 MB; d10_ignored_tick_zero_visits~0.006 ms; d10_frustum_not_world visits=2048 cap=2240 ~0.31 ms; d10_stripe_k30~4.7 ms; d10_workset_k100~84 ms; d10_storm_k30_still_under_fly~104 ms. Older D0-D91 names green. No silent shrink on alloc.
```
