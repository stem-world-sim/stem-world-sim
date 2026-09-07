# Repository layout

**Canonical remote:** https://github.com/stem-world-sim/stem-world-sim
**Owners of paths are absolute.** A Bot that writes outside its allowlist has not satisfied the sprint.

```
stem-world-sim/
├── README.md                 spec
├── LICENSE                   spec (GPL-3.0; do not replace)
├── AGENTS.md                 spec — standing orders for every Bot
├── docs/
│   ├── REPO_LAYOUT.md        spec — this file
│   ├── vision/               spec — charter, architecture, loop, merge brief
│   ├── spec/                 spec — invariants + acceptance test names
│   └── sprints/              spec — one file per generation
├── crates/                   ORACLE
│   └── sim-core/             ORACLE — only kernel package
├── oracle/                   ORACLE
│   └── RUNLOG.md             ORACLE — last run: green | NO SHIP | blocked
└── .github/workflows/        ORACLE — CI that runs cargo test
```

## Who may write what

| Path | Spec chat (`stem-world-sim-spec`) | Grok Bot oracle |
|---|---|---|
| `docs/**`, `AGENTS.md`, `README.md`, `LICENSE` | yes (PR) | **no** |
| `crates/sim-core/**` | no | yes |
| `oracle/**` | no | yes |
| `.github/workflows/**` | no | yes |

Oracle **must not** edit `docs/spec/` to make a test easier.

## What the oracle puts in `crates/sim-core`

Required shape after Sprint 1 (names exact):

```
crates/sim-core/
├── Cargo.toml          package name: sim_core
├── src/
│   └── lib.rs          public kernel API only
└── tests/
    └── acceptance.rs   implements docs/spec/ACCEPTANCE.md names
```

Workspace root `Cargo.toml` (oracle creates):

```toml
[workspace]
resolver = "2"
members = ["crates/sim-core"]
```

No second package. No client crate in Sprint 1. No rendering dependencies in `sim-core`.

## How the Bot finds work

1. `git pull origin main`
2. Read `docs/sprints/CURRENT.md`
3. Read `docs/spec/SPEC.md` + `docs/spec/ACCEPTANCE.md`
4. Implement only what that sprint lists
5. `CARGO_TARGET_DIR=/tmp/sws-target cargo test --workspace`
6. Write `oracle/RUNLOG.md`
7. Open PR from `feat/*` if green. Do not push `main`. Do not edit spec files.
