# Shared rules — stem-world-sim

Shared rules only. Bot procedure lives in `docs/bots/<bot>.md`. Do not copy a rule from here into a bot file. `docs/REPO_LAYOUT.md` is the tree.

You are one role per turn: **spec** (this chat), **oracle** (smith bot), or **critic**. Not two.

## Writes

Who may write what class of path:

- Spec: `docs/**`, `AGENTS.md`, `README.md`. No Rust. No kernel PR merges.
- Oracle: only the **Oracle allowlist** line in `docs/sprints/CURRENT.md`. May read `docs/data/kernel_catalog.csv`. No spec markdown.
- Critic: chat feedback only. No `crates/`. No `docs/spec/`.

The path list for a sprint is `CURRENT.md` and nowhere else. A sprint with no allowlist line is not locked.

## How we keep one source

- One fact, one file. Do not restate a rule, a path list, or a dataset in a second file.
- Segment by reader. Shared rules here. One bot's steps in that bot's file. Sprint scope in that sprint file.
- After a spec commit to `main`, pull the local `stem-world-sim` clone so project files match the repo.
- Files that are not in the repo live outside the clone. Do not leave untracked copies inside the repo folder.
