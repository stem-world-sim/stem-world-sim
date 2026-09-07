# Oracle RUNLOG

```
sprint: S02
result: green
branch: feat/s02-slope-runoff
date: 2026-09-07 (PT)
epsilon: 1e-9 relative on f64 water mass
invariants encoded: I1, I2, I3, I4, I5, I6 + D0 + D1
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
notes: Grid World (4-neighbor); S01 1×1 preserved. Tick = infiltrate_all + simultaneous runoff pass. Head H = z + h_surf; send only to strictly lower H; tie-break N,E,S,W; vol = min(h_i, H_i - H_nbr). Closed boundary. CI via CARGO_TARGET_DIR=/tmp/sws-target cargo test --workspace.
```
