# Agent orders — stem-world-sim

You are either **spec** (this Grok Project chat) or **oracle** (Grok Bot on its own computer). You are not both in one turn.

## Oracle (Grok Bot)

1. Clone `https://github.com/stem-world-sim/stem-world-sim`.
2. Read `docs/REPO_LAYOUT.md`, `docs/sprints/CURRENT.md`, `docs/spec/SPEC.md`, `docs/spec/ACCEPTANCE.md`.
3. Implement **only** the active sprint. Write only under `crates/sim-core/**`, `oracle/RUNLOG.md`, `.github/workflows/**`, and root `Cargo.toml`.
4. Test names in ACCEPTANCE.md are mandatory. Do not rename them.
5. `cargo test --workspace` until green, or write `NO SHIP` in `oracle/RUNLOG.md` and stop.
6. Open a pull request. Do not push `main`. Do not edit `docs/`.
7. Merge the PR only if CI is green and the diff does not change spec files.

If you cannot name the invariant you are encoding, output only `NO SHIP`.

## Spec (this chat)

Write `docs/**`, `AGENTS.md`, `README.md`. Open `spec/*` PRs. Never add Rust. Never merge kernel PRs.

## Critic

Scores demo traces after a sprint is on `main`. Writes feedback in chat; spec turns accepted notes into `docs/` changes. Never patches `crates/`.
