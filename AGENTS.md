# Shared rules — stem-world-sim

Shared rules only. Bot procedure lives in `docs/bots/<bot>.md`. Do not copy a rule from here into a bot file. `docs/REPO_LAYOUT.md` is the tree.

You are one role per turn: **spec** (this chat), **oracle** (smith bot), or **critic**. Not two.

## Write list

The active oracle write list is the **Oracle allowlist** line in `docs/sprints/CURRENT.md`.

Default if that line is missing: `crates/sim-core/**`, `oracle/RUNLOG.md`, `.github/workflows/**`, root `Cargo.toml`.

- Oracle may read `docs/data/kernel_catalog.csv`. Oracle does not edit spec markdown.
- Spec writes `docs/**`, `AGENTS.md`, `README.md`. Spec does not add Rust and does not merge kernel PRs. A new oracle path is one line on the CURRENT.md allowlist.
- Critic writes feedback in chat. Critic does not patch `crates/` or `docs/spec/`.

## How we keep one source

- One fact, one file. Do not restate a rule, a path list, or a dataset in a second file.
- Segment by reader. Shared rules here. One bot's steps in that bot's file. Sprint scope in that sprint file. A bot does not parse another bot's procedure.
- After a spec commit to `main`, pull the local `stem-world-sim` clone so project files match the repo.
- Files that are not in the repo live outside the clone (charter drafts, bulk imports, build output). Do not leave untracked copies inside the repo folder.
