# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.
`cargo test --workspace` must print these names.
ε: 1e-9 relative on f64 water mass. A = 1.
Do not delete older tests.

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
| `d1_no_uphill_creation` | pond only on low with H_low < H_high; high stays dry |
| `d1_flat_equal_H_no_net_drain` | same z same h; no net pond transfer |
| `d1_flat_pair_equalizes_no_oscillation` | 1x2 same z; heads equalize |
| `d1_same_z_three_share` | 1x3 same z; rain center; both ends gain surface |

## Sprint 3 — water mechanics

| Rust test name | Script |
|---|---|
| `d3_clay_ponds_before_sand` | after 1 tick clay has more h than sand |
| `d3_infiltrate_respects_I_max` | infiltrated depth ≤ I_max |
| `d3_hill_soil_drains_to_valley` | high starts θ=φ; high θ falls toward fc; low M rises |
| `d3_below_fc_does_not_drain` | high at θ_fc; high M unchanged |
| `d3_no_soil_drain_uphill` | low at φ; high does not gain from low soil |
| `d3_pond_moves_faster_than_soil` | more valley mass from pond case than soil case after 2 ticks |

## Sprint 3.1 — equal-z interflow

| Rust test name | Script |
|---|---|
| `d31_equal_z_mobile_shares` | A at φ, B at θ_fc; B θ rises, A θ falls |
| `d31_equal_z_below_fc_no_share` | A at θ_fc; A M unchanged |
| `d31_downslope_beats_equal_z` | drain prefers lower-z neighbor |

## Sprint 3.2 — profile first

| Rust test name | Script |
|---|---|
| `d32_top_percolates_before_lateral` | profile fill before lateral |
| `d32_no_percolate_below_fc` | top at θ_fc stays |
| `d32_downslope_fills_soil_before_pond` | low θ rises before all leftover becomes h |
| `d32_contact_limited_by_slower_soil` | sand→clay flux ≤ D_clay |

## Sprint 3.3 — pond head + R_max

`R_max = 0.15`. Sand ok unless named.

| Rust test name | Script |
|---|---|
| `d33_steep_face_does_not_empty_in_one_tick` | 1x2 slope Δz=6; high h=0.50, soils at φ so no soak; after 1 tick high h > 0 and high lost ≤ R_max +ε to the downhill edge |
| `d33_same_z_gets_pond_when_face_capped` | 2x2: high-A has pond, high-B same z dry, valley under A empty; after 1 tick high-B h > 0 (same-z edge) AND valley M rose |
| `d33_drowned_lake_heads_equalize_no_oscillation` | 1x2 Δz=6; put enough pond on low that H_low ≈ H_high; tick N≥15; |H_high-H_low| shrinks; last 3 ticks each |h| change < 1e-4 |
| `d33_equal_H_no_flux` | two cells any z with H equal; no net pond move |
