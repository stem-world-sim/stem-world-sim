# Sprint 2 — D1 slope runoff

**Status:** ready for oracle
**Clock:** fast (physics). Critic plays two cells after merge.
**Depends on:** S01 green on main.

## Goal

Ponded surface water can leave a high column and appear on a lower neighbor. Isolated dry cells stay dry. Grid mass is conserved.

## Routing (locked)

- World is a regular 4-neighbor grid of columns. S01 1-column world is grid 1×1.
- Tick order: infiltrate every column (S01 capacity-step), then one runoff pass.
- Only **surface** water moves laterally. Soil theta does not jump sideways.
- Head H_i = elevation_m + surface_water_m.
- A column may send surface water only to a neighbor with **strictly lower** H.
- If several neighbors qualify, pick the lowest H; if tied, N, E, S, W in that order.
- Volume moved this tick from i to nbr: min(h_i, H_i - H_nbr). All of that min moves (no extra rate constant).
- Closed boundary: no flux off the grid.

## Deliverables

1. Grid World with column-at-(x,y) queries and grid total mass.
2. `tick` = infiltrate all, then runoff pass.
3. Exact new test names in docs/spec/ACCEPTANCE.md (S02 section).
4. All S01 test names still pass, unchanged names.
5. oracle/RUNLOG.md for S02.

## Out of scope

Plants, ET, groundwater, D8/SWE/diffusion PDE, WASM/UI, editing docs/.
