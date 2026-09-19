# Sprint 11 — Kernel catalog ingest

**Status:** locked for oracle  
**Clock:** fast (data + occupant scalars). Critic demo after merge.  
**Spec files (already on this side):** `docs/data/kernel_catalog.csv`, `docs/data/kernel_catalog_qc.txt`, `docs/data/KERNEL_CATALOG.md`

## Goal

sim-core loads a **small typed catalog**. Occupant scalars come from a row, not from `if name == "oak"`. Missing fields use **form defaults**. No biome column.

## Catalog contract

Path in repo: `docs/data/kernel_catalog.csv` (oracle **may read** this file; do not edit it).

Columns: see `docs/data/KERNEL_CATALOG.md`.

`demo=1` rows (10) are the required fixture for tests and the critic.

Rice root rejected (6 m) → null → herb mask `[1,0]`. Oak `[1,1]`.

## Deliverables (oracle)

1. Load CSV into sim-core. Parse numbers; empty = None.
2. API: `Catalog::get(taxon_id) -> OccupantParams`. Unknown id is an error.
3. Map root_mask: None → herb `[1,0]`, shrub/tree `[1,1]`; depth ≤ 0.20 → `[1,0]`; else `[1,1]`.
4. Parse climate columns; do not apply T/rain filters (S12).
5. P_max / T_wilt stay S07 constants. Do not invent from rain_min.
6. Named tests in ACCEPTANCE.md S11. RUNLOG green or NO SHIP.

## Tests

Always-on I1 I2 I5 I6 plus d11_catalog_loads_demo_ten, d11_unknown_taxon_is_err, d11_rice_root_null_uses_herb_mask, d11_oak_deeper_than_rice_default_mask, d11_no_species_name_match_in_sim_core, d11_same_rain_two_taxa_queryable.

## Non-goals

Light, growth clock, seeds, T/rain kill, loading master sqlite, editing docs/.
