# SPEC – Named invariants and demo predicates

**Version:** 0.5
**Date:** 2026-09-08
**Status:** Fast-clock input. Oracle implements what is named here. Oracle does not edit this file.

ε = 1e-9 relative on f64 mass. A = 1.

## Active slice

Sprint 3 — `docs/sprints/CURRENT.md`.

### Stores

- Pond h_surf (fast lateral).
- Soil θ in layers. Mobile only if θ > θ_fc. Capillary θ ≤ θ_fc does not leave the column.
- M = h_surf + sum θ_i L_i. Grid mass = sum M.
- H_pond = z + h_surf (surface routing only).

### Invariants

**I1** h_surf ≥ 0 and 0 ≤ θ_i ≤ φ_i. No silent clamp that destroys mass.
**I2** Isolated column (1×1, no neighbor): rain R + M_0 = M_final ±ε.
**I3** Closed grid: sum M conserved under infiltrate, pond runoff, and soil drain.
**I4** No spontaneous water. No uphill pond. No uphill soil drain.
**I5** Same seed + same script => same fields.
**I6** Win/fail numbers from kernel queries only.

### Tick

1. Infiltrate ≤ I_max(texture), ≤ remaining pore.
2. S02.1 pond runoff (half head-drop, split lowest-H neighbors).
3. Gravity drain of water above θ_fc, ≤ D_max, to strictly lower-z neighbor, onto that neighbor's surface.

Texture table lives in S03-water-mechanics.md.

### Not yet

Plants, ET, aquifers, Navier-Stokes, orbits, heat.
