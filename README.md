# Pitch Trainer

[Play the live demo](https://benjamin-small.github.io/pitch-trainer/)

## Purpose

Browser ear trainer. The Rust engine (compiled to WebAssembly) generates test rounds, renders audio, and adapts difficulty. The Svelte UI plays the audio and shows note colors and shapes.

## Setup and usage

Requires Rust (2024 edition support) with the `wasm32-unknown-unknown` target, `wasm-pack`, and Node 22+.

Validated locally with Rust 1.95.0, wasm-pack 0.15.0, and Node 26.10.0;
CI uses Node 24 and stable Rust.

```bash
cd web
npm ci
npm run dev      # builds the wasm engine, then starts Vite on http://localhost:5173
npm test         # web unit tests
npm run build    # production build in web/dist
```

Engine tests: `cd engine && cargo test`.

## Validation

```sh
cargo test --manifest-path engine/Cargo.toml --locked
npm test --prefix web
npm run build --prefix web
npm run check --prefix web
```

CI uses Node 24 and stable Rust. Build first to generate the WASM bindings
needed by the web typecheck. See [testing and measured coverage](docs/testing.md),
[configuration](docs/configuration.md), and [contribution guidance](CONTRIBUTING.md).

## License

This project is licensed under the [MIT License](LICENSE). Third-party dependencies
retain their own licenses.
