# Oracle RUNLOG

```
sprint: S17.1
result: green
branch: feat/s17.1-critic-page
date: 2026-10-15 (PT)
epsilon: 1e-9 relative on f64 water mass; R_MAX=0.15; H_REST=0.02 m; V_REST=1e-4 m
constants: E_OPEN=0.02; E_SOIL=0.005; N_ET=10; MAX_OCCUPANTS=8; N_LAYERS=2; P_MAX=0.01; T_WILT=3; PlantStub.shade=0.25; T_SETTLE=64; CHUNK=8; H_POND=0.02; T_WL=7; T_WL_D=3; T_SUB=7; T_DARK=7; T_CROWD=7; L0=1; H_BAND=0.05; T_MIN=0.05; C_MAX=0.85; LAYER_GAP_MIN_M=1.0; LAYER_GAP_FRAC=0.25; H0=0.10; F_FERTILE=0.5; T_MATURE herb/shrub/tree=20/80/200; L_opt I|empty=1.00 M=0.60 T=0.35; theta_pwp=0.5*theta_fc
growth_cadence: once per live tick on visit set, after light filter (uses last_light); catch_up/catch_up_chunk once per calendar block after hydro before sinks (light+compress+crowd when any alive; hold last_light; same Δ0·f_L·f_w·f_T; f_w drain-shaped; wilted f_w=0)
seed_rain: after growth each live tick / catch_up block; fertile=alive∧height_frac≥F_FERTILE; one attempt/fertile; targets N,E,S,W,same first under MAX_OCCUPANTS; dispersal wind|animal→neighbors+same, water→same|pond(h_surf>0)|lower elevation_m, short|empty|other→same; seedling H0; no RNG
canopy_bucket: form∈{tree,shrub} share one per-tile A_canopy=Σ(α·height_frac²); A_canopy>1 → s=1/A_canopy on α_eff+uptake for those members; T_CROWD=7 consecutive → drop weakest (lowest f_L·f_w·f_T·height_frac; tie lower frac then lower index); reset when A_canopy≤1. Herbs stay on H_BAND bands (herb-only A_b) and are not in the canopy sum/thin. Light still walks H_BAND bands top-down; each member casts compressed α (canopy s or herb s_b) for S13.2 transmission. Catch_up uses same bucket. Seed pulse unchanged (S16 once/tick). No RNG; no English-name branch.
critic: S17 adds crate crates/critic (depends on sim-core only). API plant(taxon), plant_height(taxon,m), tick(), tick_n(k), query_tile(). sim-core gains World::query_tile(x,y) -> TileView{living, cap, occupants[slot,taxon,height_m,height_frac,layer_id,layer_kind woody|herb,A,over,light,alive]}; grouping uses the kernel's own sort, H_BAND herb bands and canopy_layers; A = bucket load Σα·height_frac²; over = crowd_steps; light = last_light. Read-only, no physics change. egui window (eframe 0.31): demo-taxon plant buttons from catalog, Tick, Tick×N, K, Play/Pause, Reset, layer grid. Native: cargo run -p critic. Wasm: cd crates/critic && trunk build --release -> crates/critic/dist/ (git-ignored, overwritten).
canopy_layers: S16.3 splits the woody bucket into height layers: two living tree/shrub occupants share a layer when |h_i-h_j| ≤ max(1 m, 0.25·max(h_i,h_j)), transitive (union over all pairs). Each layer has own A_L=Σα·height_frac², s_L=1/A_L if >1, crowd counter and T_CROWD thin of weakest in that layer. Lower-layer stems are not summed or removed by an upper layer. Herbs unchanged. Light walk unchanged. Catch_up uses same apply_light_filter_at path.
invariants encoded: I1, I2, I3(+et_lost+extract_lost), I4, I5, I6 + D0 + D1 + D3 + D31 + D32 + D33 + D331 + D4 + D41 + D5 + D6 + D7 + D8 + D9 + D91 + D10 + D11 + D12 + D121 + D13 + D131 + D132 + D14 + D141 + D142 + D15 + D16 + D162 + D163 + D17
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
  - d31_equal_z_mobile_shares
  - d31_equal_z_below_fc_no_share
  - d31_downslope_beats_equal_z
  - d32_top_percolates_before_lateral
  - d32_no_percolate_below_fc
  - d32_downslope_fills_soil_before_pond
  - d32_contact_limited_by_slower_soil
  - d33_steep_face_does_not_empty_in_one_tick
  - d33_same_z_gets_pond_when_face_capped
  - d33_drowned_lake_heads_equalize_no_oscillation
  - d33_equal_H_no_flux
  - d331_multi_donor_no_head_inversion
  - d331_drowned_lake_no_period2
  - d4_pond_below_H_rest_no_flux
  - d4_drowned_lake_reaches_rest
  - d4_rain_wakes_rest
  - d4_closed_mass_still_conserved
  - d41_no_soil_into_full
  - d41_valley_lake_snaps
  - d41_snap_conserves_mass
  - d41_high_dry_not_in_valley_component
  - d5_no_edge_leak
  - d5_pond_evaps_before_soil
  - d5_soil_evaps_below_fc
  - d5_bot_untouched
  - d5_batch_not_every_tick
  - d5_mass_et_accounts
  - d6_plant_drinks_top
  - d6_mask_can_be_bot
  - d6_two_occupants_sum
  - d6_wilt_stops_uptake
  - d6_extract_in_ledger
  - d6_bare_cell_no_extract
  - d6_cap_eight
  - d7_bare_et_unchanged
  - d7_one_plant_cuts_pond_et
  - d7_two_sum
  - d7_shade_cap_one
  - d7_wilt_no_shade
  - d7_uptake_unchanged
  - d7_ledger
  - d8_desert_sleeps_zero_awake
  - d8_rain_wakes_neighbors
  - d8_drought_wilt
  - d8_rain_fills_then_plants
  - d8_drought_matches_live_sinks
  - d8_no_teleport_uphill
  - d9_ignore_zero_visits
  - d9_observe_one_chunk_only
  - d9_cannot_ignore_moving
  - d9_catchup_other_chunk_untouched
  - d9_drought_chunk_wilts
  - d9_halo_can_export_pond
  - d91_storm_then_catchup_plant_lives
  - d91_matches_live_100
  - d91_rest_drought_still_exact
  - d10_map_constructs
  - d10_ignored_tick_zero_visits
  - d10_frustum_not_world
  - d10_stripe_k30
  - d10_workset_k100
  - d10_storm_k30_still_under_fly
  - d11_catalog_loads_demo_ten
  - d11_unknown_taxon_is_err
  - d11_rice_root_null_uses_herb_mask
  - d11_oak_deeper_than_rice_default_mask
  - d11_no_species_name_match_in_sim_core
  - d11_same_rain_two_taxa_queryable
  - d12_rice_dies_below_tmin
  - d12_wheat_lives_at_same_t
  - d12_rice_lives_in_envelope
  - d12_wheat_dies_above_tmax
  - d12_rice_dies_dry_year
  - d12_bucket_is_not_climate
  - d12_empty_envelope_no_veto
  - d12_no_species_name_match
  - d121_plant_rice_in_dry_year_if_warm
  - d121_rice_dies_when_air_turns_cold
  - d121_wheat_waterlogs_in_pond
  - d121_rice_pond_no_waterlog
  - d121_wheat_submerged_over_height
  - d121_oak_not_submerged_in_shallow_pond
  - d121_drought_kills_at_good_rain_year
  - d121_no_species_name_match
  - d13_grass_alone_full_light
  - d13_oak_over_grass_shades
  - d13_grass_dies_in_oak_shade
  - d13_remove_oak_grass_lives
  - d13_two_herbs_neither_dark
  - d13_et_uses_alpha
  - d13_no_species_name_match
  - d131_two_oaks_same_light
  - d131_two_oaks_darker_grass_than_one
  - d131_no_id_order_shade
  - d131_no_species_name_match
  - d132_understory_never_zero
  - d132_eight_oaks_hits_cap
  - d132_two_oaks_not_black
  - d132_no_species_name_match
  - d14_plant_taxon_is_mature
  - d14_seedling_starts_short
  - d14_seedling_oak_grass_lives
  - d14_intolerant_prefers_sun
  - d14_tolerant_prefers_gap
  - d14_grown_intolerant_reaches_mature
  - d14_mature_oak_still_shades
  - d14_wilt_stops_growth
  - d14_cool_grows_slower_than_opt
  - d14_no_species_name_match
  - d141_catchup_grows
  - d141_matches_live
  - d141_wilt_block_no_grow
  - d141_ignore_without_catchup_frozen
  - d142_wetland_beats_mesic_when_ponded
  - d142_mesic_full_at_fc_no_pond
  - d142_xeric_slower_at_fc
  - d142_pond_zeros_xeric_fw
  - d142_wilt_still_zero
  - d142_catchup_pond_same_as_live
  - d142_no_species_name_match
  - d15_two_seedling_oaks_both_live
  - d15_two_mature_oaks_compress
  - d15_two_mature_oaks_thin
  - d15_grass_under_one_oak_lives
  - d15_weaker_loses
  - d15_catchup_thins_same_as_live
  - d15_no_species_name_match
  - d16_fertile_pine_seeds_neighbor
  - d16_seedling_not_fertile
  - d16_short_stays_home
  - d16_oak_seed_on_crowded_tile_dies
  - d16_water_no_uphill_dry
  - d16_catchup_seeds_same_as_live
  - d16_no_species_name_match
  - d162_two_mature_pines_thin
  - d162_pine_one_tick_apart_still_thins
  - d162_seedling_under_pine_lives
  - d162_herb_not_in_canopy
  - d162_catchup_thins_same_as_live
  - d162_no_species_name_match
  - d163_two_mature_pines_thin
  - d163_midstory_not_in_dominant_layer
  - d163_seedling_not_in_mature_layer
  - d163_near_heights_still_share
  - d163_herb_not_in_layer
  - d163_catchup_layers_same_as_live
  - d163_no_species_name_match
  - d17_two_mature_pines_thin
  - d17_midstory_separate_layer
  - d17_herb_band_thins
  - d17_query_matches_kernel
  - d17_no_species_name_match
  - d171_plant_height_midstory
  - d171_plant_seedling
notes: S17 critic on sim-core, one tile, native + wasm. The UI formats query_tile output only; it does not decide layers, thinning or light. No prior test amended. Result 167 acceptance + 4 unit + 5 critic green. Seed pulse, MAX_OCCUPANTS, layer rule, herb bands unchanged. No RNG, no English-name branch. Cargo.lock untracked. docs/ untouched.
notes S17.1: critic plant height control (seedling 0.10×mature, half, mature default, metres field) via Critic::plant_choice → sim-core plant_taxon/plant_params; Play advances K (same as Tick×N) per burst then waits 500 ms; Pause stops; Reset clears tile. No physics in UI. Page: trunk build --release; docs/tools/pack_critic.py run as-is but glue exports `__wbg_init as default` (no `init`), so a patched copy outside the repo (/tmp/pack_critic.py) strips the export line, aliases init, calls init({module_or_path: bytes}); critic.html.gz uploaded to release tag `critic`, not on main; dist/ not committed. Result 167 acceptance + 4 unit + 7 critic green. docs/ untouched.
```

