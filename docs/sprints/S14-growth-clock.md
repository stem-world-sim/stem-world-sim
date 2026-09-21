# Sprint 14 — Growth clock

**Status:** locked (amended: fraction of optimal)

plant_taxon height_frac=1. Seedling H0=0.10.
alpha_eff = alpha * height_frac^2.
T_MATURE herb/shrub/tree 20/80/200. Δ0=(1-H0)/T_MATURE.
Each tick: frac += Δ0 * f_L * f_w * f_T.
f_L = 0 if L < L_min else L.
f_w = 0 if wilted else clamp((th-pwp)/(fc-pwp)) on rooted layers.
f_T = triangular in [t_min,t_max] peak at midpoint; 1 if no envelope.
No nutrients.

Tests: d14_plant_taxon_is_mature, d14_seedling_starts_short, d14_seedling_oak_grass_lives, d14_full_sun_grows_faster_than_shade, d14_grown_oak_shades_grass, d14_wilt_stops_growth, d14_cool_grows_slower_than_opt, d14_no_species_name_match.
