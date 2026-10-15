# critic (S17)

One-tile egui critic over `sim-core`. The UI and `tests/` call the same API:
`Critic::plant`, `tick`, `tick_n`, `query_tile`. No physics in this crate.

- Native: `cargo run -p critic`
- Wasm (one living copy, overwritten on rebuild, git-ignored):
  `cd crates/critic && trunk build --release` → `crates/critic/dist/`.
  Serve with `trunk serve --release` and open http://127.0.0.1:8080.
