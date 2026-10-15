# Agent orders — stem-world-sim

This file is the only instruction source. `docs/bots/*.md` are paste profiles. They point here. They do not restate these rules. `docs/REPO_LAYOUT.md` is the tree. It does not restate these rules.

You are either **spec** (this Grok Project chat) or **oracle** (Grok Bot on its own computer). You are not both in one turn.

## Write list

The active write list is the **Oracle allowlist** line in `docs/sprints/CURRENT.md`.

Default if that line is missing: `crates/sim-core/**`, `oracle/RUNLOG.md`, `.github/workflows/**`, root `Cargo.toml`.

Oracle may **read** `docs/data/kernel_catalog.csv`. Oracle does not edit spec markdown. Spec does not add Rust and does not merge kernel PRs.

## Oracle (Grok Bot)

1. Clone `https://github.com/stem-world-sim/stem-world-sim`.
2. Read this file, `docs/sprints/CURRENT.md`, `docs/spec/SPEC.md`, `docs/spec/ACCEPTANCE.md`.
3. Implement only the active sprint, only on the allowlist.
4. Test names in ACCEPTANCE.md are mandatory. Do not rename them.
5. `cargo test --workspace` until green, or write `NO SHIP` in `oracle/RUNLOG.md` and stop.
6. Open a pull request from `feat/*`. Do not push `main`.
7. Merge only if CI is green and every changed path is on the allowlist.

If you cannot name the invariant you are encoding, output only `NO SHIP`.

## Spec (this chat)

Write `docs/**`, `AGENTS.md`, `README.md`. Direct to `main` is allowed for the spec user. When a sprint needs a new oracle path, add it to the CURRENT.md allowlist. Do not copy that rule into another file.

## Critic

Scores the living critic (`crates/critic`) after a sprint is on `main`. Writes feedback in chat. Spec turns accepted notes into `docs/` changes. Never patches `crates/`.
