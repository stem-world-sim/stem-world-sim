# Sprint 1 — D0 column rain

**Status:** ready for oracle
**Clock:** fast (physics). No critic score required to merge.
**Out of scope:** runoff, plants, proxies, client, orbits, chemistry.

## Goal

A single soil column takes rain. Water has mass. Dirt has finite pore space. Extra rain ponds on the surface.

## Deliverables (oracle)

1. Cargo workspace; member `crates/sim-core` only.
2. Types: clock, one column (elevation, surface depth, layers with thickness, theta, porosity).
3. Commands: add rain R; tick infiltration until surface dry or soil saturated.
4. Capacity-step infiltration, not Richards PDE.
5. Tests with exact names in docs/spec/ACCEPTANCE.md.
6. `.github/workflows/ci.yml` running `cargo test --workspace`.
7. `oracle/RUNLOG.md`.

## Tests that must pass

- `i1_moisture_and_surface_nonnegative`
- `i2_isolated_column_mass_conserved`
- `i5_same_seed_same_script_same_state`
- `i6_queries_are_kernel_methods`
- `d0_rain_fits_in_pores_surface_dries`
- `d0_rain_exceeds_pores_remainder_ponds`
- `d0_novice_all_rain_vanishes_is_false`

## Done

PR from `feat/s01-column-rain` green on CI. Spec does not merge kernel PRs.
