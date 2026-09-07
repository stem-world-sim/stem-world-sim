# Oracle RUNLOG

```
sprint: S01
result: green
branch: feat/s01-column-rain
date: 2026-09-07 (PT)
epsilon: 1e-9 relative on f64 water mass
invariants encoded: I1, I2, I5, I6 + D0
tests:
  - i1_moisture_and_surface_nonnegative
  - i2_isolated_column_mass_conserved
  - i5_same_seed_same_script_same_state
  - i6_queries_are_kernel_methods
  - d0_rain_fits_in_pores_surface_dries
  - d0_rain_exceeds_pores_remainder_ponds
  - d0_novice_all_rain_vanishes_is_false
notes: Capacity-step infiltration (not Richards). Isolated column mass M = h_surf + sum(theta_i*L_i). CI via CARGO_TARGET_DIR=/tmp/sws-target cargo test --workspace.
```
