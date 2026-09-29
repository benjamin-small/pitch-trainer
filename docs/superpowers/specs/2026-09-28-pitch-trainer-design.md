# Pitch Trainer — Design

**Date:** 2026-09-28
**Status:** Approved in brainstorming, pending spec review

## Summary

A browser-based pitch tester and trainer. A Rust engine compiled to WebAssembly generates test rounds, renders audio, grades answers, and adapts difficulty. A Svelte front end plays the audio, collects answers, shows visual feedback, and persists progress in localStorage. No server, no accounts.

## Goals

- Three test types that train relative pitch discrimination and short-term pitch memory.
- Adaptive difficulty per test that settles near ~80% accuracy.
- A consistent color + shape per note to build a visual/auditory association, shown in examples and reveals but never during the challenge itself.
- A live waveform display of the sound being played (outside the challenge).
- All tones are standard 12-tone equal-temperament piano notes (A4 = 440 Hz). No microtones.

## Non-goals

- Accounts, sync, leaderboards, or any backend.
- Microtonal / cent-level tests.
- Real-time audio synthesis (AudioWorklet). Audio is pre-rendered per round.
- Mobile-native packaging. It is a responsive web page.

## Architecture

```
pitch-trainer/
├── engine/                     Rust crate (cdylib + rlib), built with wasm-pack
│   ├── Cargo.toml
│   └── src/
│       ├── notes.rs            Note (MIDI number) ↔ name ↔ frequency
│       ├── synth.rs            Render note / note sequence → Vec<f32> samples
│       ├── rounds.rs           Round generation for 3 test types + grading
│       ├── difficulty.rs       Staircase state per test type
│       └── lib.rs              wasm-bindgen API (thin wrapper only)
└── web/                        Svelte + Vite + TypeScript
    ├── package.json
    └── src/
        ├── lib/engine/         wasm-pack output (generated, gitignored)
        ├── lib/audio.ts        AudioContext, buffer playback, AnalyserNode
        ├── lib/noteVisuals.ts  Pitch class → { color, shape }
        ├── lib/shapes.ts       Shape rendering
        ├── lib/types.ts        Shared TypeScript types
        ├── lib/trainer.ts      Test state and round orchestration
        ├── lib/storage.ts      Versioned localStorage save/load
        ├── lib/answers.ts      Answer button labels and logic
        ├── App.svelte          Start gate, mode navigation
        └── components/
            ├── Explore.svelte
            ├── TestRunner.svelte
            ├── AnswerButtons.svelte
            ├── Waveform.svelte
            └── NoteGlyph.svelte
```

**Boundary rule:** the Rust engine knows nothing about the DOM. Svelte knows nothing about test rules or correct answers until the engine reveals them after grading.

### Engine API (wasm-bindgen surface)

```
Engine::new(sample_rate: f32, seed: u32, saved_state_json: Option<String>) -> Engine
Engine::newRound(&mut self, kind: string) -> RoundView
Engine::roundAudio(&self) -> Float32Array           // current round's challenge audio
Engine::answer(&mut self, answer: u32) -> AnswerResult
Engine::progress(&self, kind: string) -> Progress
Engine::noteAudio(&self, midi: u8) -> Float32Array  // single note, for Explore
Engine::scaleNotes(&self, from: u8, to: u8) -> Array<u8>
Engine::scaleAudio(&self, from: u8, to: u8) -> Float32Array
Engine::stateJson(&self) -> String                   // difficulty + stats, for persistence
```

- `seed` is u32 (so JS passes a number, not a BigInt; widened to u64 internally).
- `kind`: `"upDown" | "pickTwo" | "sequence"`.
- `RoundView` (sent to UI before answering): test kind, number of answer options, prompt text, number of notes in the sequence (for placeholders), note timing (onsets in seconds) so the UI can sync placeholder highlights. **Does not include note identities or the correct answer.**
- `AnswerResult`: `correct: bool`, `correct_answer: u32`, the round's notes (MIDI numbers + onsets) for the reveal, new level, best level.
- Answer encoding: UpDown `0 = Higher, 1 = Lower`; PickTwo `0 = First, 1 = Second`; Sequence `0..N-1` = position.
- Calling `answer` with no active round, or an out-of-range answer, returns an error (JS exception). The round is consumed after one answer.

