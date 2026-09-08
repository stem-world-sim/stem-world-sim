# Oracle RUNLOG

```
sprint: S08
result: green
branch: feat/s08-fidelity
date: 2026-09-08 (PT)
epsilon: 1e-9 relative on f64 water mass; R_MAX=0.15; H_REST=0.02 m; V_REST=1e-4 m
constants: E_OPEN=0.02; E_SOIL=0.005; N_ET=10; MAX_OCCUPANTS=8; N_LAYERS=2; P_MAX=0.01; T_WILT=3; PlantStub.shade=0.25; T_SETTLE=64
invariants encoded: I1, I2, I3(+et_lost+extract_lost), I4, I5, I6 + D0 + D1 + D3 + D31 + D32 + D33 + D331 + D4 + D41 + D5 + D6 + D7 + D8
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
notes: S08 awake set + catch_up. Explicit awake[] visit set for hydro; rain/add_water/add_occupant/set_column/flux-recv wake cell+4-nbrs; after hydro (and after live sinks) prune cells that are at_rest with all 4-nbrs at_rest. awake_count() query. tick = hydro_step + clock + optional ET/occupants (live sinks still scan all cells for S05-S07 cadence; losers re-enter awake). catch_up(K,rain): every cell h+=K*rain_per_step (wake if rain>0) -> settle hydro_step <= T_SETTLE=64 until awake_count==0 -> K unit ET+occupant steps (exact drought match) -> clock += K*N_ET. T_SETTLE exceeded leaves movers awake. Books: rain on grid; et_lost/extract_lost from batched sinks. Older D0-D7 names green.
```
