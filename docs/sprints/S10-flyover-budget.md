# Sprint 10 — flyover catch_up budget

**Status:** ready for oracle
**Depends on:** S09.1 green
**Out of scope:** renderer, streaming IO, networking, editing docs/.

## Why this is not “just more tests”

S09 proved ignore + scoped catch_up on 16×16. That does not tell us whether a 1 km map can accept a camera flying at 16 m/s. S10 is a **wall-clock budget** on a 1024×1024 grid: catch_up only the chunks in a 32×32 view plus a 32 m look-ahead band, for hundreds of missed ticks, with time left before those tiles enter the view.

No streaming crate. Selection is an in-memory chunk set.

## Units

| Symbol | Value |
|---|---|
| cell | 1 m × 1 m |
| CHUNK | 8 cells (already S09) |
| world | 1024 × 1024 cells |
| view frustum | 32 × 32 cells |
| fly speed | 16 m/s along +x |
| look-ahead | 32 m = 2 s of flight |
| K_nom | 30 sink-steps (300 calendar ticks) |
| K_stress | 100 sink-steps (1000 calendar ticks) |

Time to fly one chunk: 8 m / 16 m/s = **0.5 s**. That is the hard cap for catching up the *incoming stripe* (32×8 cells = 4 chunks).

## Topology

Ridge: `z=8` for `x ∈ [128,144)`, else `z=2`. Texture loam. θ = θ_fc everywhere (rest desert) except the ridge/valley band may hold a pond for the storm test. One PlantStub in the starting view.

## Camera set

Origin `(ox,oy) = (16, 496)` so view is centered on the map in y.

```
view      = [ox, ox+32) × [oy, oy+32)
look-ahead= [ox+32, ox+64) × [oy, oy+32)
work set  = chunks intersecting view ∪ look-ahead
```

Ignore every other chunk.

## Budget (oracle VM, single thread, `Instant`)

Record all times in RUNLOG.

| name | work | pass |
|---|---|---|
| `d10_map_constructs` | `World` 1024×1024 | returns; print RSS or cell count |
| `d10_ignored_tick_zero_visits` | all chunks ignored, rest; 1 tick | `last_hydro_visits==0`; wall < 5 ms |
| `d10_frustum_not_world` | observe work set only; rain in view; 1 tick | visits ≤ cells in work set + 1-cell halo, **not** 1024² |
| `d10_stripe_k30` | `catch_up_chunk` each of the 4 incoming-stripe chunks, K=30, R=0 | wall **< 50 ms** (10× spare vs 0.5 s) |
| `d10_workset_k100` | catch_up all work-set chunks, K=100, R=0 | wall **< 500 ms** |
| `d10_storm_k30_still_under_fly` | add 0.3 m pond on ridge cells in look-ahead; catch_up those chunks K=30 | wall **< 500 ms**; plant in view not required to match live (budget test) |

If 1024×1024 cannot allocate on the VM, do **not** silently shrink. Fail `d10_map_constructs` with the error. Spec will resize next sprint.

## Queries

Need `last_hydro_visits()` (S09). Add `catch_up_chunks(&[chunk_id], K, rain)` or loop `catch_up_chunk` — either is fine. Print chunk count in the work set (expect 32 cells / 8 = 4 by 8 = **8 chunks** for view+look-ahead 64×32).

## Not this sprint

Camera interpolation, frustum culling in a GPU, paging tiles from disk, multithreading. Those wait until these numbers exist.
