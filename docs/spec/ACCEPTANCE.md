# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.
`cargo test --workspace` must print these names. Missing name = sprint not done.

ε: 1e-9 relative on f64 water mass. A = 1.
M_column = h_surf + sum theta_i * L_i. Grid mass = sum of column M.
Do not delete tests from earlier sprints.

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
| `d1_pond_leaves_high_appears_low` | pond on high; high surface down, low M up |
| `d1_no_uphill_creation` | pond only on low; high stays dry |
| `d1_flat_equal_H_no_net_drain` | same z same h; no net pond transfer |
| `d1_flat_pair_equalizes_no_oscillation` | 1x2 same z; heads equalize; no swap |
| `d1_same_z_three_share` | 1x3 same z; rain center; both ends gain surface |

## Sprint 3 — water mechanics

Use Sand default unless the test names clay/loam. After drain ticks, grid mass still conserved (I3).

| Rust test name | Script |
|---|---|
| `d3_clay_ponds_before_sand` | same R, same z, 1x1; after 1 tick clay has more h_surf than sand |
| `d3_infiltrate_respects_I_max` | sand 1x1; R large; after 1 tick, infiltrated depth ≤ I_max_sand + ε |
| `d3_hill_soil_drains_to_valley` | 1x2, z_high > z_low, both start θ=φ (sand); after enough ticks high θ drops toward θ_fc and low M rises |
| `d3_below_fc_does_not_drain` | 1x2 slope; high θ = θ_fc; after ticks high M unchanged (no leak) |
| `d3_no_soil_drain_uphill` | 1x2; low starts θ=φ, high dry-ish; high does not gain soil/pond from low drain |
| `d3_pond_moves_faster_than_soil` | same slope; case A pond-only on high (soils at fc); case B no pond, high at φ; after 2 ticks more mass arrives in the valley from A than from B |
