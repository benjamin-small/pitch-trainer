# Testing

## Measured coverage

Measured on October 1, 2026 against application code at `4a7a295`, using Rust
1.95.0, cargo-llvm-cov 0.8.7, Node 26.10.0, and Vitest/V8 provider 5.0.2:

| Scope | Lines | Statements/regions | Functions | Branches |
| --- | --- | --- | --- | --- |
| Native Rust engine | 93.61% (659/704) | 93.58% regions | 87.39% | Not measured |
| Web TypeScript source | 50.56% | 48.97% statements | 41.17% | 72.72% |

The Rust measurement uses native tests, not browser WASM execution. Web coverage
includes untested TypeScript source but excludes test files, generated engine
bindings, and `vite-env.d.ts`. Svelte components and actual browser audio are
outside this measurement. These figures must not be combined into a single
whole-product percentage. All 43 engine and 19 web tests passed.

## Reproduce

Run the README validation commands first. For Rust coverage:

```sh
cargo llvm-cov --manifest-path engine/Cargo.toml --summary-only
```

For web coverage, make `@vitest/coverage-v8@5.0.2` available alongside Vitest.
The recorded run used a temporary provider linked into ignored `node_modules`,
without changing project dependency files. From `web/`, run:

```sh
npm exec -- vitest run --coverage --coverage.provider=v8 \
  --coverage.reporter=text --coverage.reporter=json-summary \
  '--coverage.include=src/**/*.ts' \
  '--coverage.exclude=src/**/*.test.ts' \
  '--coverage.exclude=src/lib/engine/**' \
  '--coverage.exclude=src/vite-env.d.ts'
```

## Test scope

Rust tests cover adaptive difficulty, note frequencies, deterministic random
selection, round generation and grading, audio synthesis, state recovery, and
trainer progress. Web tests cover answer helpers, note visuals, shape mapping,
and versioned browser storage handling. They do not establish real-device
playback quality or browser interaction behavior.

The PR workflow runs both suites, builds the WASM/Svelte app, and runs
`svelte-check`. The current typecheck reports zero errors and two existing
initial-value capture warnings in `TestRunner.svelte`. Browser playback and
exercise interaction still need manual verification when those behaviors change.
