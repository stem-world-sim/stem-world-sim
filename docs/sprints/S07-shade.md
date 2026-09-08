# Sprint 7 — shade scales ET

**Status:** ready for oracle
**Depends on:** S06 green
**Out of scope:** LAI, light, diurnal, proxy plots, genetics, editing docs/.

## Rule

On a sink-step, before ET:

```
S = sum(occ.shade for occ in cell.occupants if occ.alive)
S = clamp(S, 0, 1)
E_open_eff = E_OPEN * (1 - S)
E_soil_eff = E_SOIL * (1 - S)
```

Then S05 ET using the effective rates (pond first, else top). Occupant uptake is unchanged.

Dead / wilted occupants contribute 0 shade.

## Stub constant

PlantStub `shade = 0.25` (was 0). One living plant cuts ET 25% on that tile. Two → 50%. Four → 100% (ET off, uptake still runs).

## Tests

| name | script |
|---|---|
| `d7_bare_et_unchanged` | no occupants; one ET-step; pond loss == E_OPEN |
| `d7_one_plant_cuts_pond_et` | one alive PlantStub, h>0; pond loss == 0.75 * E_OPEN |
| `d7_two_sum` | two alive; pond loss == 0.50 * E_OPEN |
| `d7_shade_cap_one` | four alive; pond loss == 0 |
| `d7_wilt_no_shade` | wilted occupant; pond loss == E_OPEN |
| `d7_uptake_unchanged` | planted, wet top, h=0; extract still P_MAX |
| `d7_ledger` | M + et_lost + extract_lost conserved |
