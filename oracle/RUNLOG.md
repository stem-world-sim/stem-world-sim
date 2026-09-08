# Oracle RUNLOG

```
sprint: S04
result: green
branch: feat/s04-hydro-rest
date: 2026-09-08 (PT)
epsilon: 1e-9 relative on f64 water mass; R_MAX=0.15; H_REST=0.02 m; V_REST=1e-4 m
invariants encoded: I1, I2, I3, I4, I5, I6 + D0 + D1 + D3 + D31 + D32 + D33 + D331 + D4 (hydro rest)
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
notes: H_REST=0.02 drop pond edges before weights; after donor+receiver scale drop V≤V_REST. Soil percolate/lateral skip V≤V_REST; infiltrate skip take≤V_REST. at_rest per cell end of tick if no supra-floor flux (pond/soil/infiltrate) as sender or receiver. Sleep next tick if self and all 4-nbrs at_rest (skip infiltrate/pond/drain as donors). Wake on add_rain/add_water (clears at_rest) or any non-rest neighbor. Queries cell_at_rest/grid_at_rest (I6). Mass conserved; no sinks. d331 near-eq assert bumped to ≤H_REST band (deadband). Tick order unchanged.
```
