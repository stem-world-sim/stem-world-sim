# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.
`cargo test --workspace` must print these names. Missing name = sprint not done.

ε: document in the test (suggested 1e-9 relative on f64 water mass).
Cell area A = 1 implicit.
Column water mass: M = h_surf + sum theta_i * L_i

## Always on

| Rust test name | Invariant |
|---|---|
| `i1_moisture_and_surface_nonnegative` | I1 |
| `i2_isolated_column_mass_conserved` | I2 |
| `i5_same_seed_same_script_same_state` | I5 |
| `i6_queries_are_kernel_methods` | I6 |

I3 and I4 are not Sprint 1.

## Sprint 1 only — D0 column_rain

| Rust test name | Script |
|---|---|
| `d0_rain_fits_in_pores_surface_dries` | R < remaining pore; tick until surface dry |
| `d0_rain_exceeds_pores_remainder_ponds` | R > remaining pore; soil at phi; pond remainder |
| `d0_novice_all_rain_vanishes_is_false` | same as B; "dirt eats all rain" must fail |
