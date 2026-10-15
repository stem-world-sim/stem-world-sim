# Repository layout

**Canonical remote:** https://github.com/stem-world-sim/stem-world-sim
**Owners of paths are absolute.** A Bot that writes outside its allowlist has not satisfied the sprint.

**Precedence:** `docs/sprints/CURRENT.md` Oracle allowlist wins for the active sprint. `AGENTS.md`, `docs/bots/ORACLE.md`, and this file describe the default. If they disagree with CURRENT.md, CURRENT.md is the write list for that sprint. Spec must update the other three in the same commit when the allowlist grows.

```
stem-world-sim/
├── README.md                 spec
├── LICENSE                   spec (GPL-3.0; do not replace)
├── AGENTS.md                 spec — standing orders for every Bot
├── docs/
│   ├── REPO_LAYOUT.md        spec — this file
│   ├── vision/               spec — charter, architecture, loop, merge brief
│   ├── spec/                 spec — invariants + acceptance test names
│   ├── sprints/              spec — one file per generation
│   └── bots/                 spec — paste-ready Bot profiles
├── crates/                   ORACLE
│   ├── sim-core/             ORACLE — kernel
│   └── critic/               ORACLE — living critic UI (S17+). Not a per-sprint copy.
├── oracle/
│   └── RUNLOG.md             ORACLE — last run: green | NO SHIP | blocked
└── .github/workflows/        ORACLE — CI that runs cargo test
```

## Who may write what

| Path | Spec chat | Oracle |
|---|---|---|
| `docs/**`, `AGENTS.md`, `README.md`, `LICENSE` | yes | no, unless CURRENT.md allowlists that path |
| `crates/**` named on the CURRENT.md allowlist | no | yes |
| `oracle/**` | no | yes |
| `.github/workflows/**` | no | yes |

Default oracle crate is `crates/sim-core`. Extra crates (the living critic) are allowed only when CURRENT.md lists them. Oracle must not edit `docs/spec/` to make a test easier.

## How the Bot finds work

1. `git pull origin main`
2. Read `docs/sprints/CURRENT.md` — this is the write list
3. Read `docs/spec/SPEC.md` + `docs/spec/ACCEPTANCE.md`
4. Implement only what that sprint lists, only on allowlisted paths
5. `CARGO_TARGET_DIR=/tmp/sws-target cargo test --workspace`
6. Write `oracle/RUNLOG.md`
7. Open PR from `feat/*` if green. Do not push `main`.
