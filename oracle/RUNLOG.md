# Oracle RUNLOG

```
sprint: S03.2
result: green
branch: feat/s03.2-profile-first
date: 2026-09-08 (PT)
epsilon: 1e-9 relative on f64 water mass
invariants encoded: I1, I2, I3, I4, I5, I6 + D0 + D1 + D3 + D31 + D32 (profile-first percolation)
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
notes: Split gravity_drain_pass into percolate_all() then lateral_drain_pass() after S02.1 runoff. Percolate: per column, snapshot θ, mobile in layer k fills pore in k+1…; shared texture → one D_max budget; never below θ_fc. Column D_max shared with lateral (remaining after percolate). Lateral on leftover m: same candidates as S03.1; contact flux ≤ min(D_rem_i,D_j); equal-z half-diff of leftover m; total ≤ rem D and m; receiver soil top-down (overflow → surface). Downslope no longer dumps to surface when neighbor has pore. Capillary ≤θ_fc stays. Pond S02.1 and infiltrate ≤ I_max unchanged. d31 asserts soil M / any-layer θ (profile may park water deep). Prefer sand in d32_*; ε=MASS_EPSILON.
```