## S17.2 — critic can be read (2026-10-17 PT)
tests added:
  - d172_common_name_on_demo
  - d172_taeda_fills_then_wilts
notes S17.2: catalog parses optional `common_name` column (OccupantParams::common_name, display_name → `Common (Scientific)`, scientific only if missing). query_tile adds tick, pond_m, per-layer (θ, θ_fc), and per-plant dry/dark/waterlog/submerge/crowd steps; STRESS_THRESHOLDS exports T_WILT/T_DARK/T_WL/T_WL_D/T_SUB/T_CROWD for display. Critic: reason line per plant from counters, event log (seed placed / wilted / thinned / removed) by diffing query snapshots per tick, rain button via existing add_rain_at, Key panel one tap away, narrow (<600 pt) layout wraps and enlarges tap targets inside a vertical scroll. No physics, seed rain, cap, uptake or wilt change. No prior test amended. Result 168 acceptance + 4 unit + 8 critic green. Page rebuilt and critic.html.gz replaced on release `critic`. docs/ untouched.

## S18 — telemetry, continuous rain, text size, unified clock (2026-10-18 PT)
tests added:
  - d18_root_depth_queryable
  - d18_oak_root_grows_to_next_layer
  - d18_continuous_rain_applied_each_tick
  - d18_tick_dt_scales_growth_and_et
  - d18_mass_conserved_under_scaled_dt
  - d18_default_dt_matches_prior_behavior
  - d18_catchup_root_and_rain
