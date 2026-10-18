# S18 — telemetry, continuous rain, text size, unified clock

Critic feedback (2026-10-18):
- Oak dried between rain events until roots reached the next soil layer.
- Text size too small for some eyesight; add slider.
- Manual 10 mm rain button is gardener work; need rain X mm per tick so trees can grow without constant clicking. 10 mm is not enough for multi-month tree growth.
- Time scales are inconsistent (tick × K vs real growth). Design UI to set tick duration (1 s for water propagation, 1 day for grass/ponds, 1 month or 1 year for trees). Everything shares one clock; rates scale with dt.

## Goals

- Kernel queries expose current root depth / max accessible soil layer per occupant (grows with plant).
- Critic displays it in telemetry.
- Continuous rain: settable mm per tick (or rate), applied automatically on each tick (manual button remains).
- Text-size slider in critic UI (egui style scale).
- Unified time: `tick_dt` setting (seconds, days, months, years). All process rates (growth, ET, infiltration, etc.) are physical and scale with dt. K ticks advance K × dt of real time. Default preserves prior tick behavior.

## Non-goals

- Full climate model (this is the substitute until then).
- Multi-tile or spatial.
- Changing existing stress thresholds or catalog values.

## Oracle allowlist

`crates/sim-core/**`, `crates/critic/**`, `oracle/RUNLOG.md`

## Named tests (add to ACCEPTANCE.md)

- `d18_root_depth_queryable`
- `d18_oak_root_grows_to_next_layer`
- `d18_continuous_rain_applied_each_tick`
- `d18_tick_dt_scales_growth_and_et`
- `d18_mass_conserved_under_scaled_dt`
- `d18_default_dt_matches_prior_behavior`

Catch-up twin required if tick advances new state: `d18_catchup_root_and_rain` or document that catch_up does not advance root/rain counters.
