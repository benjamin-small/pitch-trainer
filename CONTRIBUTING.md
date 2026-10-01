# Contributing

Follow the [README](README.md) for the Rust, wasm-pack, and Node prerequisites.
Keep changes focused and add tests for changed behavior. Before proposing a
change, run:

```sh
npm ci --prefix web
cargo test --manifest-path engine/Cargo.toml --locked
npm test --prefix web
npm run build --prefix web
npm run check --prefix web
```

The build generates WASM bindings required by the web typecheck. See
[testing](docs/testing.md) for test scope and coverage, and [AGENTS.md](AGENTS.md)
for agent constraints. Do not commit generated WASM output, build artifacts,
credentials, or browser progress data.
