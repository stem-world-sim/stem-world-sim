# Sprint 14 — Growth clock

**Status:** locked for oracle

plant_taxon remains height_frac=1 (S13 tests unchanged). Seedling API starts at H0=0.10.
effective_height = height_frac * H_mat.
alpha_eff = alpha * height_frac^2.
T_MATURE: herb 20, shrub 80, tree 200.
Good tick (not wilted, light>=L_min): frac += (1-H0)/T_MATURE.
Wilt or dark: no growth.

Tests: d14_plant_taxon_is_mature, d14_seedling_starts_short, d14_seedling_oak_grass_lives, d14_grown_oak_shades_grass, d14_wilt_stops_growth, d14_dark_stops_growth, d14_no_species_name_match.
