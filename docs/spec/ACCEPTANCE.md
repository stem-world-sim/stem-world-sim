# Acceptance tests (named)

Function names are the contract.

## Always on

- i1_moisture_and_surface_nonnegative
- i2_isolated_column_mass_conserved
- i5_same_seed_same_script_same_state
- i6_queries_are_kernel_methods

## Sprint 12 — climate envelope (two rain-year tests AMENDED in S12.1)

- d12_rice_dies_below_tmin
- d12_wheat_lives_at_same_t
- d12_rice_lives_in_envelope
- d12_wheat_dies_above_tmax
- d12_rice_dies_dry_year (AMEND: rice remains at T=22 / 200 mm live tick)
- d12_bucket_is_not_climate (AMEND: rain_year unchanged by 0.05 m rain; rice remains)
- d12_empty_envelope_no_veto
- d12_no_species_name_match

## Sprint 12.1

- d121_plant_rice_in_dry_year_if_warm
- d121_rice_dies_when_air_turns_cold
- d121_wheat_waterlogs_in_pond
- d121_rice_pond_no_waterlog
- d121_wheat_submerged_over_height
- d121_oak_not_submerged_in_shallow_pond
- d121_drought_kills_at_good_rain_year
- d121_no_species_name_match
