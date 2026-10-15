# Sprint 17 — critic on sim-core

**Status:** locked for oracle
**Depends on:** S16.3 canopy layers.
**Does not change:** seed pulse, MAX_OCCUPANTS, layer rule, herb bands.

## Problem

HTML demos reimplemented physics and drifted. The critic must call the kernel. Traces are not enough: the critic has to plant and tick cases the tests did not name.

## Deliver

New crate `critic` in the workspace. It depends on `sim-core` only. No physics in the UI.

Stable API the UI and the tests both call:

- plant(taxon) on the single tile
- tick() and tick_n(k)
- query_tile() → living count, cap, and per occupant: taxon, height_m, height_frac, layer_id, layer_kind (woody|herb), A, over, light, alive

`critic` is an egui window: plant buttons for demo taxa already in the kernel catalog (pine, oak, rye at minimum), Tick, Tick×N, Play/Pause, K, and a layer list from query_tile. One tile. No map. No world mesh. No camera.

Targets:

- Native: `cargo run -p critic` builds.
- Wasm: a wasm build of the same UI, loadable as a page. Put the built page where a phone can open it (`docs/demo/critic/` is fine). Document the one command that rebuilds it.

The UI may format text. It must not decide who shares a layer, who thins, or what light is. Those come from the query.

## Tests

Tests call the same API the UI calls:

- d17_two_mature_pines_thin — two mature pines, after T_CROWD one remains.
- d17_midstory_separate_layer — 13.1 m and ~8.6 m pines, different layer_id, both remain after T_CROWD.
- d17_herb_band_thins — two mature rye, after T_CROWD one remains.
- d17_query_matches_kernel — query_tile layer_id and A match the S16.3 / S15 rules on a fixed setup.
- d17_no_species_name_match
- Prior suite green.

## Non-goals

Seed storm, seasons, lifespan, raising MAX_OCCUPANTS, neighbor canopy, a renderer, Bevy, a grid larger than one tile.

## Deliverables

`critic` crate + wasm page + tests + RUNLOG. No hand-written second physics. Docs only if the wasm page must live under `docs/demo/` — that path is allowed for the built page, not for a JS port of the rules.
