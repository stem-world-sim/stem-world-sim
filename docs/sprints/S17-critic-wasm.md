# Sprint 17 — critic on sim-core

**Status:** merged, PR #76, `d096b6b`
**Depends on:** S16.3 canopy layers.
**Does not change:** seed pulse, MAX_OCCUPANTS, layer rule, herb bands.

## Closed

`crates/critic` is the living critic. UI calls `plant` / `tick` / `query_tile` only.

Wasm is `trunk build` on the oracle machine. Do not add a web CI job. `ci.yml` stays native `cargo test`. Do not commit `dist/`.
