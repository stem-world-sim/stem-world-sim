# Sprint 19 — nutrient stores

**Status:** locked for oracle
**Depends on:** S18.
**Supersedes:** S18.1 draft. Do not implement S18.1.

A step only advances the clock. No rule reads the step count.

## Clock

Delete the `N_ET` gate. Hydro, evaporation, and every nutrient flux run every step, scaled by `tick_dt`. `catch_up(K)` is K of the same step as a live step. Clock advances by K, not `K * N_ET`.

Rain is `rain_per_day_m`. A step adds `rain_per_day * tick_dt`. The control is mm/day. Zero is a drought.

Seed is one attempt per elapsed day, capped by the tile occupant limit.

## One nutrient system

Light, water, air, and fertilizer are the same kind of thing. A plant holds a store per required nutrient. A step recharges each store from its source and spends that step's use. The plant dies when any required store is empty. Untransformed remainder returns to the source pool.

Nitrogen is one nutrient id. It is not a special case. Adding phosphorus later is a new id, a soil pool, and a demand. No new death branch.

Sources, not hardcoded per nutrient:

- Soil water. The existing hydro state. Feeds the water store.
- Light. The existing canopy light. Charges the energy store.
- Air. A tile or world pool. CO2 for this sprint. Intake stops when the plant is submerged, unless it can take the dissolved form.
- Dissolved in water. A tile pool. Available to a plant with `takes_dissolved` while water is present.
- Soil-bound. A tile pool keyed by nutrient id. Nitrogen for this sprint.

The plant loop iterates the need list. It does not match on the string "nitrogen".

Catalog columns, already on `kernel_catalog.csv`:

- `fertility`: low, moderate, high. Sets nitrogen demand. High spends faster.
- `takes_dissolved`: 1 if the plant can recharge an air nutrient from water. Cattail, rice, mangrove are 1.

Capacity scales with `height_frac` faster than use, so a larger plant lasts longer. SLA may scale the water store (low SLA, thicker leaf, more store). No new measured reserve column.

Initial soil nitrogen is high enough that one plant on a fresh tile does not die of nitrogen in the first days. Depletion is visible with many plants or a long run. On death, remaining nitrogen returns to the soil pool. Remaining water returns to the soil. Remaining CO2 returns to the air. Energy does not become light.

## Wired this sprint

Water, energy, CO2, nitrogen. No phosphorus. No potassium.

Submergence is the CO2 store falling, not a day count. A plant with `takes_dissolved` holds if dissolved CO2 is available. A plant without it does not recharge while submerged and dies when the store is empty.

Waterlog without full submergence does not block air intake.

## Page

Elapsed time only. No tick count. Rain labeled mm/day. Each plant shows each store as a fraction of capacity, and which store is falling. Key lists the nutrient ids.

After green, replace `critic.html.gz` on the `critic` release. Do not commit `dist/`. Do not edit `ci.yml`.

## Tests

- `d19_sinks_every_step` — pond with no plants, tick = 1 day: pond drops on steps 1, 2, and 3.
- `d19_catchup_matches_live` — K live steps and `catch_up(K, 0)` match pond, stores, and elapsed time.
- `d19_dies_when_any_store_empty` — emptying any one required store kills. The others can be full.
- `d19_nutrient_id_is_data` — a test-only nutrient id on the same loop kills when empty. No match arm for that id.
- `d19_submerge_is_co2` — submerged non-dissolved plant dies after its CO2 store empties. Cattail (or rice) holds while dissolved CO2 remains.
- `d19_nitrogen_returns` — uptake lowers the soil pool. Death puts the remainder back.
- `d19_rain_is_per_day` — mm/day times a 2-day step adds two days of rain.
- `d19_seed_one_per_day` — a 3-day step places at most 3 seeds.

Prior suite green. Amend any test that required the 10-step sink batch.

## Non-goals

Phosphorus. Potassium. A climate model. Litter that is not the death return. A map.
