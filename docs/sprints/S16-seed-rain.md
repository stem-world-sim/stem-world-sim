# Sprint 16 — seed rain

**Status:** locked for oracle

Fertile: living, not wilted, height_frac >= F=0.5.
One attempt per fertile occupant per tick after growth/compress/thin.
Target order N,E,S,W, same cell. First slot under MAX_OCCUPANTS.

dispersal:
- wind or animal: neighbors + same
- water: same, or neighbor with pond or lower z
- short / empty / other: same cell only

Seedling: same taxon, height_frac=H0. Then normal S12–S15 filters. No RNG. Catch_up runs the same attempts.

Tests: d16_fertile_pine_seeds_neighbor, d16_seedling_not_fertile, d16_short_stays_home, d16_oak_seed_on_crowded_tile_dies, d16_water_no_uphill_dry, d16_catchup_seeds_same_as_live, d16_no_species_name_match.
No docs/ edits.
