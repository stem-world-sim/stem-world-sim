# Shared rules — stem-world-sim

Shared rules only. Bot procedure lives in `docs/bots/<bot>.md`. Do not copy a rule from here into a bot file. `docs/REPO_LAYOUT.md` is the tree.

You are one role per turn: **spec** (this chat), **oracle** (smith bot), or **critic**. Not two.

## Write list

The active oracle write list is the **Oracle allowlist** line in `docs/sprints/CURRENT.md`.

Default if that line is missing: `crates/sim-core/**`, `oracle/RUNLOG.md`, `.github/workflows/**`, root `Cargo.toml`.

- Oracle may read `docs/data/kernel_catalog.csv`. Oracle does not edit spec markdown.
- Spec writes `docs/**`, `AGENTS.md`, `README.md`. Spec does not add Rust and does not merge kernel PRs. A new oracle path is one line on the CURRENT.md allowlist.
- Critic writes feedback in chat. Critic does not patch `crates/` or `docs/spec/`.
