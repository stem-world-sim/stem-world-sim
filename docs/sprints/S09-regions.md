# Sprint 9 — region observe

**Status:** ready for oracle
**Depends on:** S08 green
**Out of scope:** GPU renderer, weather field, genetics, editing docs/.

## Chunks

```
CHUNK = 8
chunk_id(x,y) = (x/CHUNK, y/CHUNK)
```

Each chunk: `observed: bool` (default true for W*H <= 12 so old tests stay live), `t_away: u64`.

API: `observe_chunk(cx,cy)`, `ignore_chunk(cx,cy)`, `catch_up_chunk(cx,cy,K,rain_per_step)`.
Queries: `observed_chunk_count`, `awake_count` (existing).

## Live tick

Hydro + sinks run only on cells in **observed chunks ∪ awake set**. Ignoring a rest chunk must not visit its cells next tick (`d9_ignore_zero_visits`).

Ignore is legal only if every cell in the chunk is `at_rest`. Else `Err` or no-op + stay observed.

## Scoped catch-up

`catch_up_chunk` = S08 order **on that chunk plus a 1-cell halo** (so pond can leave the edge). Other chunks unchanged. Halo cells that receive water become awake (may wake a neighbor chunk).

## Tests

| name | script |
|---|---|
| `d9_ignore_zero_visits` | 16×16 rest; ignore all; tick; hydro cell visits == 0 |
| `d9_observe_one_chunk_only` | ignore all but (0,0); rain inside (0,0); only that chunk + halo awake |
| `d9_cannot_ignore_moving` | pond still flowing; ignore_chunk fails or stays observed |
| `d9_catchup_other_chunk_untouched` | two chunks; catch_up_chunk A; B mass unchanged |
| `d9_drought_chunk_wilts` | plant in A, ignore A, catch_up_chunk(A,K=5,rain=0); plant wilted; B intact |
| `d9_halo_can_export_pond` | rain catch-up on high chunk; adjacent valley chunk gains water |