Randomness uses a seeded PRNG (e.g. `rand_pcg`/`fastrand` with explicit seed). JS passes a seed from `crypto.getRandomValues`, and tests pass a fixed seed.

## Notes and audio

- Note range: **C3 (MIDI 48) to C6 (MIDI 84)** inclusive.
- Frequency: `440 * 2^((midi - 69) / 12)`.
- Timbre: sine fundamental + 2nd harmonic (0.3) + 3rd harmonic (0.15), normalized so peak ≤ 0.9.
- Envelope: 10 ms linear attack, sustain, 80 ms release to exactly zero. Buffers start and end at 0.
- Timing: note duration **0.6 s**, inter-note gap **0.15 s**, pause before the target note **0.8 s**.
- Sample rate is whatever `AudioContext.sampleRate` reports, passed to the engine.

## Test types

All rounds follow the same flow:

1. **Play:** the engine renders the round audio and the UI plays it. The UI shows only neutral placeholders (grey slots that pulse on each note onset). No colors, shapes, names, or waveform.
2. **Answer:** buttons unlock when playback ends. **Replay** is available up to 2 times per round (buttons lock during replay).
3. **Reveal:** show correct/incorrect and the correct answer, then replay the round with each note's glyph (shape + color + name) appearing on its onset and the live waveform visible.
4. **Next:** a Next button (or Space) starts a new round.

JSON field names are camelCase. Kinds are `"upDown" | "pickTwo" | "sequence"`.

### Up/Down

- Audio: note A, gap, note B.
- Answer: Higher / Lower (is B higher or lower than A).
- Generation: pick gap `g` from the level, pick direction uniformly, pick A so both A and B are within range.
- Level → gap (semitones): levels 1–8 = 12, 9, 7, 5, 4, 3, 2, 1.

### Pick Two

- Audio: note A, gap, note B, 0.8 s pause, target (A or B, uniformly).
- Answer: First / Second.
- Generation: A and B differ by exactly `g` semitones (direction random), both in range.
- Level → gap: same table as Up/Down (12, 9, 7, 5, 4, 3, 2, 1).

### Sequence

- Audio: N notes separated by gaps, 0.8 s pause, target (one of the N, uniformly).
- Answer: position 1…N.
- Generation: N distinct notes, all in range. The minimum pairwise distance between any two sequence notes is at least `d`, and at least one pair is exactly `d` apart. This makes the level's spacing the actual closest distinction.
- Levels (length N, min spacing d):

| Level | N | d |
|---|---|---|
| 1 | 3 | 5 |
| 2 | 4 | 5 |
| 3 | 5 | 5 |
| 4 | 6 | 4 |
| 5 | 7 | 3 |
| 6 | 8 | 3 |
| 7 | 8 | 2 |
| 8 | 8 | 1 |

  Feasibility: 8 notes with min spacing 3 need a 21-semitone span, which fits within the 36-semitone range. All table rows are satisfiable. Generation builds sorted gaps directly: one gap is exactly `d`, the others are `d` plus 0–4 random extra semitones within the range budget. The notes are then shuffled. This always succeeds.

## Adaptive difficulty

- Each test type keeps its own state: `level` (1…8), `streak` (consecutive correct), `best_level`, `attempts`, `correct`.
- Correct answer: `streak += 1`. When `streak == 3`, `level = min(level + 1, 8)` and `streak = 0`.
- Wrong answer: `level = max(level - 1, 1)`, `streak = 0`.
- `best_level = max(best_level, level)` after every update.
- New users start at level 1.
- The UI shows the current level, best level, and accuracy % for the active test.

