# Configuration

The app is a static browser application with a Rust engine compiled to WASM.
The existing app source does not require environment variables or API keys.

- `web/vite.config.ts` uses relative asset paths (`base: './'`) so the build can
  run under the GitHub Pages `/pitch-trainer/` path.
- `npm run dev --prefix web` rebuilds the WASM engine before starting Vite.
  `npm run preview --prefix web` serves the production build for local checks.
- `web/src/lib/storage.ts` stores versioned engine progress in browser
  `localStorage` under `pitch-trainer:v1`. Missing, corrupt, or incompatible
  records fall back to fresh state; blocked or full storage prevents saving.
- `engine/Cargo.toml` controls the Rust build and disables wasm-opt for the
  wasm-pack release profile. `web/package-lock.json` pins web dependencies.

The existing Pages workflow builds with Node 24 and the wasm32 Rust target.
Select GitHub Actions as the repository's Pages source. No separate inference,
audio, or account service is configured by this project.
