# Acceptance tests (named)

Function names are the contract. Do not delete prior names.

## Always on
- i1_moisture_and_surface_nonnegative
- i2_isolated_column_mass_conserved
- i5_same_seed_same_script_same_state
- i6_queries_are_kernel_methods

## S11
- d11_catalog_loads_demo_ten
- d11_unknown_taxon_is_err
- d11_rice_root_null_uses_herb_mask
- d11_oak_deeper_than_rice_default_mask
- d11_no_species_name_match_in_sim_core
- d11_same_rain_two_taxa_queryable

## S12 (two rain-year tests amended S12.1)
- d12_rice_dies_below_tmin
- d12_wheat_lives_at_same_t
- d12_rice_lives_in_envelope
- d12_wheat_dies_above_tmax
- d12_rice_dies_dry_year
- d12_bucket_is_not_climate
- d12_empty_envelope_no_veto
- d12_no_species_name_match

## S12.1
- d121_plant_rice_in_dry_year_if_warm
- d121_rice_dies_when_air_turns_cold
- d121_wheat_waterlogs_in_pond
- d121_rice_pond_no_waterlog
- d121_wheat_submerged_over_height
- d121_oak_not_submerged_in_shallow_pond
- d121_drought_kills_at_good_rain_year
- d121_no_species_name_match

## S13
- d13_grass_alone_full_light
- d13_oak_over_grass_shades
- d13_grass_dies_in_oak_shade
- d13_remove_oak_grass_lives
- d13_two_herbs_neither_dark
- d13_et_uses_alpha
- d13_no_species_name_match
