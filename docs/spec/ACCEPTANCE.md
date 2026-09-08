# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.
`cargo test --workspace` must print these names.
ε: 1e-9 relative on f64 water mass. A = 1.
M = h_surf + sum theta_i * L_i. Do not delete older tests.

## Always on

| Rust test name | Invariant |
|---|---|
| `i1_moisture_and_surface_nonnegative` | I1 |
| `i2_isolated_column_mass_conserved` | I2 |
| `i5_same_seed_same_script_same_state` | I5 |
| `i6_queries_are_kernel_methods` | I6 |
| `i3_closed_grid_mass_conserved` | I3 |
| `i4_disconnected_dry_stays_dry` | I4 |

## Sprint 1 — D0

| Rust test name | Script |
|---|---|
| `d0_rain_fits_in_pores_surface_dries` | R < remaining pore; tick until surface dry |
| `d0_rain_exceeds_pores_remainder_ponds` | R > remaining pore; soil at phi; pond remainder |
| `d0_novice_all_rain_vanishes_is_false` | dirt eats all rain must fail |

## Sprint 2 — D1

| Rust test name | Script |
|---|---|
| `d1_pond_leaves_high_appears_low` | pond on high; low M up |
| `d1_no_uphill_creation` | pond only on low; high stays dry |
| `d1_flat_equal_H_no_net_drain` | same z same h; no net pond transfer |
| `d1_flat_pair_equalizes_no_oscillation` | 1x2 same z; heads equalize |
| `d1_same_z_three_share` | 1x3 same z; rain center; both ends gain surface |

## Sprint 3 — water mechanics

| Rust test name | Script |
|---|---|
| `d3_clay_ponds_before_sand` | same R; after 1 tick clay has more h than sand |
| `d3_infiltrate_respects_I_max` | after 1 tick infiltrated depth ≤ I_max |
| `d3_hill_soil_drains_to_valley` | slope; high starts θ=φ; high θ falls toward fc; low M rises |
| `d3_below_fc_does_not_drain` | high at θ_fc; high M unchanged |
| `d3_no_soil_drain_uphill` | low at φ; high does not gain from low |
| `d3_pond_moves_faster_than_soil` | after 2 ticks more valley mass from pond case than soil case |

## Sprint 3.1 — equal-z interflow

Use a 1x2 flat (same z). Prefer sand unless named.

| Rust test name | Script |
|---|---|
| `d31_equal_z_mobile_shares` | both sand; A starts θ=φ, B at θ_fc; after ticks B θ rises and A θ falls; no oscillation swap |
| `d31_equal_z_below_fc_no_share` | A at θ_fc, B drier; A M unchanged |
| `d31_downslope_beats_equal_z` | 3 cells: high wet, two lows same z; drain prefers the lower-z neighbor over a same-z highland neighbor |
