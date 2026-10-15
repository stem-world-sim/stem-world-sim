# Shared rules — stem-world-sim

Shared rules only. Do not list bots, paths, or procedures here.

You are one role per turn. Not two.

Your role file is `docs/bots/<role>.md`. Paths, reads, and steps are there. A sprint's path list is the allowlist line in `docs/sprints/CURRENT.md` and nowhere else.

`docs/REPO_LAYOUT.md` is the tree.

- One fact, one file. Do not restate a rule, a path list, or a dataset.
- A file holds only what its reader needs. Shared file: what every role needs. Role file: that role. Sprint file: that sprint. Do not merge files to stop drift. Point, do not copy.
- After a spec commit to `main`, pull the local `stem-world-sim` clone so project files match the repo.
- Files that are not in the repo live outside the clone. Do not leave untracked copies inside the repo folder.
