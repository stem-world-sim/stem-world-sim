# Acceptance tests (named)

Oracle must implement these as Rust tests. **Function names are the contract.**  
`cargo test --workspace` must print these names. Missing name = sprint not done.

ε: document in the test (suggested `1e-9` relative on f64 water mass). Not a UI constant.

Cell area \(A = 1\) implicit until SPEC names otherwise.  
Column water mass:

\[
M = h_{\text{surf}} + \sum_i \theta_i L_i
\]

---

## Always on (every sprint after they first appear)

| Rust test name | Invariant | Fail if |
|---|---|---|
| `i1_moisture_and_surface_nonnegative` | I1 | any \(h_{\text{surf}} < 0\) or \(\theta \notin [0,\phi]\) after a legal tick |
| `i2_isolated_column_mass_conserved` | I2 | \(\lvert M_{\text{final}} - (M_0 + R)\rvert > \varepsilon\) with no lateral flux, no sink |
| `i5_same_seed_same_script_same_state` | I5 | two runs differ in moisture or surface fields |
| `i6_queries_are_kernel_methods` | I6 | a test asserts on a number that is not returned by `sim_core` |

I3 and I4 are **not** Sprint 1 (they need a grid + runoff).

---

## Sprint 1 only — D0 `column_rain`

| Rust test name | Script | Fail if |
|---|---|---|
| `d0_rain_fits_in_pores_surface_dries` | A: \(R <\) remaining pore space; tick until surface dry | soil \(\Delta M \neq R \pm\varepsilon\) or \(h_{\text{surf}} \neq 0\) |
| `d0_rain_exceeds_pores_remainder_ponds` | B: \(R >\) remaining pore space | \(\theta \neq \phi\) or \(h_{\text{surf}} \neq R - \text{remaining pore} \pm\varepsilon\) |
| `d0_novice_all_rain_vanishes_is_false` | same setup as B | a helper that assumes “dirt eats all rain” **passes** (the test must show that model is wrong) |

No plants, no runoff, no evaporation, no orbits.

---

## Sprint 11 — kernel catalog

| Rust test name | Fail if |
|---|---|
| `d11_catalog_loads_demo_ten` | any of the ten `demo=1` taxon_ids missing after load |
| `d11_unknown_taxon_is_err` | `get("not_a_plant")` succeeds |
| `d11_rice_root_null_uses_herb_mask` | `oryza_sativa` root_mask is not `[1,0]` |
| `d11_oak_deeper_than_rice_default_mask` | `quercus_alba` root_mask is not `[1,1]` |
| `d11_no_species_name_match_in_sim_core` | occupant params for demo ids are assigned by a Rust match on English names |
| `d11_same_rain_two_taxa_queryable` | after plant maize + cattail, rain, tick, kernel cannot query both ids and catalog heights |

Always-on I1 I2 I5 I6 still required.

---

## Sprint 12 — climate envelope

| Rust test name | Fail if |
|---|---|
| `d12_rice_dies_below_tmin` | rice remains at T=10 °C, 1000 mm, or reject is not `t_min` |
| `d12_wheat_lives_at_same_t` | wheat gone at T=10 °C, 1000 mm |
| `d12_rice_lives_in_envelope` | rice gone at T=22 °C, 1200 mm |
| `d12_wheat_dies_above_tmax` | wheat remains at T=32 °C, or reject is not `t_max` |
| `d12_rice_dies_dry_year` | **AMENDED S12.1:** rice gone at 22 °C / 200 mm on a live tick (must remain; year rain is not a live veto) |
| `d12_bucket_is_not_climate` | **AMENDED S12.1:** 0.05 m rain changes `rain_year_mm`, or rice is removed at T=22 |
| `d12_empty_envelope_no_veto` | a taxon with empty T/rain dies from climate at T=-40 |
| `d12_no_species_name_match` | filter branches on English / taxon string instead of catalog ranges |

---

## Sprint 12.1 — current conditions + event hydro

