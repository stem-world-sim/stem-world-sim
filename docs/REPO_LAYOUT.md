# Repository layout

**Canonical remote:** https://github.com/stem-world-sim/stem-world-sim

Shared rules: `AGENTS.md`. Bot procedure: `docs/bots/<bot>.md`. This file is the tree only.

```
stem-world-sim/
├── README.md                 spec
├── LICENSE                   spec (GPL-3.0; do not replace)
├── AGENTS.md                 spec — shared rules only
├── docs/
│   ├── REPO_LAYOUT.md        spec — this file
│   ├── vision/               spec — charter, architecture, loop
│   ├── spec/                 spec — invariants + acceptance test names
│   ├── sprints/              spec — CURRENT.md holds the active allowlist
│   └── bots/                 spec — one procedure file per bot
├── crates/                   oracle, paths named on the CURRENT.md allowlist
│   ├── sim-core/             kernel
│   └── critic/               living critic UI (S17+). One crate, not a per-sprint copy.
├── oracle/
│   └── RUNLOG.md             last run: green | NO SHIP | blocked
└── .github/workflows/        CI that runs cargo test
```