## Note visuals

The same pitch class always gets the same color and shape in every octave. Colors walk the hue wheel in 30° steps.

| Note | Hue | Shape |
|---|---|---|
| C | 0° (red) | circle |
| C# | 30° | ring |
| D | 60° | triangle |
| D# | 90° | inverted triangle |
| E | 120° | square |
| F | 150° | diamond |
| F# | 180° | pentagon |
| G | 210° | star |
| G# | 240° | hexagon |
| A | 270° | cross |
| A# | 300° | crescent |
| B | 330° | octagon |

Colors are HSL with fixed saturation/lightness tuned for contrast on both light and dark backgrounds. `NoteGlyph` renders the shape as inline SVG with the note name and octave label below (e.g. "G4").

**Visibility rule:** glyphs, colors, note names, and waveform are shown in Explore and during Reveal replays only. They are never shown while a challenge is playing or awaiting an answer.

## Explore mode

- On-screen keyboard, C3–C6 (37 keys). Each key is tinted with its note color and shows a small glyph.
- Click/tap plays the note. Computer keys `A W S E D F T G Y H U J K` map to one octave (C…C), and `Z`/`X` shift the octave down/up within range.
- A large glyph for the last-played note, plus the live waveform.
- **Play scale:** plays the chromatic scale in the current octave, and the glyph updates on each note.

## Waveform

- `Waveform.svelte` reads time-domain data from an `AnalyserNode` (fftSize 2048) each animation frame and draws it on a canvas, stroked in the current note's color.
- The canvas is not mounted during challenge playback or answering.

## Persistence

- localStorage key `pitch-trainer:v1`. Value: `{ version: 1, engine: <state_json string> }`.
- Saved after every answer.
- On load, if parsing fails or the version differs, discard and start fresh (no error shown).
- The engine validates `saved_state_json`. On invalid input it falls back to defaults rather than failing.

## Error handling

- **Audio unlock:** a "Tap to start" gate creates/resumes the AudioContext on user gesture, then initializes the engine with the context's sample rate.
- **WASM load failure:** show a plain message ("Couldn't load the audio engine. Try a current version of Chrome, Firefox, or Safari.").
- **Input during playback:** answer buttons are disabled while audio plays, and the engine also rejects answers with no active round.
- **Out-of-range answer:** engine returns an error, and the UI never sends one by construction.

## Testing

**Rust (`cargo test`, native target):**
- notes: A4 = 440 Hz, C4 ≈ 261.63 Hz.
- synth: buffer length = expected samples, peak ≤ 0.9, first and last samples == 0, no NaN.
- rounds: for each test × each level × many seeds, every note is within C3–C6. Up/Down and Pick Two gaps equal the level gap. Sequence notes are distinct, min spacing == d exactly, length == N. The target is one of the sequence notes, and `correct_answer` matches its position. Grading returns correct only for the right answer.
- difficulty: 3 correct → level up, 1 wrong → level down, clamping at 1 and 8, best_level tracking.
- state: `state_json` → `Engine::new(..., Some(json))` round-trips. Garbage JSON → defaults.

**Web (Vitest):**
- `noteVisuals`: name formatting for every note in range, all 12 pitch classes mapped, shapes unique, colors unique.
- `storage`: save/load round-trip, corrupt data → null, wrong version → null.

**Manual:** run the app and play several rounds of each test. Check that nothing leaks visually during challenges, that the reveal replay shows glyphs in sync, and that reload restores levels.

## Build and tooling

- `engine/`: `wasm-pack build --target web --out-dir ../web/src/lib/engine`.
- `web/`: Vite + Svelte 5 + TypeScript. `npm run dev` runs the wasm-pack build, then `vite`. `npm run build` does the same for production. `npm test` runs Vitest.
- `.gitignore`: `target/`, `web/node_modules/`, `web/src/lib/engine/`, `web/dist/`.