invariants: (1) one clock — a tick lasts tick_dt base units, base DT_BASE_S = 1 day = the prior implicit tick, default dt = 1; every per-tick rate scales by dt (I_max, D_max, R_MAX, E_OPEN, E_SOIL, uptake, growth Δ0); flux rest floor V_REST·min(dt,1); K ticks advance K·dt (sim_time_s), catch_up K blocks advance K·N_ET·dt. Stress counters still count steps (thresholds unchanged, non-goal). (2) continuous rain — rain_per_tick_m lands on each observed-chunk cell at the start of every live tick, booked in rain_added_m; catch_up / catch_up_chunk add K·N_ET·rain_per_tick_m in their source dump. Closed basin: initial + adds + rain_added = grid mass + et_lost + extract_lost. (3) roots — r = catalog root_depth_m · height_frac; layer k ≥ 1 is reached iff the catalog mask has k and its top depth < r (layer 0 always); uptake and f_w use only reached layers; no catalog depth → stored mask as before. Catch-up uses the same rule (root follows height_frac).
notes S18: query_tile adds per-plant root_depth_m + max_layer and tile tick_dt, sim_time_s, rain_per_tick_m. Critic: text-size slider (egui zoom), tick length radio (1 s / 1 day / 1 month / 1 year via TickUnit), rain-per-tick field (mm, applied by the kernel each tick; 10 mm button kept), roots shown per plant, elapsed time shown. Kernel fix: percolate's θ_fc floor now guards sources only — snapping a below-θ_fc receiver up to θ_fc created mass when uptake/ET at large dt dried a lower layer. Default dt reproduces a pre-S18 script exactly (golden from 77cd639).
amended: d16_catchup_seeds_same_as_live — seedlings now drink only reached layers, so a catch_up seedling can stay at exactly H0 on a dry top layer; the test now compares pines placed north (= K seed attempts) for live and catch_up instead of counting plants at exactly H0. No test deleted or renamed. Result 175 acceptance + 4 unit + 8 critic green. Cargo.lock untracked. docs/ untouched.
