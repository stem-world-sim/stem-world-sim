# Sprint 13 — Height-layered light

**Status:** locked for oracle

## Goal

L0=1. Sort occupants tallest-first. Each casts alpha and passes L(1-alpha). light < L_min for T_DARK=7 → light_reject dark. Same alpha sums into S07 ET (min 1).

## Alpha

SLA if present: clamp(0.15, 0.85, 0.55 - 0.01*(S-20)). Else herb 0.25, shrub 0.45, tree 0.60.
Height if null: herb 0.4, shrub 2.0, tree 8.0.
L_min: herb 0.25, shrub 0.15, tree 0.08.
Dead occupants do not cast.

## Tests

d13_grass_alone_full_light
d13_oak_over_grass_shades
d13_grass_dies_in_oak_shade
d13_remove_oak_grass_lives
d13_two_herbs_neither_dark
d13_et_uses_alpha
d13_no_species_name_match

No growth, density, seeds, usda_shade, seasonal L0.
