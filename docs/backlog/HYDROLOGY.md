# Hydrology tickets (not active)

Do not put these in CURRENT.md until a sprint is written. Oracle ignores this folder.

Today: infiltration is an instant capacity fill. Lateral move is surface pond only.

## T-HYD-01 — finite infiltration rate

A tick must not empty the whole remaining pore space unless the rate allows it.
Texture (or K_sat) caps how much pond becomes theta this tick.
Clay: small cap, pond while soil is still wetting. Sand: large cap.

## T-HYD-02 — two clocks for lateral water

Pond (h) uses S02.1 head-equalize (fast).
Soil theta does not move sideways until a sprint names it.
Lesson: flood on wet ground is few ticks; wetting a dry bed takes more.

## T-HYD-03 — texture as shared state

Layers already have phi. Add a texture tag or K that sets infiltrate cap.
Clay can have high phi and still take water slowly.

| texture | phi | relative cap / tick |
|---|---|---|
| sand | ~0.40 | 1.0 |
| loam | ~0.45 | 0.3 |
| clay | ~0.50 | 0.05 |

## T-HYD-04 — critic colors

Tile color by theta/phi (green to yellow), pond separately (blue). Visual is on docs/demo/d1.html. Kernel rates are not.
