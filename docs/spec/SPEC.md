# SPEC – Named invariants and demo predicates

**Version:** 0.2
**Date:** 2026-09-07
**Status:** Fast-clock input. Oracle implements what is named here. Oracle does not edit this file.
**Path:** `docs/spec/SPEC.md` on https://github.com/stem-world-sim/stem-world-sim

ε is a test tolerance, not a physical fudge. Suggested `1e-9` relative for f64 mass. Document ε in the test.

## Active slice

Sprint 1 only (see `docs/sprints/CURRENT.md`). D1–D3 are scheduled, not active.

### Shared quantities (oracle must create these types)

- Column: `elevation_m`, `surface_water_m`, ordered soil layers with volumetric moisture and thickness.
- Optional in Sprint 1: a 1-column world + deterministic clock.
- Water mass of a column, A = 1:

  M = h_surf + sum theta_i * L_i

### Invariants (do not weaken)

**I1** After any legal tick: h_surf >= 0 and 0 <= theta_i <= phi_i. No silent clamp that destroys mass.

**I2** Isolated column, no lateral flux, no sink: rain R + initial M = final M ±ε.

**I3** Isolated grid mass conservation under infiltration and runoff. (Sprint 2+)

**I4** No spontaneous water. (Sprint 2+)

**I5** Same seed + same command script => same moisture and surface fields.

**I6** Win/fail numbers come from kernel queries only.

### Not yet

Evapotranspiration, aquifers, Navier-Stokes, plant genetics, orbits, stoichiometry, heat.

## Demo predicates

**D0 column_rain** — Sprint 1. See ACCEPTANCE.md and S01-column-rain.md.

**D1–D3** — after D0 is green on main.