| Rust test name | Fail if |
|---|---|
| `d121_plant_rice_in_dry_year_if_warm` | rice gone at T=22, rain_year=200 on a live tick |
| `d121_rice_dies_when_air_turns_cold` | rice still present after T_air set to 10 °C |
| `d121_wheat_waterlogs_in_pond` | wheat remains after T_WL ponded ticks, or rice gone |
| `d121_rice_pond_no_waterlog` | rice gone in shallow pond for T_WL |
| `d121_wheat_submerged_over_height` | wheat remains when pond ≥ catalog height for T_SUB |
| `d121_oak_not_submerged_in_shallow_pond` | oak gone at h_surf=0.5 m |
| `d121_drought_kills_at_good_rain_year` | plant lives T_wilt dry ticks at rain_year=1000, or year mm changes |
| `d121_no_species_name_match` | flood rules branch on taxon English / id string |

---

## Sprint 13 — height-layered light

| Rust test name | Fail if |
|---|---|
| `d13_grass_alone_full_light` | ryegrass light is not ~1, or it dies in 7 ticks alone |
| `d13_oak_over_grass_shades` | oak is not first in the stack, or grass light ≠ \(1-\alpha_{\mathrm{oak}}\) |
| `d13_grass_dies_in_oak_shade` | grass remains after T_DARK under oak, or oak dies |
| `d13_remove_oak_grass_lives` | grass dies at full light after oak is removed |
| `d13_two_herbs_neither_dark` | two similar-height herbs produce a dark kill in 7 ticks |
| `d13_et_uses_alpha` | pond ET ignores occupant \(\alpha\) |
| `d13_no_species_name_match` | light path branches on English / taxon id string |

---

## Sprint 13.1 — light bands

| Rust test name | Fail if |
|---|---|
| `d131_two_oaks_same_light` | two oaks have different `light` |
| `d131_two_oaks_darker_grass_than_one` | two-oak grass L ≥ one-oak grass L |
| `d131_no_id_order_shade` | insertion order changes oak lights |
| `d131_no_species_name_match` | band walk branches on taxon strings |

`d13_two_herbs_neither_dark` must use two ryegrass; both L≈1.

---

## Sprint 13.2 — overlap cover

| Rust test name | Fail if |
|---|---|
| `d132_understory_never_zero` | any living occupant has light 0 |
| `d132_eight_oaks_hits_cap` | eight oaks drive T to 0 or C ≠ C_MAX |
| `d132_two_oaks_not_black` | two-oak grass light is 0 |
| `d132_no_species_name_match` | overlap branches on taxon strings |

---

## Sprint 14 — growth clock

| Rust test name | Fail if |
|---|---|
| `d14_plant_taxon_is_mature` | plant_taxon oak height_frac is not ~1 |
| `d14_seedling_starts_short` | seedling oak height_frac is not ~0.10 |
| `d14_seedling_oak_grass_lives` | grass dies in 7 ticks under one seedling oak |
| `d14_intolerant_prefers_sun` | I pine in shade gains as much or more than full sun |
| `d14_tolerant_prefers_gap` | T ryegrass at L=1 gains as much or more than under one oak |
| `d14_grown_intolerant_reaches_mature` | I pine at L=1 for T_MATURE tree ticks is not ~1 |
| `d14_mature_oak_still_shades` | mature oak no longer dark-kills grass |
| `d14_wilt_stops_growth` | dry seedling gains height |
| `d14_cool_grows_slower_than_opt` | T at t_min grows as fast as T_opt |
| `d14_no_species_name_match` | growth clock branches on taxon strings |

---

## Sprint 14.1 — catch-up growth

| Rust test name | Fail if |
|---|---|
| `d141_catchup_grows` | ignore + catch_up(20) on a good-condition seedling leaves frac at H0 |
| `d141_matches_live` | live-20 and ignore+catch_up(20) height_frac diverge past d91 tolerance |
| `d141_wilt_block_no_grow` | drought catch_up raises height |
| `d141_ignore_without_catchup_frozen` | ignored cell with no catch_up still grew |

---

## After a sprint ships

Do not delete tests. Later sprints only **add** rows to this file (spec chat) and add functions (oracle).

If a sprint adds occupant or column state that `tick` changes, ACCEPTANCE must name a `d*_catchup_*` twin **or** Non-goals must say catch_up does not advance that field. Silence is a ship blocker.
