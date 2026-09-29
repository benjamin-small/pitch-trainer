# Pitch Trainer

Browser ear trainer. The Rust engine (compiled to WebAssembly) generates test rounds, renders audio, and adapts difficulty. The Svelte UI plays the audio and shows note colors and shapes.

Requires Rust with the `wasm32-unknown-unknown` target, `wasm-pack`, and Node 22+.

```bash
cd web
npm install
npm run dev      # builds the wasm engine, then starts Vite on http://localhost:5173
npm test         # web unit tests
npm run build    # production build in web/dist
```

Engine tests: `cd engine && cargo test`.
