# Sprint 14 — Growth clock

**Status:** locked (f_L from shade class)

plant_taxon height_frac=1. Seedling H0=0.10. alpha_eff=alpha*height_frac^2.
T_MATURE herb/shrub/tree 20/80/200. Δ0=(1-H0)/T_MATURE.
frac += Δ0 * f_L * f_w * f_T.

Catalog column shade: I/M/T/empty (USDA join; 233/451 filled).
L_opt: I or empty=1.00; M=0.60; T=0.35.
L_min remains form dark-kill (not L_opt).
f_L=0 if L<L_min else clamp(1-|L-L_opt|,0,1).
f_w wilted=0 else θ between pwp and fc.
f_T triangular in envelope; 1 if missing.

Tests: d14_plant_taxon_is_mature, d14_seedling_starts_short, d14_seedling_oak_grass_lives, d14_intolerant_prefers_sun (pinus I), d14_tolerant_prefers_gap (ryegrass T under one oak), d14_grown_intolerant_reaches_mature, d14_mature_oak_still_shades, d14_wilt_stops_growth, d14_cool_grows_slower_than_opt, d14_no_species_name_match.

Oracle reads kernel_catalog; if shade column missing treat empty.
