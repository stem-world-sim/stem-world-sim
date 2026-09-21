# Sprint 15 — band compress and thin

**Status:** locked for oracle

Band load: occupant cover a=α·height_frac² in the S13.1 band of h_eff only (crown, not every band below). Herbs load their low band. Stem area neglected.
If A_b>1: s_b=1/A_b; scale α_eff and uptake in that band. S13.2 product/caps after compress.
T_CROWD=7. Consecutive ticks with A_b>1 → remove occupant in that band with lowest f_L·f_w·f_T·height_frac. Tie: lower frac, then index. Reset counter when A_b≤1.
Catch_up applies compress + crowd counter. No litter/decompose/N.
Rider: d11 loads eleven demo=1 ids including hilaria_jamesii.

Tests: d15_two_seedling_oaks_both_live, d15_two_mature_oaks_compress, d15_two_mature_oaks_thin, d15_grass_under_one_oak_lives, d15_weaker_loses, d15_catchup_thins_same_as_live, d15_no_species_name_match.
No docs/ edits.
