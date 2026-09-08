# SPEC – Named invariants and demo predicates

**Version:** 0.4
**Date:** 2026-09-08
**Status:** Fast-clock input. Oracle implements what is named here. Oracle does not edit this file.

ε suggested 1e-9 relative for f64 mass. A = 1.

## Active slice

Sprint 2.1 (see `docs/sprints/CURRENT.md`).

### Shared quantities

- Grid of columns. Each column: elevation_m, surface_water_m, soil layers (theta, L, phi).
- Column mass M = h_surf + sum theta_i * L_i
- Grid mass = sum of column M
- Head H = elevation_m + surface_water_m

### Invariants (do not weaken)

**I1** After any legal tick: h_surf >= 0 and 0 <= theta_i <= phi_i. No silent clamp that destroys mass.

**I2** Isolated column, no lateral flux, no sink: rain R + initial M = final M ±ε.

**I3** Closed grid, no sink: sum M conserved under infiltration and runoff.

**I4** A cell does not gain water without rain, lateral inflow, or a documented command.

**I5** Same seed + same command script => same moisture and surface fields.

**I6** Win/fail numbers come from kernel queries only.

### Surface routing (S02.1)

Tick: infiltrate all columns, then one simultaneous runoff pass. Surface only.
A cell may send only to 4-neighbors with strictly lower H.
Among those, take the set T with the lowest H and split
V = min(h_i, 0.5 * (H_i - H*)) equally across T.
Closed boundary.

### Not yet

Evapotranspiration, aquifers, Navier-Stokes, plant genetics, orbits, stoichiometry, heat.

## Demo predicates

**D0 column_rain** — S01, still must pass.
**D1 slope_runoff** — S02 + S02.1 pooling.
**D2 plant_thirst**, **D3 proxy_farm** — later.
