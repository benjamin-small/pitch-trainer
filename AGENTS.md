# Agent instructions

## Purpose

Pitch Trainer is a browser ear trainer with a Rust/WASM engine and Svelte UI.
Keep exercise generation, synthesis, and adaptive difficulty in `engine`; the
web layer handles playback, presentation, answers, and browser storage.

## Setup and validation

Use stable Rust with the wasm32 target, wasm-pack, and Node 22+ (CI uses 24).

```sh
npm ci --prefix web
cargo test --manifest-path engine/Cargo.toml --locked
npm test --prefix web
npm run build --prefix web
npm run check --prefix web
```

## Constraints

- Preserve existing behavior unless the issue authorizes a change.
- Add tests for changed engine rules or UI helpers; update measured coverage honestly.
- Keep generated files under `web/src/lib/engine` out of commits.
- Do not commit credentials or local user progress.
- Preserve unrelated changes in a dirty checkout.
