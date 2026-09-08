# Oracle RUNLOG

```
sprint: S03
result: green
branch: feat/s03-water-mechanics
date: 2026-09-07 (PT)
epsilon: 1e-9 relative on f64 water mass
invariants encoded: I1, I2, I3, I4, I5, I6 + D0 + D1 + D3 (water mechanics)
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
notes: Texture {Sand,Loam,Clay} on SoilLayer (φ, θ_fc=0.5φ, I_max, D_max from table). Default Sand. Tick = rate-limited infiltrate (budget min(h,I_max) top-down) + S02.1 runoff + gravity_drain_pass (mobile θ>θ_fc, m=min(D_max,m_avail) to strictly lower-z neighbor NESW/lowest-z, onto surface; 1×1/pit no-op). Capillary ≤θ_fc stays. S02 theta-never-moves revoked for mobile only. tick_until bound accounts for I_max. CI via CARGO_TARGET_DIR=/tmp/sws-target cargo test --workspace.
```
