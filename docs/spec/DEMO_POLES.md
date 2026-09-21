# Demo poles

`docs/data/kernel_catalog.csv` `demo=1` is the critic kit (11 taxa).

| Pole | taxon_id | drain_ok |
|---|---|---|
| wetland | `oryza_sativa` | W |
| mesic | `triticum_aestivum` | MD |
| xeric | `hilaria_jamesii` | D |

New moisture / drain / drought / flood tests use these three unless the sprint is about another axis. Do not invent a `D` OccupantParams when galleta exists. Do not recode a famous cactus as `D`.

`d11_catalog_loads_demo_ten` must load all eleven `demo=1` ids. Next kernel sprint that touches D11 may rename the test; until then the name stays and the count is 11.
