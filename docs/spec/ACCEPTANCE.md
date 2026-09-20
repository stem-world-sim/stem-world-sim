# Acceptance tests (named)

Oracle must implement these as Rust tests. **Function names are the contract.**
`cargo test --workspace` must print these names. Missing name = sprint not done.

ε: document in the test (suggested `1e-9` relative on f64 water mass).

## Always on

- i1_moisture_and_surface_nonnegative
- i2_isolated_column_mass_conserved
- i5_same_seed_same_script_same_state
- i6_queries_are_kernel_methods

## Sprint 1 — D0 column_rain

- d0_rain_fits_in_pores_surface_dries
- d0_rain_exceeds_pores_remainder_ponds
- d0_novice_all_rain_vanishes_is_false

## Sprint 11 — kernel catalog

- d11_catalog_loads_demo_ten
- d11_unknown_taxon_is_err
- d11_rice_root_null_uses_herb_mask
- d11_oak_deeper_than_rice_default_mask
- d11_no_species_name_match_in_sim_core
- d11_same_rain_two_taxa_queryable

## Sprint 12 — climate envelope

- d12_rice_dies_below_tmin
- d12_wheat_lives_at_same_t
- d12_rice_lives_in_envelope
- d12_wheat_dies_above_tmax
- d12_rice_dies_dry_year
- d12_bucket_is_not_climate
- d12_empty_envelope_no_veto
- d12_no_species_name_match

Do not delete tests. Later sprints only add rows.
