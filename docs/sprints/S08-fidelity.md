# Sprint 8 — awake set and catch-up

**Status:** ready for oracle
**Depends on:** S07 green
**Out of scope:** chunks UI, weather field, genetics, editing docs/.

## Goal

Sleep is not a museum. Unobserved time still applies climate and sinks. We do not run hydro on every tile every tick.

## Grid

World is `W × H` (existing 2×3 demos stay valid). Tests must include a larger rest grid (16×16 or similar).

## Awake set

Maintain the set of cells that may run hydro / sinks this tick.

- Rain, `add_occupant`, catch_up, or a neighbor sending pond/soil flux **wakes** the cell and its 4-neighbors.
- After a tick, a cell with `at_rest` and all 4-neighbors `at_rest` **leaves** the set.
- `tick` visits **only** the awake set. A 16×16 desert after rest: hydro visits == 0.

Query: `awake_count() -> usize`.

## Catch-up (facsimile, not magic)

`catch_up(K, rain_per_step)` with `K` = number of **sink-steps** skipped (`K * N_ET` hydro ticks of calendar time).

Legal on the whole world this sprint (region flags later). Order:

1. **Source:** every cell `h += K * rain_per_step`. Wake those cells. `rain_per_step` may be 0 (drought).
2. **Settle:** live hydro ticks until `awake_count()==0` or `T_SETTLE=64` ticks. Same pond/soil/snap as live. This is how rain on a hill reaches the pond — not a teleport.
3. **Sinks:** one batched ET+occupants pass using **K** as multiplier on `E_eff` and `P_max` (pond first, leftover to top; plants pull `K * P_max` from the root mask, cap by water). `dry_steps +=` 1 per fully dry batch toward `T_WILT` (if `K>=T_WILT` and no water, wilt). Shade from living occupants still scales `E`.
4. Ledger: added rain counts as water-ever-added; ET and extract use the batched volumes.

Interleaved rain→settle→ET, K times, is the reference. All-at-once rain→settle→K-sink is the allowed facsimile. Mass after catch_up must match the reference within 5% relative on a closed rest basin test; exact match required when `rain_per_step==0` (sinks only).

`T_SETTLE` exceeded: leave cells awake (honest “still moving”), do not invent a drained state.

## Tests

| name | script |
|---|---|
| `d8_desert_sleeps_zero_awake` | 16×16 all rest; tick; awake_count==0 |
| `d8_rain_wakes_neighbors` | rain one cell; it and 4-neighbors awake |
| `d8_drought_wilt` | planted rest cell, catch_up(K=5, rain=0); plant wilted; extract and ET in ledger |
| `d8_rain_fills_then_plants` | rest valley + plant, catch_up(K=3, rain>0); h or θ rose then sinks applied; mass books close |
| `d8_drought_matches_live_sinks` | rain=0; catch_up(K) vs K live sink-steps; θ,h,et,extract equal within 1e-9 |
| `d8_no_teleport_uphill` | rain only on high; after catch_up valley has water (settle routed it) |
