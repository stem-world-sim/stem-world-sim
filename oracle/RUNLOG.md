# Oracle RUNLOG

```
sprint: S03.1
result: green
branch: feat/s03.1-lateral-interflow
date: 2026-09-07 (PT)
epsilon: 1e-9 relative on f64 water mass
invariants encoded: I1, I2, I3, I4, I5, I6 + D0 + D1 + D3 + D31 (lateral interflow)
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
notes: Replaced gravity_drain_pass with simultaneous S03+S03.1. Candidates = lower-z OR (equal-z && m_nbr < m_i); never z_nbr > z_i. If any lower-z: V=min(D_max,m_i) split equally across all at min_z → neighbor surface. Else flat: per poorer equal-z neighbor V_j=min(D_max/|P|, 0.5*(m_i-m_nbr)), total ≤ D_max and m_i; add to neighbor soil (excess over φ → surface). Column mobile = sum layers; remove mobile top-down; soil fill top-down to φ. Pond S02.1 unchanged. Capillary ≤θ_fc stays. Prefer sand in d31_*; ε=MASS_EPSILON.
```
