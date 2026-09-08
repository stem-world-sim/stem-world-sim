# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.
`cargo test --workspace` must print these names. Missing name = sprint not done.

ε: document in the test (suggested 1e-9 relative on f64 water mass).
Cell area A = 1 implicit.
Column water mass: M = h_surf + sum theta_i * L_i
Grid mass: sum of column M.

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

## Sprint 1 — D0 column_rain

| Rust test name | Script |
|---|---|
| `d0_rain_fits_in_pores_surface_dries` | R < remaining pore; tick until surface dry |
| `d0_rain_exceeds_pores_remainder_ponds` | R > remaining pore; soil at phi; pond remainder |
| `d0_novice_all_rain_vanishes_is_false` | same as B; dirt eats all rain must fail |

## Sprint 2 — D1 slope_runoff

| Rust test name | Script |
|---|---|
| `i3_closed_grid_mass_conserved` | rain on one cell of a closed 1x2 or 2x2; sum M constant |
| `i4_disconnected_dry_stays_dry` | connected pair plus isolated dry; isolated M unchanged |
| `d1_pond_leaves_high_appears_low` | z_high > z_low; pond on high; high surface down, low M up |
| `d1_no_uphill_creation` | pond only on the low cell; high stays dry |
| `d1_flat_equal_H_no_net_drain` | same z and same h; no net transfer |

## Sprint 2.1 — head equalize

Use saturated soil (θ = φ) so pond stays on the surface.

| Rust test name | Script |
|---|---|
| `d1_flat_pair_equalizes_no_oscillation` | 1x2, same z, all pond on cell A; after ticks h_A ≈ h_B; one extra tick does not swap |
| `d1_same_z_three_share` | 1x3 same z, rain on center; both ends gain surface water |
