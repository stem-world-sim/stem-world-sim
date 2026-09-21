# Acceptance tests (named)

Oracle must implement these as Rust tests. Function names are the contract.

## Sprint 14.1 — catch-up growth

| Rust test name | Fail if |
|---|---|
| `d141_catchup_grows` | ignore + catch_up(20) on a good-condition seedling leaves frac at H0 |
| `d141_matches_live` | live-20 and ignore+catch_up(20) height_frac diverge past d91 tolerance |
| `d141_wilt_block_no_grow` | drought catch_up raises height |
| `d141_ignore_without_catchup_frozen` | ignored cell with no catch_up still grew |

If a sprint adds occupant or column state that tick changes, ACCEPTANCE must name a d*_catchup_* twin or Non-goals must waive catch_up for that field. Silence is a ship blocker.
