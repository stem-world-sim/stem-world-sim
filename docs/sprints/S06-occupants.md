# Sprint 6 — occupants and plant stub

**Status:** ready for oracle
**Depends on:** S05 green
**Out of scope:** genetics, spread, nutrients, flooding kill, climate field, editing docs/.
**Do not implement shade math this sprint.** Store the field so a later sprint can scale E.

## Why this shape

Do not hard-code “the plant is the top layer.” A cell already has layers and pond. Anything that adds or removes water is an **occupant** with a **root mask** and a **signed flux**. Grass + bush + tree on one tile sum. A taproot later is a different mask, not a new solver. Shade later is a modifier on E, not a second ET path.

## Data

```
MAX_OCCUPANTS = 8
N_LAYERS      = 2   // already true; mask is [f64; N_LAYERS], sum weights = 1

Occupant {
  kind: PlantStub,          // enum; later Animal, Machine, Drip
  alive: bool,
  uptake_max: f64,          // m / sink-step, demand
  root: [f64; N_LAYERS],    // fraction of demand per layer
  shade: f64,               // 0..=1; unused this sprint, must persist
  dry_steps: u32,
}

Cell.occupants: small vec, cap MAX_OCCUPANTS
```

Queries (I6): `plant_at(x,y)`, `occupant_count`, `extract_lost() -> f64`.
API: `add_occupant(x,y, Occupant) -> Result` (err if full); `clear_occupants`.

Stub species `PlantStub`:

```
P_MAX    = 0.01 m / sink-step
T_WILT   = 3 dry sink-steps → alive = false, uptake = 0
root     = [1.0, 0.0]   // top only THIS species, not the architecture
shade    = 0.0
```

A second occupant in a test may use root `[0.0, 1.0]` to prove the mask is not a global “top only” flag.

## Ledger (lock)

Every sink-step, per cell, build a list of signed water deltas then apply:

1. **ET** (existing S05), still pond-then-top. Shade is read but multiplied as `1` this sprint (`E_eff = E * (1 - clamp(sum shade, 0, 1))` with all shade=0 ⇒ identical to S05).
2. **Each alive occupant**, in insert order: demand `u = uptake_max`. For layer k, take `min(u * root[k], θ_k * L)` from that layer (below θ_fc allowed). If total taken == 0, `dry_steps += 1`; else `dry_steps = 0`. If `dry_steps >= T_WILT`, `alive = false`.
3. Occupants do **not** drink pond this sprint.
4. Books: `grid_mass + et_lost + extract_lost == water ever added`.

Positive sources (irrigation, condensation) use the same list later with `+` flux. Do not invent a second water store.

## Clock and cost

Same cadence as ET: after hydro + lake_snap, if `tick % N_ET == 0`: ET then occupants.
Cost O(occupants on the map), not O(world). Bare sleeping cells stay asleep except ET-steps.

## Tests

| name | script |
|---|---|
| `d6_plant_drinks_top` | one PlantStub, wet top; after one sink-step θ_top fell, bot unchanged |
| `d6_mask_can_be_bot` | occupant root [0,1]; θ_bot falls, θ_top unchanged |
| `d6_two_occupants_sum` | two PlantStubs on one cell; extract ≈ 2 * P_MAX (capped by water) |
| `d6_wilt_stops_uptake` | dry top; after 3 sink-steps alive=false; further steps θ unchanged |
| `d6_extract_in_ledger` | M + et_lost + extract_lost conserved |
| `d6_bare_cell_no_extract` | no occupants; extract_lost == 0 |
| `d6_cap_eight` | 9th add_occupant fails |
