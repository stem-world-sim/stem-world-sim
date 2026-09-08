# Sprint 3 — water mechanics

**Status:** ready for oracle
**Depends on:** S02.1 green on main
**Out of scope:** plants, ET, aquifers, Richards PDE, WASM.

## Goal

Water on a slope behaves like water. Pond runs fast. Soil takes water at a texture-limited rate. Water held above field capacity drains downhill (slower than pond). A saturated hill does not stay yellow forever while the valley stays dry.

S02 “theta never moves” is revoked for *mobile* soil water only (θ > θ_fc). Capillary water at or below field capacity stays put.

## Texture table (locked units: metres of water per tick, A = 1)

| Texture | φ | θ_fc / φ | I_max infiltrate | D_max soil drain |
|---|---|---|---|---|
| Sand | 0.40 | 0.50 | 0.20 | 0.04 |
| Loam | 0.45 | 0.50 | 0.05 | 0.01 |
| Clay | 0.50 | 0.50 | 0.01 | 0.002 |

Pond runoff stays S02.1 (half head-drop). That is the fast path — no extra resistance coefficient.

Default texture for `World::new` / unspecified layers: **Sand**, so existing D0 `tick_until` scripts still finish.

## Tick order (locked)

1. Rate-limited infiltrate every column: move `min(h, remaining_pore, I_max)` into layers top-down.
2. Surface runoff (S02.1 unchanged).
3. Gravity drain: for each column, mobile `m = min(D_max, sum ((θ-θ_fc)+ * L))`. Send to the 4-neighbor with **strictly lower z** (lowest z; NESW if tied). Add to that neighbor’s **surface**. If no lower-z neighbor (1×1 or pit), keep m in place.

Do not drain uphill. Do not move water at or below θ_fc.

## Deliverables

1. Texture on layers or columns; I_max, D_max, φ, θ_fc from the table.
2. Tick order above.
3. New ACCEPTANCE names (S03 section). Old names still pass.
4. `oracle/RUNLOG.md` for S03.
5. Do not edit `docs/`.
