# Sprint 12 — Climate envelope filter

**Status:** locked for oracle  
**Clock:** fast. Critic after merge.  
**Depends on:** S11 catalog (`t_min_c`, `t_max_c`, `rain_min_mm`, `rain_max_mm` already parsed).

## Goal

A tile has climate **state**. A catalog taxon with an EcoCrop range dies or fails to plant when the tile is outside that range. A taxon with empty climate fields is not vetoed. Hydro, `P_max`, and soil classes do not change.

Irrigation (tick rain into the column) is **not** the annual climate coordinate.

## Shared state (new)

On the column or world (oracle choice; must be queryable):

- `T_air_c: f64`
- `rain_year_mm: f64` — annual / period depth in **millimeters**, not column `h_surf`.

Commands:

- `set_climate(T_air_c, rain_year_mm)` (or two setters).
- Existing rain command still adds **meters** of water to the column. It must **not** by itself flip `rain_year_mm` unless the test also calls `set_climate`. Do not silently equate a bucket with EcoCrop annual rain.

## Filter

For occupant taxon `p` after plant and after each tick:

- If `p.t_min_c` is Some and `T_air_c < p.t_min_c` → reject `t_min`.
- If `p.t_max_c` is Some and `T_air_c > p.t_max_c` → reject `t_max`.
- If `p.rain_min_mm` is Some and `rain_year_mm < p.rain_min_mm` → reject `r_min`.
- If `p.rain_max_mm` is Some and `rain_year_mm > p.rain_max_mm` → reject `r_max`.
- If the relevant field is None → that clause does not fire.

On reject: occupant does not remain (remove or never insert). Query must expose why (`climate_reject` → `t_min` | `t_max` | `r_min` | `r_max` | none).

Demo rows (read from Catalog::get, do not hardcode in physics):

- `oryza_sativa`: T 16–38, rain 800–4000
- `triticum_aestivum`: T 5–27, rain 300–1600

## Deliverables (oracle)

1. Climate fields + set + queries.
2. Filter using catalog values, not a match on rice.
3. Named d12_* tests.
4. I1 I2 I5 I6 still green. Climate reject is not a mass leak.
5. RUNLOG green or NO SHIP.
6. Do not edit docs/.

## Tests

See docs/spec/ACCEPTANCE.md Sprint 12.

## Non-goals

Degree-days, frost nights, C3/C4, EcoCrop texture as soil, P_max from rain_min, light stack, growth clock.

## Done

CI green. RUNLOG lists the d12_* names.
