# Critic demos

`d0.html` is a phone-ready page that repeats the S01 capacity-step so a human can poke pond vs dry surface without Rust.

Open after merge:
https://htmlpreview.github.io/?https://github.com/stem-world-sim/stem-world-sim/blob/main/docs/demo/d0.html

This is a view. `cargo test` in sim-core is the kernel. If they disagree, the test wins.

A wasm crate that *calls* sim-core is scheduled after a rustup/wasm CI job exists. This sandbox cannot compile wasm32 today.
