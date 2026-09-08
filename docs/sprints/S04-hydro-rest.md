# Sprint 4 — hydro rest

**Status:** ready for oracle
**Depends on:** S03.3.1 green
**Out of scope:** plants, ET, proxies beyond skip-hydro, editing docs/.

## Why

Critic at tick 740: all H in 8.11–8.21, M fixed, soil m frozen — but pond h still walks ±0.02 every tick. Two causes:

1. Pond flux has no deadband. ΔH = 5 cm still moves water.
2. Tick order infiltrate → pond → soil drain pumps lake water into high pores and back out as valley pond. A closed basin never reaches bit-stable rest.

Scalability: distant patches must *sleep*. Rest is the first fidelity gate.

## Locked floors

```
H_rest = 0.02   // metres. Pond edge ignored if ΔH ≤ H_rest
V_rest = 1e-4   // metres water. Soil / infiltrate / drain flux ignored if |V| ≤ V_rest
```

## Rules

1. Pond: drop any candidate edge with ΔH ≤ H_rest before weights. After scale, drop V ≤ V_rest.
2. Soil percolate / lateral: skip if candidate V ≤ V_rest.
3. Infiltrate: skip if take ≤ V_rest.
4. After a tick, cell is `at_rest` iff no pond edge, no soil flux, and no infiltrate exceeded the floors this tick.
5. Next tick: if cell `at_rest` and every 4-neighbor is `at_rest`, skip infiltrate/pond/drain for that cell. Wake if rain, explicit add_water, or a neighbor wakes with ΔH > H_rest.
6. Query: `cell_at_rest(x,y) -> bool` and `grid_at_rest() -> bool` (all cells rest). I6: these are kernel methods.

Mass still conserved. Do not invent sinks.

## Tests

| name | script |
|---|---|
| `d4_pond_below_H_rest_no_flux` | two cells ΔH = 0.01; after tick both h unchanged |
| `d4_drowned_lake_reaches_rest` | 2x3 drowned like critic; N≤100 ticks; then `grid_at_rest()` true; 10 more ticks h frozen |
| `d4_rain_wakes_rest` | rest grid; add rain one cell; that cell not at_rest after wake tick |
| `d4_closed_mass_still_conserved` | rest path still I3 |
