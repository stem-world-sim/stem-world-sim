# Sprint 5 — batched evaporation

**Status:** ready for oracle
**Depends on:** S04.1 green
**Out of scope:** plants, animals, machines, weather field, edge outlets, editing docs/.

## Closed basin (lock)

The grid is a terrarium. Missing 4-neighbors are glass. No flux off the map. Rivers end in lakes. Water leaves only by:

1. Evaporation / chemistry (this sprint).
2. Later extractors (plants, animals, machines) — not this sprint.

I3 becomes: grid mass + cumulative ET = mass ever added. Query `et_lost() -> f64`.

## Rates (constant climate stub)

```
E_open = 0.02   // m per ET-step from pond (h first)
E_soil = 0.005  // m per ET-step from top layer only, may go below θ_fc to 0
N_et   = 10     // ET-step every N_et hydro ticks (batch)
```

On an ET-step, apply `E * 1` (one step), not `E * N_et`. Hydro ticks between ET-steps do not evaporate. N_et is the cadence, not a multiplier. (If climate later stores heat, the ET-step uses accumulated potential; same call site.)

## Rules

1. If h > 0: take min(h, E_open) from pond. Do not also pull soil that step.
2. Else: take min(θ_top * L, E_soil) from top layer. Bot untouched this sprint.
3. Skip a cell if the take would be ≤ V_rest.
4. Sleep: a cell may stay at_rest between ET-steps. An ET-step wakes only cells that will actually lose > V_rest. After ET, re-run lake_snap / rest as S04.1.
5. Tick order: infiltrate → pond → soil → lake_snap → ET (if tick % N_et == 0) → at_rest.
6. No new spatial state. `pub const E_OPEN, E_SOIL, N_ET`.

## Tests

| name | script |
|---|---|
| `d5_no_edge_leak` | pond against map edge; many ticks; M + et_lost conserved; edge cell does not dump off-grid |
| `d5_pond_evaps_before_soil` | h>0 and wet top; one ET-step; h dropped, θ_top unchanged |
| `d5_soil_evaps_below_fc` | h=0, top at θ_fc; enough ET-steps; θ_top < θ_fc |
| `d5_bot_untouched` | top dries; bot stays |
| `d5_batch_not_every_tick` | N_et hydro ticks with no ET-step in between; M unchanged until the ET tick |
| `d5_mass_et_accounts` | rain then ET; M + et_lost == initial + rain |
