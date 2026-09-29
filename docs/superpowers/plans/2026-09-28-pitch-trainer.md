# Pitch Trainer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a browser pitch tester/trainer. A Rust→WASM engine generates rounds, renders audio, grades answers and adapts difficulty. A Svelte UI plays the audio, collects answers, shows note glyphs/waveform on examples and reveals, and saves progress to localStorage.

**Architecture:** `engine/` is a pure-Rust crate (`pitch_engine`) with a thin wasm-bindgen wrapper (`lib.rs`). All logic lives in plain Rust modules, tested natively with `cargo test`. `web/` is a Vite + Svelte 5 + TypeScript app. It loads the wasm-pack output from `web/src/lib/engine/` through a typed wrapper (`web/src/lib/trainer.ts`). The engine never exposes the correct answer or note identities until after grading.

**Tech Stack:** Rust 1.95 (edition 2024), wasm-bindgen 0.2.129, serde 1, serde_json 1, serde-wasm-bindgen 0.6.5, wasm-pack 0.15. Node 26, Vite 8.3, Svelte 5.57, @sveltejs/vite-plugin-svelte 7.3, TypeScript 6.0 (**not 7**, because svelte-check 4.7 only supports TS ≤ 6), svelte-check 4.7, Vitest 5.0.

**Spec:** `docs/superpowers/specs/2026-09-28-pitch-trainer-design.md`

## Global Constraints

- Notes are MIDI numbers, and every played note is in **C3 (48) … C6 (84)** inclusive. Standard 12-TET notes only, A4 = 440 Hz. No microtones.
- Timing: note **0.6 s**, gap between notes **0.15 s**, pause before target note **0.8 s**.
- Timbre: harmonics 1, 2, 3 at amplitudes 1.0 / 0.3 / 0.15. Peak ≤ **0.9**. 10 ms attack, 80 ms release. Every buffer starts and ends at exactly 0.
- Levels 1…8 per test. **3 correct in a row → +1 level, 1 wrong → −1 level**, clamped to [1, 8].
- Pair gaps (Up/Down, Pick Two) by level: **12, 9, 7, 5, 4, 3, 2, 1** semitones.
- Sequence (length, closest spacing) by level: **(3,5) (4,5) (5,5) (6,4) (7,3) (8,3) (8,2) (8,1)**.
- Replays allowed per round before answering: **2**.
- Visibility rule: glyphs, note colors, names and waveform are shown in **Explore** and **Reveal** only, never while a challenge is playing or waiting for an answer.
- localStorage key **`pitch-trainer:v1`**, value `{ "version": 1, "engine": "<engine state JSON string>" }`.
- All JS-facing JSON uses **camelCase** field names. Test kinds are the strings **`"upDown" | "pickTwo" | "sequence"`**.
- WASM-load failure message (exact): **"Couldn't load the audio engine. Try a current version of Chrome, Firefox, or Safari."**

## File Map

```
.gitignore
README.md
engine/
  Cargo.toml
  src/lib.rs          wasm-bindgen `Engine` wrapper (thin; no logic)
  src/notes.rs        note range + frequency
  src/rng.rs          seeded SplitMix64 PRNG
  src/synth.rs        NoteEvent, timeline(), render()
  src/difficulty.rs   Progress (staircase)
  src/rounds.rs       TestKind, Round generation + grading
  src/trainer.rs      Trainer (state, rounds, audio, persistence), RoundView, AnswerResult
web/
  package.json, vite.config.ts, svelte.config.js, tsconfig.json, index.html
  src/main.ts, src/app.css, src/App.svelte
  src/lib/types.ts        TS mirrors of engine JSON types
  src/lib/trainer.ts      typed wrapper around wasm Engine
  src/lib/noteVisuals.ts  pitch class → name/color/shape (+ test)
  src/lib/shapes.ts       shape → SVG geometry (+ test)
  src/lib/storage.ts      versioned localStorage (+ test)
  src/lib/answers.ts      answer options + test titles (+ test)
  src/lib/audio.ts        AudioOut: AudioContext, playback, analyser
  src/components/NoteGlyph.svelte
  src/components/Waveform.svelte
  src/components/Explore.svelte
  src/components/AnswerButtons.svelte
  src/components/TestRunner.svelte
```

**Deviation from spec file list (intentional, DRY):** the spec's three answer panels (`UpDownPanel`, `PickTwoPanel`, `SequencePanel`) differ only in their labels. They become one `AnswerButtons.svelte`, fed by `answerOptions()` in `answers.ts`.

---

### Task 1: Engine crate scaffold, note math, PRNG

**Files:**
- Create: `.gitignore`, `engine/Cargo.toml`, `engine/src/lib.rs`, `engine/src/notes.rs`, `engine/src/rng.rs`

**Interfaces:**
- Produces: `notes::{LOWEST: u8 = 48, HIGHEST: u8 = 84, frequency(midi: u8) -> f32}`; `rng::Rng { new(seed: u64), next_u64(&mut) -> u64, range(&mut, lo: u32, hi: u32) -> u32 /* inclusive */, coin(&mut) -> bool, shuffle<T>(&mut, &mut [T]) }`

- [ ] **Step 1: Create `.gitignore`**

```
target/
web/node_modules/
web/src/lib/engine/
web/dist/
.DS_Store
```

- [ ] **Step 2: Create `engine/Cargo.toml`**

```toml
[package]
name = "pitch_engine"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2.129"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
serde-wasm-bindgen = "0.6.5"

[profile.release]
opt-level = "s"

# Skip wasm-opt so builds don't need to download binaryen.
[package.metadata.wasm-pack.profile.release]
wasm-opt = false
```

- [ ] **Step 3: Create `engine/src/lib.rs`**

```rust
pub mod notes;
pub mod rng;
```

- [ ] **Step 4: Write failing tests in `engine/src/notes.rs`**

```rust
//! Equal-temperament note math. Notes are MIDI numbers (60 = C4, 69 = A4).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_440() {
        assert!((frequency(69) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn c4_is_middle_c() {
        assert!((frequency(60) - 261.6256).abs() < 1e-2);
    }

    #[test]
    fn octave_doubles_frequency() {
        assert!((frequency(81) / frequency(69) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn range_is_c3_to_c6() {
        assert_eq!(LOWEST, 48);
        assert_eq!(HIGHEST, 84);
    }
}
```

- [ ] **Step 5: Write failing tests in `engine/src/rng.rs`**

```rust
//! Small deterministic PRNG (SplitMix64). Seeded from JS at startup; fixed seeds in tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn range_is_inclusive_and_covers_all_values() {
        let mut rng = Rng::new(1);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let v = rng.range(3, 7);
            assert!((3..=7).contains(&v));
            seen[(v - 3) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn range_single_value() {
        let mut rng = Rng::new(2);
        assert_eq!(rng.range(4, 4), 4);
    }

    #[test]
    fn coin_gives_both_sides() {
        let mut rng = Rng::new(3);
        let heads = (0..1000).filter(|_| rng.coin()).count();
        assert!(heads > 400 && heads < 600, "heads = {heads}");
    }

    #[test]
    fn shuffle_keeps_elements() {
        let mut rng = Rng::new(4);
        let mut items = [1, 2, 3, 4, 5, 6, 7, 8];
        rng.shuffle(&mut items);
        let mut sorted = items;
        sorted.sort();
        assert_eq!(sorted, [1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
```

- [ ] **Step 6: Run tests to verify they fail**

Run: `cd engine && cargo test`
Expected: compile errors: `cannot find function 'frequency'`, `cannot find type 'Rng'`, etc.

- [ ] **Step 7: Implement `notes.rs` (insert above the `#[cfg(test)]` block)**

```rust
/// Lowest note used anywhere in the app (C3).
pub const LOWEST: u8 = 48;
/// Highest note used anywhere in the app (C6).
pub const HIGHEST: u8 = 84;

pub fn frequency(midi: u8) -> f32 {
    440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0)
}
```

- [ ] **Step 8: Implement `rng.rs` (insert above the `#[cfg(test)]` block)**

```rust
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi, "empty range {lo}..={hi}");
        let span = hi as u64 - lo as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range(0, i as u32) as usize;
            items.swap(i, j);
        }
    }
}
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `cd engine && cargo test`
Expected: 9 tests pass.

- [ ] **Step 10: Commit**

```bash
git add .gitignore engine/Cargo.toml engine/Cargo.lock engine/src
git commit -m "feat(engine): scaffold crate with note math and seeded rng"
```

---

### Task 2: Synth: timelines and sample rendering

**Files:**
- Create: `engine/src/synth.rs`
- Modify: `engine/src/lib.rs` (add `pub mod synth;`)

**Interfaces:**
- Consumes: `notes::frequency`
- Produces: `synth::{NOTE_SECS, GAP_SECS, TARGET_PAUSE_SECS, PEAK}`; `#[derive(Serialize)] #[serde(rename_all="camelCase")] pub struct NoteEvent { pub midi: u8, pub onset: f32 /* seconds */ }`; `timeline(notes: &[u8], target: Option<u8>) -> Vec<NoteEvent>`; `render(events: &[NoteEvent], sample_rate: f32) -> Vec<f32>`

- [ ] **Step 1: Add module to `engine/src/lib.rs`**

```rust
pub mod notes;
pub mod rng;
pub mod synth;
```

- [ ] **Step 2: Write failing tests in `engine/src/synth.rs`**

```rust
//! Renders notes to mono f32 sample buffers.

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 8000.0;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn timeline_spaces_notes_and_pauses_before_target() {
        let events = timeline(&[60, 62, 64], Some(62));
        let onsets: Vec<f32> = events.iter().map(|e| e.onset).collect();
        let midis: Vec<u8> = events.iter().map(|e| e.midi).collect();
        assert_eq!(midis, vec![60, 62, 64, 62]);
        assert!(close(onsets[0], 0.0));
        assert!(close(onsets[1], 0.75));
        assert!(close(onsets[2], 1.5));
        assert!(close(onsets[3], 1.5 + 0.6 + 0.8));
    }

    #[test]
    fn timeline_without_target() {
        let events = timeline(&[60, 72], None);
        assert_eq!(events.len(), 2);
        assert!(close(events[1].onset, 0.75));
    }

    #[test]
    fn single_note_length() {
        let buf = render(&timeline(&[69], None), SR);
        assert_eq!(buf.len(), (0.6 * SR).round() as usize);
    }

    #[test]
    fn sequence_length_ends_with_last_note() {
        let buf = render(&timeline(&[60, 62, 64], Some(62)), SR);
        let last_start = (2.9 * SR).round() as usize;
        assert_eq!(buf.len(), last_start + (0.6 * SR).round() as usize);
    }

    #[test]
    fn buffers_start_and_end_silent() {
        let buf = render(&timeline(&[48, 84], None), SR);
        assert_eq!(buf[0], 0.0);
        assert_eq!(*buf.last().unwrap(), 0.0);
    }

    #[test]
    fn peak_is_bounded_and_audible() {
        for midi in [48u8, 60, 69, 84] {
            let buf = render(&timeline(&[midi], None), SR);
            let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak <= PEAK + 1e-6, "midi {midi} peak {peak}");
            assert!(peak > 0.3, "midi {midi} peak {peak}");
            assert!(buf.iter().all(|s| s.is_finite()));
        }
    }

    #[test]
    fn empty_events_render_empty_buffer() {
        assert!(render(&[], SR).is_empty());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd engine && cargo test synth`
Expected: compile errors (`timeline`, `render` not found).

- [ ] **Step 4: Implement (insert above the `#[cfg(test)]` block)**

```rust
use serde::Serialize;

use crate::notes::frequency;

pub const NOTE_SECS: f32 = 0.6;
pub const GAP_SECS: f32 = 0.15;
pub const TARGET_PAUSE_SECS: f32 = 0.8;
pub const PEAK: f32 = 0.9;
const ATTACK_SECS: f32 = 0.010;
const RELEASE_SECS: f32 = 0.080;
/// Relative amplitudes of harmonics 1, 2, 3.
const HARMONICS: [f32; 3] = [1.0, 0.3, 0.15];

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEvent {
    pub midi: u8,
    /// Seconds from the start of the buffer.
    pub onset: f32,
}

/// Lays `notes` out back to back, then `target` after the longer pause.
pub fn timeline(notes: &[u8], target: Option<u8>) -> Vec<NoteEvent> {
    let step = NOTE_SECS + GAP_SECS;
    let mut events: Vec<NoteEvent> = notes
        .iter()
        .enumerate()
        .map(|(i, &midi)| NoteEvent { midi, onset: i as f32 * step })
        .collect();
    if let Some(midi) = target {
        let last_end = events.last().map_or(0.0, |e| e.onset + NOTE_SECS);
        events.push(NoteEvent { midi, onset: last_end + TARGET_PAUSE_SECS });
    }
    events
}

pub fn render(events: &[NoteEvent], sample_rate: f32) -> Vec<f32> {
    let note_len = (NOTE_SECS * sample_rate).round() as usize;
    let starts: Vec<usize> = events
        .iter()
        .map(|e| (e.onset * sample_rate).round() as usize)
        .collect();
    let total = starts.iter().map(|s| s + note_len).max().unwrap_or(0);
    let mut out = vec![0.0; total];
    for (event, &start) in events.iter().zip(&starts) {
        write_note(&mut out[start..start + note_len], frequency(event.midi), sample_rate);
    }
    out
}

fn write_note(buf: &mut [f32], freq: f32, sample_rate: f32) {
    let n = buf.len();
    let attack = (ATTACK_SECS * sample_rate).max(1.0);
    let release = (RELEASE_SECS * sample_rate).max(1.0);
    // Harmonic amplitudes sum to 1.45, so scaling by PEAK / 1.45 keeps |sample| <= PEAK.
    let norm = PEAK / HARMONICS.iter().sum::<f32>();
    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 / sample_rate;
        let env = (i as f32 / attack).min(1.0) * ((n - 1 - i) as f32 / release).min(1.0);
        let wave: f32 = HARMONICS
            .iter()
            .enumerate()
            .map(|(h, amp)| amp * (std::f32::consts::TAU * freq * (h as f32 + 1.0) * t).sin())
            .sum();
        *sample += wave * env * norm;
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd engine && cargo test`
Expected: all tests pass (9 from Task 1 + 7 new).

- [ ] **Step 6: Commit**

```bash
git add engine/src
git commit -m "feat(engine): render note timelines to sample buffers"
```

---

### Task 3: Adaptive difficulty (Progress)

**Files:**
- Create: `engine/src/difficulty.rs`
- Modify: `engine/src/lib.rs` (add `pub mod difficulty;`)

**Interfaces:**
- Produces: `difficulty::{MIN_LEVEL: u8 = 1, MAX_LEVEL: u8 = 8}`; `#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(default, rename_all="camelCase")] pub struct Progress { pub level: u8, pub streak: u8, pub best_level: u8, pub attempts: u32, pub correct: u32 }`, where `Default` gives level 1 and best_level 1; `Progress::record(&mut self, correct: bool)`; `Progress::sanitized(self) -> Progress`

- [ ] **Step 1: Add module to `lib.rs`**

```rust
pub mod difficulty;
pub mod notes;
pub mod rng;
pub mod synth;
```

- [ ] **Step 2: Write failing tests in `engine/src/difficulty.rs`**

```rust
//! Per-test adaptive staircase: 3 correct in a row → harder, 1 wrong → easier.

#[cfg(test)]
mod tests {
    use super::*;

    fn at_level(level: u8) -> Progress {
        Progress { level, best_level: level, ..Progress::default() }
    }

    #[test]
    fn starts_at_level_one() {
        let p = Progress::default();
        assert_eq!((p.level, p.streak, p.best_level, p.attempts, p.correct), (1, 0, 1, 0, 0));
    }

    #[test]
    fn three_correct_levels_up_and_resets_streak() {
        let mut p = Progress::default();
        p.record(true);
        p.record(true);
        assert_eq!((p.level, p.streak), (1, 2));
        p.record(true);
        assert_eq!((p.level, p.streak), (2, 0));
    }

    #[test]
    fn wrong_levels_down_and_resets_streak() {
        let mut p = at_level(3);
        p.record(true);
        p.record(false);
        assert_eq!((p.level, p.streak), (2, 0));
    }

    #[test]
    fn clamps_at_bounds() {
        let mut low = Progress::default();
        low.record(false);
        assert_eq!(low.level, MIN_LEVEL);

        let mut high = at_level(MAX_LEVEL);
        for _ in 0..3 {
            high.record(true);
        }
        assert_eq!(high.level, MAX_LEVEL);
    }

    #[test]
    fn best_level_never_drops() {
        let mut p = Progress::default();
        for _ in 0..6 {
            p.record(true);
        }
        assert_eq!(p.level, 3);
        p.record(false);
        assert_eq!((p.level, p.best_level), (2, 3));
    }

    #[test]
    fn counts_attempts_and_correct() {
        let mut p = Progress::default();
        p.record(true);
        p.record(false);
        p.record(true);
        assert_eq!((p.attempts, p.correct), (3, 2));
    }

    #[test]
    fn sanitized_repairs_bad_values() {
        let bad = Progress { level: 0, streak: 9, best_level: 20, attempts: 2, correct: 5 };
        let fixed = bad.sanitized();
        assert_eq!(fixed, Progress { level: 1, streak: 2, best_level: 8, attempts: 2, correct: 2 });

        let behind = Progress { level: 5, best_level: 1, ..Progress::default() }.sanitized();
        assert_eq!(behind.best_level, 5);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd engine && cargo test difficulty`
Expected: compile error, `Progress` not found.

- [ ] **Step 4: Implement (insert above `#[cfg(test)]`)**

```rust
use serde::{Deserialize, Serialize};

pub const MIN_LEVEL: u8 = 1;
pub const MAX_LEVEL: u8 = 8;
const STREAK_TO_LEVEL_UP: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Progress {
    pub level: u8,
    /// Consecutive correct answers since the last level change.
    pub streak: u8,
    pub best_level: u8,
    pub attempts: u32,
    pub correct: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Progress { level: MIN_LEVEL, streak: 0, best_level: MIN_LEVEL, attempts: 0, correct: 0 }
    }
}

impl Progress {
    pub fn record(&mut self, correct: bool) {
        self.attempts += 1;
        if correct {
            self.correct += 1;
            self.streak += 1;
            if self.streak >= STREAK_TO_LEVEL_UP {
                self.level = (self.level + 1).min(MAX_LEVEL);
                self.streak = 0;
            }
        } else {
            self.level = self.level.saturating_sub(1).max(MIN_LEVEL);
            self.streak = 0;
        }
        self.best_level = self.best_level.max(self.level);
    }

    /// Repairs values loaded from untrusted saved data.
    pub fn sanitized(self) -> Self {
        let level = self.level.clamp(MIN_LEVEL, MAX_LEVEL);
        Progress {
            level,
            streak: self.streak.min(STREAK_TO_LEVEL_UP - 1),
            best_level: self.best_level.clamp(level, MAX_LEVEL),
            attempts: self.attempts,
            correct: self.correct.min(self.attempts),
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd engine && cargo test`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add engine/src
git commit -m "feat(engine): adaptive difficulty staircase"
```

---

### Task 4: Round generation and grading

**Files:**
- Create: `engine/src/rounds.rs`
- Modify: `engine/src/lib.rs` (add `pub mod rounds;`)

**Interfaces:**
- Consumes: `notes::{LOWEST, HIGHEST}`, `rng::Rng`, `synth::{timeline, NoteEvent}`, `difficulty::{MIN_LEVEL, MAX_LEVEL}`
- Produces: `#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all="camelCase")] pub enum TestKind { UpDown, PickTwo, Sequence }` with `TestKind::parse(&str) -> Option<TestKind>` accepting `"upDown" | "pickTwo" | "sequence"`; `PAIR_GAPS: [u8; 8]`; `SEQUENCE_LEVELS: [(usize, u8); 8]`; `pub struct Round { pub kind: TestKind, pub notes: Vec<u8>, pub target: Option<u8>, pub answer: u32 }` with `Round::generate(kind, level: u8, rng: &mut Rng) -> Round`, `options(&self) -> u32`, `events(&self) -> Vec<NoteEvent>`, `is_correct(&self, answer: u32) -> bool`
- Answer codes: UpDown `0 = higher, 1 = lower`. PickTwo `0 = first, 1 = second`. Sequence `0..N-1` = position.

**Note on the spec:** the spec says sequence generation uses rejection sampling with a fallback. This plan uses a direct construction that always succeeds and satisfies the same invariants (N distinct notes in range, closest pair exactly `d`). Task 13 updates the spec wording.

- [ ] **Step 1: Add module to `lib.rs`**

```rust
pub mod difficulty;
pub mod notes;
pub mod rng;
pub mod rounds;
pub mod synth;
```

- [ ] **Step 2: Write failing tests in `engine/src/rounds.rs`**

```rust
//! Generates rounds for the three test types and grades answers.

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: u64 = 300;

    fn in_range(notes: &[u8]) -> bool {
        notes.iter().all(|&n| (LOWEST..=HIGHEST).contains(&n))
    }

    fn closest_spacing(notes: &[u8]) -> u8 {
        let mut sorted = notes.to_vec();
        sorted.sort();
        sorted.windows(2).map(|w| w[1] - w[0]).min().unwrap()
    }

    fn each_round(kind: TestKind, mut check: impl FnMut(u8, &Round)) {
        for level in MIN_LEVEL..=MAX_LEVEL {
            for seed in 0..SEEDS {
                let mut rng = Rng::new(seed);
                let round = Round::generate(kind, level, &mut rng);
                check(level, &round);
            }
        }
    }

    #[test]
    fn up_down_rounds() {
        let mut saw = [false; 2];
        each_round(TestKind::UpDown, |level, r| {
            assert_eq!(r.notes.len(), 2);
            assert!(in_range(&r.notes));
            assert_eq!(r.target, None);
            assert_eq!(r.options(), 2);
            let (a, b) = (r.notes[0], r.notes[1]);
            assert_eq!(a.abs_diff(b), PAIR_GAPS[level as usize - 1]);
            assert_eq!(r.answer, if b > a { 0 } else { 1 });
            saw[r.answer as usize] = true;
        });
        assert_eq!(saw, [true, true]);
    }

    #[test]
    fn pick_two_rounds() {
        let mut saw = [false; 2];
        each_round(TestKind::PickTwo, |level, r| {
            assert_eq!(r.notes.len(), 2);
            assert!(in_range(&r.notes));
            assert_eq!(r.options(), 2);
            assert_eq!(r.notes[0].abs_diff(r.notes[1]), PAIR_GAPS[level as usize - 1]);
            assert_eq!(r.target, Some(r.notes[r.answer as usize]));
            saw[r.answer as usize] = true;
        });
        assert_eq!(saw, [true, true]);
    }

    #[test]
    fn sequence_rounds() {
        each_round(TestKind::Sequence, |level, r| {
            let (len, spacing) = SEQUENCE_LEVELS[level as usize - 1];
            assert_eq!(r.notes.len(), len);
            assert_eq!(r.options(), len as u32);
            assert!(in_range(&r.notes));
            let mut unique = r.notes.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), len, "notes must be distinct: {:?}", r.notes);
            assert_eq!(closest_spacing(&r.notes), spacing, "notes {:?}", r.notes);
            assert!(r.answer < len as u32);
            assert_eq!(r.target, Some(r.notes[r.answer as usize]));
        });
    }

    #[test]
    fn sequence_answer_positions_vary() {
        let mut seen = [false; 8];
        for seed in 0..SEEDS {
            let r = Round::generate(TestKind::Sequence, 8, &mut Rng::new(seed));
            seen[r.answer as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn out_of_range_levels_are_clamped() {
        let mut rng = Rng::new(0);
        assert_eq!(Round::generate(TestKind::Sequence, 0, &mut rng).notes.len(), 3);
        assert_eq!(Round::generate(TestKind::Sequence, 99, &mut rng).notes.len(), 8);
    }

    #[test]
    fn grading() {
        let r = Round::generate(TestKind::Sequence, 4, &mut Rng::new(9));
        for a in 0..r.options() {
            assert_eq!(r.is_correct(a), a == r.answer);
        }
    }

    #[test]
    fn events_include_target_last() {
        let r = Round::generate(TestKind::PickTwo, 1, &mut Rng::new(5));
        let events = r.events();
        assert_eq!(events.len(), 3);
        assert_eq!(Some(events[2].midi), r.target);
    }

    #[test]
    fn test_kind_names() {
        for kind in TestKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(TestKind::parse(json.trim_matches('"')), Some(kind));
        }
        assert_eq!(serde_json::to_string(&TestKind::UpDown).unwrap(), "\"upDown\"");
        assert_eq!(TestKind::parse("nope"), None);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd engine && cargo test rounds`
Expected: compile errors (`Round`, `TestKind` not found).

- [ ] **Step 4: Implement (insert above `#[cfg(test)]`)**

```rust
use serde::{Deserialize, Serialize};

use crate::difficulty::{MAX_LEVEL, MIN_LEVEL};
use crate::notes::{HIGHEST, LOWEST};
use crate::rng::Rng;
use crate::synth::{NoteEvent, timeline};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestKind {
    UpDown,
    PickTwo,
    Sequence,
}

impl TestKind {
    pub const ALL: [TestKind; 3] = [TestKind::UpDown, TestKind::PickTwo, TestKind::Sequence];

    pub fn parse(name: &str) -> Option<TestKind> {
        match name {
            "upDown" => Some(TestKind::UpDown),
            "pickTwo" => Some(TestKind::PickTwo),
            "sequence" => Some(TestKind::Sequence),
            _ => None,
        }
    }
}

/// Semitone gap between the two notes (Up/Down, Pick Two), indexed by level - 1.
pub const PAIR_GAPS: [u8; 8] = [12, 9, 7, 5, 4, 3, 2, 1];
/// (length, closest spacing in semitones) for Sequence, indexed by level - 1.
pub const SEQUENCE_LEVELS: [(usize, u8); 8] =
    [(3, 5), (4, 5), (5, 5), (6, 4), (7, 3), (8, 3), (8, 2), (8, 1)];
/// Most extra semitones added to any non-tight gap in a sequence, to keep runs compact.
const MAX_EXTRA_SPACING: u32 = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub kind: TestKind,
    /// Notes in playback order, excluding the target.
    pub notes: Vec<u8>,
    /// The note played again after the pause (Pick Two, Sequence).
    pub target: Option<u8>,
    /// Correct answer code (see module docs on answer codes).
    pub answer: u32,
}

impl Round {
    pub fn generate(kind: TestKind, level: u8, rng: &mut Rng) -> Round {
        let index = (level.clamp(MIN_LEVEL, MAX_LEVEL) - 1) as usize;
        match kind {
            TestKind::UpDown => up_down(PAIR_GAPS[index], rng),
            TestKind::PickTwo => pick_two(PAIR_GAPS[index], rng),
            TestKind::Sequence => {
                let (len, spacing) = SEQUENCE_LEVELS[index];
                sequence(len, spacing, rng)
            }
        }
    }

    pub fn options(&self) -> u32 {
        match self.kind {
            TestKind::UpDown | TestKind::PickTwo => 2,
            TestKind::Sequence => self.notes.len() as u32,
        }
    }

    pub fn events(&self) -> Vec<NoteEvent> {
        timeline(&self.notes, self.target)
    }

    pub fn is_correct(&self, answer: u32) -> bool {
        answer == self.answer
    }
}

/// Two notes exactly `gap` apart in random order. Returns (first, second).
fn pair(gap: u8, rng: &mut Rng) -> (u8, u8) {
    let low = rng.range(LOWEST as u32, (HIGHEST - gap) as u32) as u8;
    if rng.coin() { (low, low + gap) } else { (low + gap, low) }
}

fn up_down(gap: u8, rng: &mut Rng) -> Round {
    let (a, b) = pair(gap, rng);
    Round { kind: TestKind::UpDown, notes: vec![a, b], target: None, answer: if b > a { 0 } else { 1 } }
}

fn pick_two(gap: u8, rng: &mut Rng) -> Round {
    let (a, b) = pair(gap, rng);
    let pick = rng.range(0, 1);
    let target = if pick == 0 { a } else { b };
    Round { kind: TestKind::PickTwo, notes: vec![a, b], target: Some(target), answer: pick }
}

/// `len` distinct notes whose closest pair is exactly `spacing` apart, in random order.
fn sequence(len: usize, spacing: u8, rng: &mut Rng) -> Round {
    let range = (HIGHEST - LOWEST) as u32;
    let gap_count = len - 1;
    let tight = rng.range(0, gap_count as u32 - 1) as usize;
    let mut slack = range - gap_count as u32 * spacing as u32;
    let mut gaps = vec![spacing as u32; gap_count];
    for (i, gap) in gaps.iter_mut().enumerate() {
        if i != tight {
            let extra = rng.range(0, slack.min(MAX_EXTRA_SPACING));
            *gap += extra;
            slack -= extra;
        }
    }
    let span: u32 = gaps.iter().sum();
    let mut note = LOWEST as u32 + rng.range(0, range - span);
    let mut notes = vec![note as u8];
    for gap in gaps {
        note += gap;
        notes.push(note as u8);
    }
    rng.shuffle(&mut notes);
    let answer = rng.range(0, len as u32 - 1);
    Round { kind: TestKind::Sequence, target: Some(notes[answer as usize]), notes, answer }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd engine && cargo test`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add engine/src
git commit -m "feat(engine): generate and grade rounds for all three tests"
```

---

### Task 5: Trainer core: rounds, audio, answers, persistence

**Files:**
- Create: `engine/src/trainer.rs`
- Modify: `engine/src/lib.rs` (add `pub mod trainer;`)

**Interfaces:**
- Consumes: everything from Tasks 1–4
- Produces:
  - `#[serde(default, rename_all="camelCase")] pub struct SavedState { pub up_down: Progress, pub pick_two: Progress, pub sequence: Progress }` with `SavedState::from_json(&str) -> SavedState` (never fails; garbage → default)
  - `#[serde(rename_all="camelCase")] pub struct RoundView { pub kind: TestKind, pub prompt: &'static str, pub options: u32, pub onsets: Vec<f32>, pub has_target: bool }`
  - `#[serde(rename_all="camelCase")] pub struct AnswerResult { pub correct: bool, pub correct_answer: u32, pub notes: Vec<NoteEvent>, pub progress: Progress }`
  - `pub enum TrainerError { NoActiveRound, AnswerOutOfRange { answer: u32, options: u32 } }`: implements `Display` + `std::error::Error`
  - `pub struct Trainer` with `new(sample_rate: f32, seed: u64, saved_json: Option<&str>)`, `new_round(&mut, TestKind) -> RoundView`, `round_audio(&) -> Result<Vec<f32>, TrainerError>` (works before *and* after answering, for the reveal), `answer(&mut, u32) -> Result<AnswerResult, TrainerError>` (one answer per round), `note_audio(&, midi: u8) -> Vec<f32>`, `scale_notes(&, from: u8, to: u8) -> Vec<NoteEvent>`, `render_events(&, &[NoteEvent]) -> Vec<f32>`, `progress(&, TestKind) -> Progress`, `state_json(&) -> String`

- [ ] **Step 1: Add module to `lib.rs`**

```rust
pub mod difficulty;
pub mod notes;
pub mod rng;
pub mod rounds;
pub mod synth;
pub mod trainer;
```

- [ ] **Step 2: Write failing tests in `engine/src/trainer.rs`**

```rust
//! The engine's stateful core: owns progress, the current round, and rendering.

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 8000.0;

    fn trainer() -> Trainer {
        Trainer::new(SR, 42, None)
    }

    fn right_answer(t: &Trainer) -> u32 {
        t.round.as_ref().unwrap().answer
    }

    #[test]
    fn view_has_slots_but_no_notes() {
        let mut t = trainer();
        let v = t.new_round(TestKind::PickTwo);
        assert_eq!(v.kind, TestKind::PickTwo);
        assert_eq!(v.options, 2);
        assert_eq!(v.onsets.len(), 3);
        assert!(v.has_target);
        assert!(!v.prompt.is_empty());
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("midi") && !json.contains("answer"), "view leaks: {json}");
    }

    #[test]
    fn up_down_view_has_no_target() {
        let mut t = trainer();
        let v = t.new_round(TestKind::UpDown);
        assert_eq!(v.onsets.len(), 2);
        assert!(!v.has_target);
    }

    #[test]
    fn round_audio_matches_timeline() {
        let mut t = trainer();
        t.new_round(TestKind::UpDown);
        let audio = t.round_audio().unwrap();
        let note_len = (0.6 * SR).round() as usize;
        assert_eq!(audio.len(), (0.75 * SR).round() as usize + note_len);
    }

    #[test]
    fn answering_records_progress_and_reveals_notes() {
        let mut t = trainer();
        t.new_round(TestKind::Sequence);
        let expected_notes = t.round.as_ref().unwrap().events();
        let a = right_answer(&t);
        let result = t.answer(a).unwrap();
        assert!(result.correct);
        assert_eq!(result.correct_answer, a);
        assert_eq!(result.notes, expected_notes);
        assert_eq!(result.progress.attempts, 1);
        assert_eq!(t.progress(TestKind::Sequence).correct, 1);
        assert_eq!(t.progress(TestKind::UpDown).attempts, 0);
    }

    #[test]
    fn wrong_answer_is_graded_wrong() {
        let mut t = trainer();
        t.new_round(TestKind::UpDown);
        let wrong = 1 - right_answer(&t);
        let result = t.answer(wrong).unwrap();
        assert!(!result.correct);
        assert_eq!(result.correct_answer, 1 - wrong);
    }

    #[test]
    fn one_answer_per_round() {
        let mut t = trainer();
        assert_eq!(t.answer(0), Err(TrainerError::NoActiveRound));
        t.new_round(TestKind::UpDown);
        t.answer(0).unwrap();
        assert_eq!(t.answer(0), Err(TrainerError::NoActiveRound));
        assert!(t.round_audio().is_ok(), "audio stays available for the reveal");
    }

    #[test]
    fn out_of_range_answer_is_rejected_and_round_stays_open() {
        let mut t = trainer();
        t.new_round(TestKind::PickTwo);
        assert_eq!(t.answer(2), Err(TrainerError::AnswerOutOfRange { answer: 2, options: 2 }));
        assert!(t.answer(0).is_ok());
    }

    #[test]
    fn round_audio_before_any_round_errors() {
        assert_eq!(trainer().round_audio(), Err(TrainerError::NoActiveRound));
    }

    #[test]
    fn three_correct_raises_level() {
        let mut t = trainer();
        for _ in 0..3 {
            t.new_round(TestKind::UpDown);
            let a = right_answer(&t);
            t.answer(a).unwrap();
        }
        assert_eq!(t.progress(TestKind::UpDown).level, 2);
    }

    #[test]
    fn state_round_trips_through_json() {
        let mut t = trainer();
        for _ in 0..4 {
            t.new_round(TestKind::PickTwo);
            let a = right_answer(&t);
            t.answer(a).unwrap();
        }
        let json = t.state_json();
        let restored = Trainer::new(SR, 1, Some(&json));
        assert_eq!(restored.progress(TestKind::PickTwo), t.progress(TestKind::PickTwo));
        assert!(json.contains("\"pickTwo\"") && json.contains("\"bestLevel\""), "{json}");
    }

    #[test]
    fn bad_saved_state_falls_back() {
        assert_eq!(SavedState::from_json("not json"), SavedState::default());
        let partial = SavedState::from_json(r#"{"sequence":{"level":5}}"#);
        assert_eq!(partial.sequence.level, 5);
        assert_eq!(partial.sequence.best_level, 5);
        assert_eq!(partial.up_down, Progress::default());
        let wild = SavedState::from_json(r#"{"upDown":{"level":200}}"#);
        assert_eq!(wild.up_down.level, 8);
    }

    #[test]
    fn scale_and_note_audio() {
        let t = trainer();
        let scale = t.scale_notes(60, 72);
        assert_eq!(scale.len(), 13);
        assert_eq!(scale[12].midi, 72);
        assert!(!t.render_events(&scale).is_empty());
        assert_eq!(t.note_audio(69).len(), (0.6 * SR).round() as usize);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd engine && cargo test trainer`
Expected: compile errors (`Trainer` not found).

- [ ] **Step 4: Implement (insert above `#[cfg(test)]`)**

```rust
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::difficulty::Progress;
use crate::rng::Rng;
use crate::rounds::{Round, TestKind};
use crate::synth::{NoteEvent, render, timeline};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SavedState {
    pub up_down: Progress,
    pub pick_two: Progress,
    pub sequence: Progress,
}

impl SavedState {
    /// Parses saved state, falling back to defaults on any problem.
    pub fn from_json(json: &str) -> SavedState {
        match serde_json::from_str::<SavedState>(json) {
            Ok(s) => SavedState {
                up_down: s.up_down.sanitized(),
                pick_two: s.pick_two.sanitized(),
                sequence: s.sequence.sanitized(),
            },
            Err(_) => SavedState::default(),
        }
    }

    fn progress(&self, kind: TestKind) -> &Progress {
        match kind {
            TestKind::UpDown => &self.up_down,
            TestKind::PickTwo => &self.pick_two,
            TestKind::Sequence => &self.sequence,
        }
    }

    fn progress_mut(&mut self, kind: TestKind) -> &mut Progress {
        match kind {
            TestKind::UpDown => &mut self.up_down,
            TestKind::PickTwo => &mut self.pick_two,
            TestKind::Sequence => &mut self.sequence,
        }
    }
}

/// What the UI may know about a round before it is answered.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundView {
    pub kind: TestKind,
    pub prompt: &'static str,
    pub options: u32,
    /// Onset (seconds) of every sounding note, target last when present.
    pub onsets: Vec<f32>,
    pub has_target: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerResult {
    pub correct: bool,
    pub correct_answer: u32,
    /// Every sounding note with its onset, for the reveal replay.
    pub notes: Vec<NoteEvent>,
    pub progress: Progress,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TrainerError {
    NoActiveRound,
    AnswerOutOfRange { answer: u32, options: u32 },
}

impl fmt::Display for TrainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainerError::NoActiveRound => write!(f, "no round is waiting for an answer"),
            TrainerError::AnswerOutOfRange { answer, options } => {
                write!(f, "answer {answer} is out of range (round has {options} options)")
            }
        }
    }
}

impl std::error::Error for TrainerError {}

fn prompt(kind: TestKind) -> &'static str {
    match kind {
        TestKind::UpDown => "Was the second note higher or lower than the first?",
        TestKind::PickTwo => "Which note was played again?",
        TestKind::Sequence => "Which position was played again?",
    }
}

pub struct Trainer {
    sample_rate: f32,
    rng: Rng,
    state: SavedState,
    round: Option<Round>,
    answered: bool,
}

impl Trainer {
    pub fn new(sample_rate: f32, seed: u64, saved_json: Option<&str>) -> Trainer {
        Trainer {
            sample_rate,
            rng: Rng::new(seed),
            state: saved_json.map(SavedState::from_json).unwrap_or_default(),
            round: None,
            answered: false,
        }
    }

    pub fn new_round(&mut self, kind: TestKind) -> RoundView {
        let level = self.state.progress(kind).level;
        let round = Round::generate(kind, level, &mut self.rng);
        let view = RoundView {
            kind,
            prompt: prompt(kind),
            options: round.options(),
            onsets: round.events().iter().map(|e| e.onset).collect(),
            has_target: round.target.is_some(),
        };
        self.round = Some(round);
        self.answered = false;
        view
    }

    pub fn round_audio(&self) -> Result<Vec<f32>, TrainerError> {
        let round = self.round.as_ref().ok_or(TrainerError::NoActiveRound)?;
        Ok(self.render_events(&round.events()))
    }

    pub fn answer(&mut self, answer: u32) -> Result<AnswerResult, TrainerError> {
        let round = match &self.round {
            Some(round) if !self.answered => round,
            _ => return Err(TrainerError::NoActiveRound),
        };
        let options = round.options();
        if answer >= options {
            return Err(TrainerError::AnswerOutOfRange { answer, options });
        }
        let correct = round.is_correct(answer);
        let correct_answer = round.answer;
        let notes = round.events();
        let kind = round.kind;

        let progress = self.state.progress_mut(kind);
        progress.record(correct);
        let progress = *progress;
        self.answered = true;
        Ok(AnswerResult { correct, correct_answer, notes, progress })
    }

    pub fn note_audio(&self, midi: u8) -> Vec<f32> {
        self.render_events(&timeline(&[midi], None))
    }

    pub fn scale_notes(&self, from: u8, to: u8) -> Vec<NoteEvent> {
        let notes: Vec<u8> = (from.min(to)..=from.max(to)).collect();
        timeline(&notes, None)
    }

    pub fn render_events(&self, events: &[NoteEvent]) -> Vec<f32> {
        render(events, self.sample_rate)
    }

    pub fn progress(&self, kind: TestKind) -> Progress {
        *self.state.progress(kind)
    }

    pub fn state_json(&self) -> String {
        serde_json::to_string(&self.state).expect("saved state always serializes")
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd engine && cargo test`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add engine/src
git commit -m "feat(engine): trainer core with answers, audio and persistence"
```

---

### Task 6: WASM bindings

**Files:**
- Modify: `engine/src/lib.rs`

**Interfaces:**
- Consumes: `trainer::Trainer`, `rounds::TestKind`
- Produces (JS, from wasm-pack `--target web`): default export `init(): Promise<…>`. `class Engine { constructor(sampleRate: number, seed: number, savedStateJson?: string | null); newRound(kind: string): any; roundAudio(): Float32Array; answer(answer: number): any; noteAudio(midi: number): Float32Array; scaleNotes(from: number, to: number): any; scaleAudio(from: number, to: number): Float32Array; progress(kind: string): any; stateJson(): string }`. Methods throw a JS `Error` on engine errors or unknown kind strings.
- **Spec deviation:** `seed` is `u32` (not `u64`), so JS passes a plain number instead of a BigInt. Task 13 updates the spec.

- [ ] **Step 1: Replace `engine/src/lib.rs`**

```rust
pub mod difficulty;
pub mod notes;
pub mod rng;
pub mod rounds;
pub mod synth;
pub mod trainer;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use rounds::TestKind;
use trainer::Trainer;

/// JS-facing wrapper around [`Trainer`]. Holds no logic of its own.
#[wasm_bindgen]
pub struct Engine {
    trainer: Trainer,
}

fn parse_kind(kind: &str) -> Result<TestKind, JsError> {
    TestKind::parse(kind).ok_or_else(|| JsError::new(&format!("unknown test kind: {kind}")))
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: f32, seed: u32, saved_state_json: Option<String>) -> Engine {
        Engine { trainer: Trainer::new(sample_rate, seed as u64, saved_state_json.as_deref()) }
    }

    #[wasm_bindgen(js_name = newRound)]
    pub fn new_round(&mut self, kind: &str) -> Result<JsValue, JsError> {
        to_js(&self.trainer.new_round(parse_kind(kind)?))
    }

    #[wasm_bindgen(js_name = roundAudio)]
    pub fn round_audio(&self) -> Result<Vec<f32>, JsError> {
        Ok(self.trainer.round_audio()?)
    }

    pub fn answer(&mut self, answer: u32) -> Result<JsValue, JsError> {
        to_js(&self.trainer.answer(answer)?)
    }

    #[wasm_bindgen(js_name = noteAudio)]
    pub fn note_audio(&self, midi: u8) -> Vec<f32> {
        self.trainer.note_audio(midi)
    }

    #[wasm_bindgen(js_name = scaleNotes)]
    pub fn scale_notes(&self, from: u8, to: u8) -> Result<JsValue, JsError> {
        to_js(&self.trainer.scale_notes(from, to))
    }

    #[wasm_bindgen(js_name = scaleAudio)]
    pub fn scale_audio(&self, from: u8, to: u8) -> Vec<f32> {
        self.trainer.render_events(&self.trainer.scale_notes(from, to))
    }

    pub fn progress(&self, kind: &str) -> Result<JsValue, JsError> {
        to_js(&self.trainer.progress(parse_kind(kind)?))
    }

    #[wasm_bindgen(js_name = stateJson)]
    pub fn state_json(&self) -> String {
        self.trainer.state_json()
    }
}
```

- [ ] **Step 2: Native tests still pass**

Run: `cd engine && cargo test`
Expected: all pass (the bindings compile natively).

- [ ] **Step 3: Build the WASM package**

Run: `cd engine && wasm-pack build --target web --out-dir ../web/src/lib/engine`
Expected: `[INFO]: :-) Your wasm pkg is ready to publish at .../web/src/lib/engine`. The folder contains `pitch_engine.js`, `pitch_engine.d.ts` and `pitch_engine_bg.wasm`.

- [ ] **Step 4: Verify the generated API**

Run: `grep -E "newRound|roundAudio|stateJson|constructor" web/src/lib/engine/pitch_engine.d.ts`
Expected: lines showing `newRound(kind: string): any;`, `roundAudio(): Float32Array;`, `stateJson(): string;` and `constructor(sample_rate: number, seed: number, saved_state_json?: string | null);`.

- [ ] **Step 5: Commit** (`web/src/lib/engine/` is gitignored)

```bash
git add engine/src/lib.rs
git commit -m "feat(engine): wasm-bindgen Engine wrapper"
```

---

### Task 7: Web scaffold, typed engine wrapper, smoke page

**Files:**
- Create: `web/package.json`, `web/vite.config.ts`, `web/svelte.config.js`, `web/tsconfig.json`, `web/index.html`, `web/src/main.ts`, `web/src/app.css`, `web/src/App.svelte` (temporary smoke version), `web/src/lib/types.ts`, `web/src/lib/trainer.ts`

**Interfaces:**
- Consumes: wasm-pack output `web/src/lib/engine/pitch_engine.js` (Task 6)
- Produces:
  - `types.ts`: `TestKind = 'upDown' | 'pickTwo' | 'sequence'`, `NoteEvent { midi: number; onset: number }`, `Progress { level; streak; bestLevel; attempts; correct }` (numbers), `RoundView { kind: TestKind; prompt: string; options: number; onsets: number[]; hasTarget: boolean }`, `AnswerResult { correct: boolean; correctAnswer: number; notes: NoteEvent[]; progress: Progress }`
  - `trainer.ts`: `class Trainer { static create(sampleRate: number, savedStateJson: string | null): Promise<Trainer>; newRound(kind): RoundView; roundAudio(): Float32Array; answer(n): AnswerResult; noteAudio(midi): Float32Array; scaleNotes(from, to): NoteEvent[]; scaleAudio(from, to): Float32Array; progress(kind): Progress; stateJson(): string }`
  - `app.css`: CSS tokens `--bg --surface --surface-2 --text --muted --border --accent --good --bad --placeholder`, and button classes `.btn`, `.btn.primary`

- [ ] **Step 1: Create `web/package.json`**

```json
{
  "name": "pitch-trainer-web",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "wasm": "wasm-pack build ../engine --target web --out-dir ../web/src/lib/engine",
    "dev": "npm run wasm && vite",
    "build": "npm run wasm && vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "test": "vitest run"
  }
}
```

- [ ] **Step 2: Install dev dependencies**

Run: `cd web && npm install -D svelte@^5.57.1 vite@^8.3.1 @sveltejs/vite-plugin-svelte@^7.3.1 typescript@^6.0.3 svelte-check@^4.7.6 @tsconfig/svelte@^5.0.8 vitest@^5.0.2`
Expected: `added N packages`, no peer-dependency errors.

- [ ] **Step 3: Create config files**

`web/vite.config.ts`:
```ts
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  test: { include: ['src/**/*.test.ts'] },
});
```

`web/svelte.config.js`:
```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default { preprocess: vitePreprocess() };
```

`web/tsconfig.json`:
```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": true,
    "types": ["vite/client"]
  },
  "include": ["src/**/*.ts", "src/**/*.svelte", "vite.config.ts"]
}
```

`web/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Pitch Trainer</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 4: Create `web/src/main.ts` and `web/src/app.css`**

`web/src/main.ts`:
```ts
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

mount(App, { target: document.getElementById('app')! });
```

`web/src/app.css`:
```css
:root {
  --bg: #f6f5f1;
  --surface: #ffffff;
  --surface-2: #eceae4;
  --text: #1d1c1a;
  --muted: #6b6862;
  --border: #d9d6ce;
  --accent: #3b5bdb;
  --good: #2b8a3e;
  --bad: #c92a2a;
  --placeholder: #c8c5bd;
  color-scheme: light dark;
  font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
  line-height: 1.45;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg: #151514;
    --surface: #1f1f1d;
    --surface-2: #2a2a27;
    --text: #ecebe6;
    --muted: #9c9a93;
    --border: #393935;
    --accent: #7c93f5;
    --good: #51cf66;
    --bad: #ff6b6b;
    --placeholder: #4a4945;
  }
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
}

button {
  font: inherit;
  color: inherit;
  cursor: pointer;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.btn {
  padding: 0.6rem 1.1rem;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface);
}

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.btn:focus-visible,
.key:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

- [ ] **Step 5: Create `web/src/lib/types.ts`**

```ts
// Mirrors of the engine's JSON shapes (engine/src/trainer.rs, synth.rs, difficulty.rs).

export type TestKind = 'upDown' | 'pickTwo' | 'sequence';

export interface NoteEvent {
  midi: number;
  /** Seconds from the start of the buffer. */
  onset: number;
}

export interface Progress {
  level: number;
  streak: number;
  bestLevel: number;
  attempts: number;
  correct: number;
}

export interface RoundView {
  kind: TestKind;
  prompt: string;
  options: number;
  onsets: number[];
  hasTarget: boolean;
}

export interface AnswerResult {
  correct: boolean;
  correctAnswer: number;
  notes: NoteEvent[];
  progress: Progress;
}
```

- [ ] **Step 6: Create `web/src/lib/trainer.ts`**

```ts
import init, { Engine } from './engine/pitch_engine';
import type { AnswerResult, NoteEvent, Progress, RoundView, TestKind } from './types';

/** Typed wrapper over the wasm Engine (wasm-bindgen types object returns as `any`). */
export class Trainer {
  private constructor(private readonly engine: Engine) {}

  static async create(sampleRate: number, savedStateJson: string | null): Promise<Trainer> {
    await init();
    const [seed] = crypto.getRandomValues(new Uint32Array(1));
    return new Trainer(new Engine(sampleRate, seed, savedStateJson));
  }

  newRound(kind: TestKind): RoundView {
    return this.engine.newRound(kind);
  }

  roundAudio(): Float32Array {
    return this.engine.roundAudio();
  }

  answer(answer: number): AnswerResult {
    return this.engine.answer(answer);
  }

  noteAudio(midi: number): Float32Array {
    return this.engine.noteAudio(midi);
  }

  scaleNotes(from: number, to: number): NoteEvent[] {
    return this.engine.scaleNotes(from, to);
  }

  scaleAudio(from: number, to: number): Float32Array {
    return this.engine.scaleAudio(from, to);
  }

  progress(kind: TestKind): Progress {
    return this.engine.progress(kind);
  }

  stateJson(): string {
    return this.engine.stateJson();
  }
}
```

- [ ] **Step 7: Create temporary smoke `web/src/App.svelte`**

```svelte
<script lang="ts">
  import { Trainer } from './lib/trainer';

  let status = $state('Loading engine…');

  Trainer.create(48000, null)
    .then((trainer) => {
      const view = trainer.newRound('sequence');
      status = `Engine ready: "${view.prompt}" ${view.options} options, ${trainer.roundAudio().length} samples`;
    })
    .catch((error) => (status = `Engine failed: ${error}`));
</script>

<p>{status}</p>
```

- [ ] **Step 8: Type-check and build**

Run: `cd web && npm run build && npm run check`
Expected: the Vite build finishes and writes `dist/`. svelte-check reports `0 errors`. If svelte-check flags a `tsconfig` option as deprecated under TS 6, remove that option from `web/tsconfig.json` and re-run.

- [ ] **Step 9: Smoke-test in a browser**

Run: `cd web && npx vite --port 5173` (the wasm was built in Step 8), then open `http://localhost:5173`.
Expected: the page shows `Engine ready: "Which position was played again?" 3 options, N samples` with N > 0. The browser console shows no errors.

- [ ] **Step 10: Commit**

```bash
git add web/package.json web/package-lock.json web/vite.config.ts web/svelte.config.js web/tsconfig.json web/index.html web/src
git commit -m "feat(web): scaffold Svelte app and typed engine wrapper"
```

---

### Task 8: Note visuals: names, colors, shapes, NoteGlyph

**Files:**
- Create: `web/src/lib/noteVisuals.ts`, `web/src/lib/noteVisuals.test.ts`, `web/src/lib/shapes.ts`, `web/src/lib/shapes.test.ts`, `web/src/components/NoteGlyph.svelte`

**Interfaces:**
- Produces:
  - `noteVisuals.ts`: `LOWEST = 48`, `HIGHEST = 84`, `type Shape = 'circle' | 'ring' | 'triangle' | 'invertedTriangle' | 'square' | 'diamond' | 'pentagon' | 'star' | 'hexagon' | 'cross' | 'crescent' | 'octagon'`, `pitchClass(midi): number`, `noteName(midi): string` (e.g. `"C#4"`), `noteColor(midi): string` (CSS `hsl(...)`), `noteFrequency(midi): number`, `isSharp(midi): boolean`, `noteVisual(midi): { name: string; label: string; color: string; shape: Shape }`
  - `shapes.ts`: `type ShapeGeometry = { type: 'circle'; r: number } | { type: 'ring'; r: number; width: number } | { type: 'polygon'; points: string } | { type: 'path'; d: string }`, `shapeGeometry(shape: Shape): ShapeGeometry` (in the `viewBox="-50 -50 100 100"` coordinate space)
  - `NoteGlyph.svelte` props: `{ midi: number; size?: number /* px, default 96 */; label?: boolean /* default true */ }`

- [ ] **Step 1: Write failing tests `web/src/lib/noteVisuals.test.ts`**

```ts
import { describe, expect, it } from 'vitest';
import { isSharp, noteColor, noteFrequency, noteName, noteVisual, pitchClass } from './noteVisuals';

describe('noteVisuals', () => {
  it('names notes with octave numbers', () => {
    expect(noteName(48)).toBe('C3');
    expect(noteName(60)).toBe('C4');
    expect(noteName(61)).toBe('C#4');
    expect(noteName(69)).toBe('A4');
    expect(noteName(71)).toBe('B4');
    expect(noteName(84)).toBe('C6');
  });

  it('gives all 12 pitch classes a unique shape and color', () => {
    const octave = Array.from({ length: 12 }, (_, i) => noteVisual(60 + i));
    expect(new Set(octave.map((v) => v.shape)).size).toBe(12);
    expect(new Set(octave.map((v) => v.color)).size).toBe(12);
  });

  it('keeps shape and color the same across octaves', () => {
    for (let pc = 0; pc < 12; pc++) {
      const low = noteVisual(48 + pc);
      const high = noteVisual(72 + pc);
      expect(high.shape).toBe(low.shape);
      expect(high.color).toBe(low.color);
      expect(high.name).toBe(low.name);
    }
  });

  it('uses the agreed shape order', () => {
    expect(noteVisual(60).shape).toBe('circle');
    expect(noteVisual(67).shape).toBe('star');
    expect(noteVisual(71).shape).toBe('octagon');
  });

  it('computes frequencies and sharps', () => {
    expect(noteFrequency(69)).toBeCloseTo(440, 5);
    expect(noteFrequency(60)).toBeCloseTo(261.626, 2);
    expect(isSharp(61)).toBe(true);
    expect(isSharp(64)).toBe(false);
    expect(pitchClass(73)).toBe(1);
    expect(noteColor(60)).toBe('hsl(0 72% 52%)');
  });
});
```

- [ ] **Step 2: Write failing tests `web/src/lib/shapes.test.ts`**

```ts
import { describe, expect, it } from 'vitest';
import { shapeGeometry } from './shapes';
import type { Shape } from './noteVisuals';

const vertexCount = (points: string) => points.trim().split(/\s+/).length;

describe('shapeGeometry', () => {
  it('builds regular polygons with the right vertex counts', () => {
    const expected: [Shape, number][] = [
      ['triangle', 3], ['invertedTriangle', 3], ['square', 4], ['diamond', 4],
      ['pentagon', 5], ['hexagon', 6], ['octagon', 8], ['star', 10],
    ];
    for (const [shape, count] of expected) {
      const geo = shapeGeometry(shape);
      expect(geo.type).toBe('polygon');
      if (geo.type === 'polygon') expect(vertexCount(geo.points)).toBe(count);
    }
  });

  it('points the triangle up and the inverted triangle down', () => {
    const up = shapeGeometry('triangle');
    const down = shapeGeometry('invertedTriangle');
    if (up.type !== 'polygon' || down.type !== 'polygon') throw new Error('expected polygons');
    const firstY = (points: string) => Number(points.split(' ')[0].split(',')[1]);
    expect(firstY(up.points)).toBeLessThan(0);
    expect(firstY(down.points)).toBeGreaterThan(0);
  });

  it('keeps every polygon inside the viewBox', () => {
    for (const shape of ['triangle', 'square', 'star', 'octagon'] as Shape[]) {
      const geo = shapeGeometry(shape);
      if (geo.type !== 'polygon') continue;
      for (const pair of geo.points.split(' ')) {
        const [x, y] = pair.split(',').map(Number);
        expect(Math.abs(x)).toBeLessThanOrEqual(50);
        expect(Math.abs(y)).toBeLessThanOrEqual(50);
      }
    }
  });

  it('handles the non-polygon shapes', () => {
    expect(shapeGeometry('circle').type).toBe('circle');
    expect(shapeGeometry('ring').type).toBe('ring');
    expect(shapeGeometry('cross').type).toBe('path');
    expect(shapeGeometry('crescent').type).toBe('path');
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd web && npm test`
Expected: FAIL, `Failed to resolve import "./noteVisuals"` / `"./shapes"`.

- [ ] **Step 4: Implement `web/src/lib/noteVisuals.ts`**

```ts
/** Lowest and highest notes used in the app (C3–C6), matching engine/src/notes.rs. */
export const LOWEST = 48;
export const HIGHEST = 84;

export type Shape =
  | 'circle' | 'ring' | 'triangle' | 'invertedTriangle' | 'square' | 'diamond'
  | 'pentagon' | 'star' | 'hexagon' | 'cross' | 'crescent' | 'octagon';

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const SHAPES: Shape[] = [
  'circle', 'ring', 'triangle', 'invertedTriangle', 'square', 'diamond',
  'pentagon', 'star', 'hexagon', 'cross', 'crescent', 'octagon',
];

export interface NoteVisual {
  /** Pitch-class name, e.g. "G". */
  name: string;
  /** Name with octave, e.g. "G4". */
  label: string;
  color: string;
  shape: Shape;
}

export function pitchClass(midi: number): number {
  return ((midi % 12) + 12) % 12;
}

export function noteName(midi: number): string {
  return `${NAMES[pitchClass(midi)]}${Math.floor(midi / 12) - 1}`;
}

/** Hue walks the color wheel in 30° steps, so neighboring notes get neighboring colors. */
export function noteColor(midi: number): string {
  return `hsl(${pitchClass(midi) * 30} 72% 52%)`;
}

export function noteFrequency(midi: number): number {
  return 440 * 2 ** ((midi - 69) / 12);
}

export function isSharp(midi: number): boolean {
  return NAMES[pitchClass(midi)].endsWith('#');
}

export function noteVisual(midi: number): NoteVisual {
  const pc = pitchClass(midi);
  return { name: NAMES[pc], label: noteName(midi), color: noteColor(midi), shape: SHAPES[pc] };
}
```

- [ ] **Step 5: Implement `web/src/lib/shapes.ts`**

```ts
import type { Shape } from './noteVisuals';

/** Geometry in the SVG space viewBox="-50 -50 100 100". */
export type ShapeGeometry =
  | { type: 'circle'; r: number }
  | { type: 'ring'; r: number; width: number }
  | { type: 'polygon'; points: string }
  | { type: 'path'; d: string };

/** Vertices around the origin; rotation 0 puts the first vertex straight up. */
function radial(radii: number[], rotationDeg: number): string {
  return radii
    .map((r, i) => {
      const angle = ((rotationDeg + (360 * i) / radii.length - 90) * Math.PI) / 180;
      return `${(r * Math.cos(angle)).toFixed(2)},${(r * Math.sin(angle)).toFixed(2)}`;
    })
    .join(' ');
}

const regular = (sides: number, radius: number, rotationDeg = 0) =>
  radial(Array(sides).fill(radius), rotationDeg);

export function shapeGeometry(shape: Shape): ShapeGeometry {
  switch (shape) {
    case 'circle':
      return { type: 'circle', r: 40 };
    case 'ring':
      return { type: 'ring', r: 34, width: 12 };
    case 'triangle':
      return { type: 'polygon', points: regular(3, 46) };
    case 'invertedTriangle':
      return { type: 'polygon', points: regular(3, 46, 180) };
    case 'square':
      return { type: 'polygon', points: regular(4, 48, 45) };
    case 'diamond':
      return { type: 'polygon', points: regular(4, 46) };
    case 'pentagon':
      return { type: 'polygon', points: regular(5, 44) };
    case 'star':
      return { type: 'polygon', points: radial([46, 19, 46, 19, 46, 19, 46, 19, 46, 19], 0) };
    case 'hexagon':
      return { type: 'polygon', points: regular(6, 44) };
    case 'cross':
      return { type: 'path', d: 'M-13,-42 H13 V-13 H42 V13 H13 V42 H-13 V13 H-42 V-13 H-13 Z' };
    case 'crescent':
      return { type: 'path', d: 'M25,-38 A40,40 0 1 0 25,38 A44,44 0 0 1 25,-38 Z' };
    case 'octagon':
      return { type: 'polygon', points: regular(8, 44, 22.5) };
  }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: all tests pass.

- [ ] **Step 7: Implement `web/src/components/NoteGlyph.svelte`**

```svelte
<script lang="ts">
  import { noteVisual } from '../lib/noteVisuals';
  import { shapeGeometry } from '../lib/shapes';

  let { midi, size = 96, label = true }: { midi: number; size?: number; label?: boolean } = $props();

  const visual = $derived(noteVisual(midi));
  const geo = $derived(shapeGeometry(visual.shape));
</script>

<figure class="glyph" style:width="{size}px">
  <svg viewBox="-50 -50 100 100" width={size} height={size} role="img" aria-label={visual.label}>
    {#if geo.type === 'circle'}
      <circle r={geo.r} fill={visual.color} />
    {:else if geo.type === 'ring'}
      <circle r={geo.r} fill="none" stroke={visual.color} stroke-width={geo.width} />
    {:else if geo.type === 'polygon'}
      <polygon points={geo.points} fill={visual.color} />
    {:else}
      <path d={geo.d} fill={visual.color} />
    {/if}
  </svg>
  {#if label}
    <figcaption>{visual.label}</figcaption>
  {/if}
</figure>

<style>
  .glyph {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }
  svg {
    display: block;
  }
  figcaption {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
```

- [ ] **Step 8: Type-check**

Run: `cd web && npm run check`
Expected: `0 errors`.

- [ ] **Step 9: Commit**

```bash
git add web/src/lib/noteVisuals.ts web/src/lib/noteVisuals.test.ts web/src/lib/shapes.ts web/src/lib/shapes.test.ts web/src/components/NoteGlyph.svelte
git commit -m "feat(web): note colors, shapes and NoteGlyph"
```

---

### Task 9: Versioned localStorage persistence

**Files:**
- Create: `web/src/lib/storage.ts`, `web/src/lib/storage.test.ts`

**Interfaces:**
- Produces: `STORAGE_KEY = 'pitch-trainer:v1'`; `loadEngineState(storage?: Pick<Storage, 'getItem'>): string | null`; `saveEngineState(engineJson: string, storage?: Pick<Storage, 'setItem'>): void`. Both default to `localStorage` and never throw.

- [ ] **Step 1: Write failing tests `web/src/lib/storage.test.ts`**

```ts
import { describe, expect, it } from 'vitest';
import { STORAGE_KEY, loadEngineState, saveEngineState } from './storage';

function memoryStorage(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (key: string) => data.get(key) ?? null,
    setItem: (key: string, value: string) => void data.set(key, value),
    data,
  };
}

describe('storage', () => {
  it('round-trips engine state', () => {
    const storage = memoryStorage();
    saveEngineState('{"upDown":{"level":3}}', storage);
    expect(JSON.parse(storage.data.get(STORAGE_KEY)!)).toEqual({
      version: 1,
      engine: '{"upDown":{"level":3}}',
    });
    expect(loadEngineState(storage)).toBe('{"upDown":{"level":3}}');
  });

  it('returns null when nothing is saved', () => {
    expect(loadEngineState(memoryStorage())).toBeNull();
  });

  it('returns null for corrupt JSON', () => {
    expect(loadEngineState(memoryStorage({ [STORAGE_KEY]: '{oops' }))).toBeNull();
  });

  it('returns null for another version', () => {
    const saved = JSON.stringify({ version: 2, engine: '{}' });
    expect(loadEngineState(memoryStorage({ [STORAGE_KEY]: saved }))).toBeNull();
  });

  it('returns null when engine is not a string', () => {
    const saved = JSON.stringify({ version: 1, engine: { level: 3 } });
    expect(loadEngineState(memoryStorage({ [STORAGE_KEY]: saved }))).toBeNull();
  });

  it('swallows storage errors', () => {
    const broken = {
      getItem: () => { throw new Error('denied'); },
      setItem: () => { throw new Error('full'); },
    };
    expect(loadEngineState(broken)).toBeNull();
    expect(() => saveEngineState('{}', broken)).not.toThrow();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test storage`
Expected: FAIL, `Failed to resolve import "./storage"`.

- [ ] **Step 3: Implement `web/src/lib/storage.ts`**

```ts
export const STORAGE_KEY = 'pitch-trainer:v1';
const VERSION = 1;

function defaultStorage(): Storage | undefined {
  try {
    return globalThis.localStorage;
  } catch {
    return undefined;
  }
}

/** Returns saved engine state JSON, or null if missing, corrupt, or from another version. */
export function loadEngineState(
  storage: Pick<Storage, 'getItem'> | undefined = defaultStorage(),
): string | null {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (!raw) return null;
    const saved = JSON.parse(raw);
    if (saved?.version !== VERSION || typeof saved.engine !== 'string') return null;
    return saved.engine;
  } catch {
    return null;
  }
}

export function saveEngineState(
  engineJson: string,
  storage: Pick<Storage, 'setItem'> | undefined = defaultStorage(),
): void {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify({ version: VERSION, engine: engineJson }));
  } catch {
    // Storage full or blocked: progress just isn't saved this time.
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/storage.ts web/src/lib/storage.test.ts
git commit -m "feat(web): versioned localStorage persistence"
```

---

### Task 10: Audio output, waveform, Explore mode

**Files:**
- Create: `web/src/lib/audio.ts`, `web/src/components/Waveform.svelte`, `web/src/components/Explore.svelte`
- Modify: `web/src/App.svelte` (temporary: a start gate that mounts Explore, so it can be tried; Task 12 replaces it)

**Interfaces:**
- Consumes: `Trainer` (Task 7), `NoteGlyph`, `noteVisuals` (Task 8)
- Produces:
  - `audio.ts`: `class AudioOut { readonly context: AudioContext; readonly analyser: AnalyserNode; get sampleRate(): number; resume(): Promise<void>; play(samples: Float32Array, cues?: number[], onCue?: (index: number) => void): Promise<void>; stop(): void }`. `play` stops any current playback. `cues` are seconds from the start, and `onCue(i)` fires at each. The returned promise resolves when playback ends or is stopped.
  - `Waveform.svelte` props: `{ analyser: AnalyserNode; color?: string /* a concrete CSS color, not var() */ }`
  - `Explore.svelte` props: `{ trainer: Trainer; audio: AudioOut }`

- [ ] **Step 1: Implement `web/src/lib/audio.ts`**

```ts
/** Owns the AudioContext: plays engine-rendered buffers and exposes an analyser for the scope. */
export class AudioOut {
  readonly context: AudioContext;
  readonly analyser: AnalyserNode;
  private current: { source: AudioBufferSourceNode; timers: number[] } | null = null;

  constructor() {
    this.context = new AudioContext();
    this.analyser = this.context.createAnalyser();
    this.analyser.fftSize = 2048;
    this.analyser.connect(this.context.destination);
  }

  get sampleRate(): number {
    return this.context.sampleRate;
  }

  /** Must be called from a user gesture before the first sound. */
  async resume(): Promise<void> {
    if (this.context.state !== 'running') await this.context.resume();
  }

  play(samples: Float32Array, cues: number[] = [], onCue?: (index: number) => void): Promise<void> {
    this.stop();
    const buffer = this.context.createBuffer(1, samples.length, this.context.sampleRate);
    buffer.getChannelData(0).set(samples);
    const source = this.context.createBufferSource();
    source.buffer = buffer;
    source.connect(this.analyser);
    const timers = cues.map((seconds, i) => window.setTimeout(() => onCue?.(i), seconds * 1000));
    const playing = { source, timers };
    this.current = playing;
    return new Promise((resolve) => {
      source.onended = () => {
        timers.forEach(clearTimeout);
        if (this.current === playing) this.current = null;
        resolve();
      };
      source.start();
    });
  }

  stop(): void {
    const playing = this.current;
    if (!playing) return;
    this.current = null;
    playing.timers.forEach(clearTimeout);
    playing.source.stop();
  }
}
```

- [ ] **Step 2: Implement `web/src/components/Waveform.svelte`**

```svelte
<script lang="ts">
  let { analyser, color = '#888888' }: { analyser: AnalyserNode; color?: string } = $props();

  let canvas: HTMLCanvasElement;

  $effect(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const data = new Float32Array(analyser.fftSize);
    const stroke = color;
    let frame = 0;

    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const width = Math.round(canvas.clientWidth * dpr);
      const height = Math.round(canvas.clientHeight * dpr);
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      analyser.getFloatTimeDomainData(data);
      ctx.clearRect(0, 0, width, height);
      ctx.lineWidth = 2 * dpr;
      ctx.strokeStyle = stroke;
      ctx.beginPath();
      for (let i = 0; i < data.length; i++) {
        const x = (i / (data.length - 1)) * width;
        const y = (0.5 - data[i] * 0.5) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      frame = requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(frame);
  });
</script>

<canvas bind:this={canvas} class="waveform" aria-hidden="true"></canvas>

<style>
  .waveform {
    display: block;
    width: 100%;
    height: 120px;
    border-radius: 12px;
    background: var(--surface-2);
  }
</style>
```

- [ ] **Step 3: Implement `web/src/components/Explore.svelte`**

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { AudioOut } from '../lib/audio';
  import type { Trainer } from '../lib/trainer';
  import { HIGHEST, LOWEST, isSharp, noteColor, noteFrequency, noteName } from '../lib/noteVisuals';
  import NoteGlyph from './NoteGlyph.svelte';
  import Waveform from './Waveform.svelte';

  let { trainer, audio }: { trainer: Trainer; audio: AudioOut } = $props();

  /** Computer keys for one octave, C to C. */
  const LETTERS = ['a', 'w', 's', 'e', 'd', 'f', 't', 'g', 'y', 'h', 'u', 'j', 'k'];

  const allKeys = Array.from({ length: HIGHEST - LOWEST + 1 }, (_, i) => LOWEST + i);
  const whiteKeys = allKeys.filter((midi) => !isSharp(midi));
  const blackKeys = allKeys
    .filter(isSharp)
    .map((midi) => ({ midi, whitesBefore: whiteKeys.filter((w) => w < midi).length }));

  let octaveBase = $state(60);
  let current = $state<number | null>(null);

  onDestroy(() => audio.stop());

  function letterFor(midi: number): string | null {
    const offset = midi - octaveBase;
    return offset >= 0 && offset < LETTERS.length ? LETTERS[offset].toUpperCase() : null;
  }

  function playNote(midi: number) {
    current = midi;
    void audio.play(trainer.noteAudio(midi));
  }

  function playScale() {
    const top = Math.min(octaveBase + 12, HIGHEST);
    const notes = trainer.scaleNotes(octaveBase, top);
    void audio.play(
      trainer.scaleAudio(octaveBase, top),
      notes.map((n) => n.onset),
      (i) => (current = notes[i].midi),
    );
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
    const key = event.key.toLowerCase();
    if (key === 'z') octaveBase = Math.max(LOWEST, octaveBase - 12);
    else if (key === 'x') octaveBase = Math.min(HIGHEST - 12, octaveBase + 12);
    else {
      const offset = LETTERS.indexOf(key);
      if (offset >= 0) playNote(octaveBase + offset);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="explore">
  <section class="display">
    <div class="now">
      {#if current !== null}
        <NoteGlyph midi={current} size={132} />
        <p class="freq">{noteFrequency(current).toFixed(1)} Hz</p>
      {:else}
        <p class="hint">Tap a key, or play with A–K on your keyboard.</p>
      {/if}
    </div>
    <Waveform analyser={audio.analyser} color={current !== null ? noteColor(current) : '#888888'} />
  </section>

  <div class="controls">
    <button class="btn" onclick={playScale}>Play scale from {noteName(octaveBase)}</button>
    <span class="octave">Keys A–K play {noteName(octaveBase)}–{noteName(octaveBase + 12)} · Z / X shift octave</span>
  </div>

  <div class="keyboard-scroll">
    <div class="keyboard" style:--whites={whiteKeys.length}>
      {#each whiteKeys as midi (midi)}
        <button
          class="key white"
          class:active={current === midi}
          aria-label={noteName(midi)}
          onclick={() => playNote(midi)}
        >
          <NoteGlyph {midi} size={18} label={false} />
          <span class="letter">{letterFor(midi) ?? ''}</span>
        </button>
      {/each}
      {#each blackKeys as { midi, whitesBefore } (midi)}
        <button
          class="key black"
          class:active={current === midi}
          style:--pos={whitesBefore}
          style:--tint={noteColor(midi)}
          aria-label={noteName(midi)}
          onclick={() => playNote(midi)}
        >
          <span class="letter">{letterFor(midi) ?? ''}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .explore {
    display: grid;
    gap: 1.25rem;
  }
  .display {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .now {
    min-height: 190px;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 0.25rem;
  }
  .freq,
  .hint,
  .octave {
    margin: 0;
    color: var(--muted);
  }
  .freq {
    font-variant-numeric: tabular-nums;
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem 1rem;
  }
  .keyboard-scroll {
    overflow-x: auto;
    padding-bottom: 0.25rem;
  }
  .keyboard {
    --white-w: calc(100% / var(--whites));
    position: relative;
    display: flex;
    min-width: 600px;
    height: 170px;
  }
  .key {
    border: 1px solid #0003;
    padding: 0;
  }
  .key.white {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    padding-bottom: 8px;
    background: #fbfaf7;
    color: #444;
    border-radius: 0 0 6px 6px;
  }
  .key.black {
    position: absolute;
    top: 0;
    z-index: 1;
    width: calc(var(--white-w) * 0.62);
    left: calc(var(--white-w) * var(--pos) - var(--white-w) * 0.31);
    height: 60%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 6px;
    background: color-mix(in srgb, var(--tint) 55%, #111);
    color: #fff;
    border-radius: 0 0 5px 5px;
  }
  .key.active {
    box-shadow: inset 0 -6px 0 var(--accent);
  }
  .letter {
    font-size: 0.7rem;
    font-weight: 600;
    min-height: 1em;
  }
</style>
```

- [ ] **Step 4: Temporary `web/src/App.svelte` to try Explore**

```svelte
<script lang="ts">
  import { AudioOut } from './lib/audio';
  import { Trainer } from './lib/trainer';
  import Explore from './components/Explore.svelte';

  let audio = $state.raw<AudioOut | null>(null);
  let trainer = $state.raw<Trainer | null>(null);

  async function start() {
    const out = new AudioOut();
    await out.resume();
    trainer = await Trainer.create(out.sampleRate, null);
    audio = out;
  }
</script>

<main style="max-width: 900px; margin: 0 auto; padding: 16px;">
  {#if trainer && audio}
    <Explore {trainer} {audio} />
  {:else}
    <button class="btn primary" onclick={start}>Tap to start</button>
  {/if}
</main>
```

- [ ] **Step 5: Type-check**

Run: `cd web && npm run check`
Expected: `0 errors`.

- [ ] **Step 6: Try it in a browser**

Run: `cd web && npm run dev`, open `http://localhost:5173`, and click **Tap to start**.
Expected:
- Clicking any key plays a clean tone with no clicks, and the big glyph + frequency update. For example, A4 shows `440.0 Hz`, a purple cross, and the label `A4`.
- The waveform draws in the note's color while it sounds and goes flat after.
- Keys `A`…`K` play C4…C5, `X` shifts the letter hints to C5, and `Z` shifts them down.
- **Play scale** steps the glyph through 13 notes in time with the audio.
- No console errors.

- [ ] **Step 7: Commit**

```bash
git add web/src/lib/audio.ts web/src/components/Waveform.svelte web/src/components/Explore.svelte web/src/App.svelte
git commit -m "feat(web): audio output, live waveform and Explore keyboard"
```

---

### Task 11: Test runner (all three tests)

**Files:**
- Create: `web/src/lib/answers.ts`, `web/src/lib/answers.test.ts`, `web/src/components/AnswerButtons.svelte`, `web/src/components/TestRunner.svelte`

**Interfaces:**
- Consumes: `Trainer`, `AudioOut`, `NoteGlyph`, `Waveform`, `noteColor`, types
- Produces:
  - `answers.ts`: `interface AnswerOption { value: number; label: string; key: string /* KeyboardEvent.key */ }`, `answerOptions(kind: TestKind, count: number): AnswerOption[]`, `TEST_INFO: Record<TestKind, { title: string; blurb: string }>`
  - `AnswerButtons.svelte` props: `{ options: AnswerOption[]; disabled: boolean; chosen: number | null; correct: number | null; onChoose: (value: number) => void }`
  - `TestRunner.svelte` props: `{ trainer: Trainer; audio: AudioOut; kind: TestKind; onAnswered: () => void }`. The parent must remount it (`{#key kind}`) when `kind` changes.

- [ ] **Step 1: Write failing tests `web/src/lib/answers.test.ts`**

```ts
import { describe, expect, it } from 'vitest';
import { TEST_INFO, answerOptions } from './answers';

describe('answerOptions', () => {
  it('up/down maps Higher=0 and Lower=1 to arrow keys', () => {
    expect(answerOptions('upDown', 2)).toEqual([
      { value: 0, label: 'Higher', key: 'ArrowUp' },
      { value: 1, label: 'Lower', key: 'ArrowDown' },
    ]);
  });

  it('pick two maps First=0 and Second=1 to 1/2', () => {
    expect(answerOptions('pickTwo', 2).map((o) => [o.value, o.label, o.key])).toEqual([
      [0, 'First', '1'],
      [1, 'Second', '2'],
    ]);
  });

  it('sequence has one numbered option per position', () => {
    const options = answerOptions('sequence', 6);
    expect(options.map((o) => o.label)).toEqual(['1', '2', '3', '4', '5', '6']);
    expect(options.map((o) => o.value)).toEqual([0, 1, 2, 3, 4, 5]);
    expect(new Set(options.map((o) => o.key)).size).toBe(6);
  });

  it('has a title for every test', () => {
    expect(Object.keys(TEST_INFO).sort()).toEqual(['pickTwo', 'sequence', 'upDown']);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test answers`
Expected: FAIL, `Failed to resolve import "./answers"`.

- [ ] **Step 3: Implement `web/src/lib/answers.ts`**

```ts
import type { TestKind } from './types';

export interface AnswerOption {
  /** Answer code sent to the engine. */
  value: number;
  label: string;
  /** KeyboardEvent.key that picks this option. */
  key: string;
}

export const TEST_INFO: Record<TestKind, { title: string; blurb: string }> = {
  upDown: { title: 'Up / Down', blurb: 'Two notes play. Was the second one higher or lower?' },
  pickTwo: { title: 'Pick Two', blurb: 'Two notes play, then one of them again. Which one was it?' },
  sequence: { title: 'Sequence', blurb: 'A run of notes plays, then one of them again. Which position was it?' },
};

export function answerOptions(kind: TestKind, count: number): AnswerOption[] {
  switch (kind) {
    case 'upDown':
      return [
        { value: 0, label: 'Higher', key: 'ArrowUp' },
        { value: 1, label: 'Lower', key: 'ArrowDown' },
      ];
    case 'pickTwo':
      return [
        { value: 0, label: 'First', key: '1' },
        { value: 1, label: 'Second', key: '2' },
      ];
    case 'sequence':
      return Array.from({ length: count }, (_, i) => ({ value: i, label: String(i + 1), key: String(i + 1) }));
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: all pass.

- [ ] **Step 5: Implement `web/src/components/AnswerButtons.svelte`**

```svelte
<script lang="ts">
  import type { AnswerOption } from '../lib/answers';

  let {
    options,
    disabled,
    chosen,
    correct,
    onChoose,
  }: {
    options: AnswerOption[];
    disabled: boolean;
    chosen: number | null;
    correct: number | null;
    onChoose: (value: number) => void;
  } = $props();
</script>

<div class="answers">
  {#each options as option (option.value)}
    <button
      class="btn answer"
      class:right={correct === option.value}
      class:wrong={chosen === option.value && correct !== null && correct !== option.value}
      {disabled}
      onclick={() => onChoose(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .answers {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.6rem;
  }
  .answer {
    min-width: 4.5rem;
    font-size: 1.1rem;
    font-weight: 600;
  }
  .answer.right {
    border-color: var(--good);
    box-shadow: inset 0 0 0 2px var(--good);
    opacity: 1;
  }
  .answer.wrong {
    border-color: var(--bad);
    box-shadow: inset 0 0 0 2px var(--bad);
    opacity: 1;
  }
</style>
```

- [ ] **Step 6: Implement `web/src/components/TestRunner.svelte`**

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { AudioOut } from '../lib/audio';
  import type { Trainer } from '../lib/trainer';
  import type { AnswerResult, Progress, RoundView, TestKind } from '../lib/types';
  import { TEST_INFO, answerOptions } from '../lib/answers';
  import { noteColor } from '../lib/noteVisuals';
  import AnswerButtons from './AnswerButtons.svelte';
  import NoteGlyph from './NoteGlyph.svelte';
  import Waveform from './Waveform.svelte';

  type Phase = 'ready' | 'playing' | 'answering' | 'revealing' | 'revealed';
  const MAX_REPLAYS = 2;
  const MAX_LEVEL = 8;

  let {
    trainer,
    audio,
    kind,
    onAnswered,
  }: { trainer: Trainer; audio: AudioOut; kind: TestKind; onAnswered: () => void } = $props();

  let phase = $state<Phase>('ready');
  let view = $state<RoundView | null>(null);
  let result = $state<AnswerResult | null>(null);
  let chosen = $state<number | null>(null);
  /** Index of the slot currently sounding. */
  let cue = $state<number | null>(null);
  /** How many slots have shown their glyph during the reveal. */
  let revealed = $state(0);
  let replaysLeft = $state(MAX_REPLAYS);
  let progress = $state<Progress>(trainer.progress(kind));

  const info = $derived(TEST_INFO[kind]);
  const options = $derived(view ? answerOptions(kind, view.options) : []);
  const showVisuals = $derived(phase === 'revealing' || phase === 'revealed');
  const accuracy = $derived(
    progress.attempts ? Math.round((100 * progress.correct) / progress.attempts) : null,
  );
  const cueColor = $derived(result && cue !== null ? noteColor(result.notes[cue].midi) : '#888888');

  onDestroy(() => audio.stop());

  function slotLabel(index: number, isTarget: boolean): string {
    if (isTarget) return '?';
    if (kind === 'sequence') return String(index + 1);
    return index === 0 ? '1st' : '2nd';
  }

  async function playChallenge() {
    if (!view) return;
    phase = 'playing';
    await audio.play(trainer.roundAudio(), view.onsets, (i) => (cue = i));
    cue = null;
    if (phase === 'playing') phase = 'answering';
  }

  function nextRound() {
    view = trainer.newRound(kind);
    result = null;
    chosen = null;
    revealed = 0;
    replaysLeft = MAX_REPLAYS;
    void playChallenge();
  }

  function replay() {
    if (phase !== 'answering' || replaysLeft === 0) return;
    replaysLeft -= 1;
    void playChallenge();
  }

  function choose(value: number) {
    if (phase !== 'answering') return;
    chosen = value;
    result = trainer.answer(value);
    progress = result.progress;
    onAnswered();
    void playReveal();
  }

  async function playReveal() {
    if (!result) return;
    const notes = result.notes;
    phase = 'revealing';
    revealed = 0;
    await audio.play(
      trainer.roundAudio(),
      notes.map((n) => n.onset),
      (i) => {
        cue = i;
        revealed = i + 1;
      },
    );
    cue = null;
    revealed = notes.length;
    if (phase === 'revealing') phase = 'revealed';
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
    const onButton = (event.target as HTMLElement | null)?.closest('button');
    if (event.key === ' ') {
      if (onButton) return; // let the focused button handle Space itself
      if (phase === 'ready' || phase === 'revealed') {
        event.preventDefault();
        nextRound();
      }
      return;
    }
    if (event.key.toLowerCase() === 'r') {
      replay();
      return;
    }
    const option = options.find((o) => o.key === event.key);
    if (option && phase === 'answering') {
      event.preventDefault();
      choose(option.value);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="runner">
  <header class="head">
    <div>
      <h2>{info.title}</h2>
      <p class="blurb">{info.blurb}</p>
    </div>
    <dl class="stats">
      <div><dt>Level</dt><dd>{progress.level} / {MAX_LEVEL}</dd></div>
      <div><dt>Best</dt><dd>{progress.bestLevel}</dd></div>
      <div><dt>Accuracy</dt><dd>{accuracy === null ? '–' : `${accuracy}%`}</dd></div>
      <div>
        <dt>Streak</dt>
        <dd class="streak" aria-label="{progress.streak} of 3 toward next level">
          {#each [0, 1, 2] as i (i)}<span class:on={i < progress.streak}></span>{/each}
        </dd>
      </div>
    </dl>
  </header>

  <section class="stage">
    {#if view}
      <div class="slots">
        {#each view.onsets as _, i (i)}
          {@const isTarget = view.hasTarget && i === view.onsets.length - 1}
          {#if isTarget}<span class="again">again</span>{/if}
          <div
            class="slot"
            class:active={cue === i}
            class:source={result !== null && kind !== 'upDown' && !isTarget && i === result.correctAnswer}
          >
            {#if showVisuals && result && i < revealed}
              <NoteGlyph midi={result.notes[i].midi} size={52} />
            {:else}
              <span class="placeholder">{slotLabel(i, isTarget)}</span>
            {/if}
          </div>
        {/each}
      </div>
      <p class="prompt">
        {#if phase === 'playing'}
          Listen…
        {:else if result}
          {#if result.correct}
            <strong class="good">Correct!</strong>
          {:else}
            <strong class="bad">Not quite.</strong> The answer was {options[result.correctAnswer].label}.
          {/if}
        {:else}
          {view.prompt}
        {/if}
      </p>
    {:else}
      <p class="prompt">Press Start (or Space) when you're ready.</p>
    {/if}

    {#if showVisuals}
      <Waveform analyser={audio.analyser} color={cueColor} />
    {/if}
  </section>

  <div class="actions">
    {#if phase === 'ready'}
      <button class="btn primary" onclick={nextRound}>Start</button>
    {:else if phase === 'playing' || phase === 'answering'}
      <AnswerButtons {options} disabled={phase !== 'answering'} {chosen} correct={null} onChoose={choose} />
      <button class="btn" disabled={phase !== 'answering' || replaysLeft === 0} onclick={replay}>
        Replay ({replaysLeft})
      </button>
    {:else}
      <AnswerButtons {options} disabled={true} {chosen} correct={result?.correctAnswer ?? null} onChoose={choose} />
      <div class="row">
        <button class="btn" disabled={phase !== 'revealed'} onclick={() => void playReveal()}>Hear again</button>
        <button class="btn primary" disabled={phase !== 'revealed'} onclick={nextRound}>Next</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .runner {
    display: grid;
    gap: 1.25rem;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }
  h2 {
    margin: 0;
  }
  .blurb {
    margin: 0.25rem 0 0;
    color: var(--muted);
  }
  .stats {
    display: flex;
    gap: 1.25rem;
    margin: 0;
  }
  .stats div {
    display: grid;
    gap: 0.1rem;
  }
  dt {
    font-size: 0.75rem;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  dd {
    margin: 0;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .streak {
    display: flex;
    gap: 4px;
    align-items: center;
    height: 1.45em;
  }
  .streak span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--placeholder);
  }
  .streak span.on {
    background: var(--good);
  }
  .stage {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .slots {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: 0.6rem;
    min-height: 96px;
  }
  .slot {
    width: 72px;
    height: 88px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    border: 2px solid transparent;
    background: var(--surface-2);
    transition: transform 120ms ease, border-color 120ms ease;
  }
  .slot.active {
    transform: scale(1.08);
    border-color: var(--accent);
  }
  .slot.source {
    border-color: var(--good);
  }
  .placeholder {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--muted);
  }
  .again {
    font-size: 0.8rem;
    color: var(--muted);
    padding: 0 0.25rem;
  }
  .prompt {
    margin: 0;
    text-align: center;
    min-height: 1.5em;
  }
  .good {
    color: var(--good);
  }
  .bad {
    color: var(--bad);
  }
  .actions {
    display: grid;
    justify-items: center;
    gap: 0.75rem;
  }
  .row {
    display: flex;
    gap: 0.75rem;
  }
</style>
```

- [ ] **Step 7: Type-check**

Run: `cd web && npm run check`
Expected: `0 errors`. (A `state_referenced_locally` warning for `trainer.progress(kind)` is acceptable, because the parent remounts on kind change.)

- [ ] **Step 8: Commit** (it gets wired into App in Task 12)

```bash
git add web/src/lib/answers.ts web/src/lib/answers.test.ts web/src/components/AnswerButtons.svelte web/src/components/TestRunner.svelte
git commit -m "feat(web): test runner with challenge, answer and reveal phases"
```

---

### Task 12: App shell: start gate, navigation, persistence, errors

**Files:**
- Modify: `web/src/App.svelte` (full replacement)
- Create: `README.md`

**Interfaces:**
- Consumes: `AudioOut`, `Trainer`, `loadEngineState`/`saveEngineState`, `TEST_INFO`, `Explore`, `TestRunner`

- [ ] **Step 1: Replace `web/src/App.svelte`**

```svelte
<script lang="ts">
  import { AudioOut } from './lib/audio';
  import { Trainer } from './lib/trainer';
  import { loadEngineState, saveEngineState } from './lib/storage';
  import { TEST_INFO } from './lib/answers';
  import type { TestKind } from './lib/types';
  import Explore from './components/Explore.svelte';
  import TestRunner from './components/TestRunner.svelte';

  type Mode = 'explore' | TestKind;
  const MODES: { id: Mode; label: string }[] = [
    { id: 'explore', label: 'Explore' },
    { id: 'upDown', label: TEST_INFO.upDown.title },
    { id: 'pickTwo', label: TEST_INFO.pickTwo.title },
    { id: 'sequence', label: TEST_INFO.sequence.title },
  ];

  let status = $state<'gate' | 'loading' | 'ready' | 'error'>('gate');
  let audio = $state.raw<AudioOut | null>(null);
  let trainer = $state.raw<Trainer | null>(null);
  let mode = $state<Mode>('explore');

  async function start() {
    status = 'loading';
    try {
      const out = new AudioOut();
      await out.resume();
      trainer = await Trainer.create(out.sampleRate, loadEngineState());
      audio = out;
      status = 'ready';
    } catch (error) {
      console.error(error);
      status = 'error';
    }
  }

  function persist() {
    if (trainer) saveEngineState(trainer.stateJson());
  }
</script>

<div class="app">
  <header class="top">
    <h1>Pitch Trainer</h1>
    {#if status === 'ready'}
      <nav aria-label="Mode">
        {#each MODES as m (m.id)}
          <button class="tab" class:current={mode === m.id} aria-current={mode === m.id} onclick={() => (mode = m.id)}>
            {m.label}
          </button>
        {/each}
      </nav>
    {/if}
  </header>

  <main>
    {#if status === 'ready' && trainer && audio}
      {#if mode === 'explore'}
        <Explore {trainer} {audio} />
      {:else}
        {#key mode}
          <TestRunner {trainer} {audio} kind={mode} onAnswered={persist} />
        {/key}
      {/if}
    {:else if status === 'error'}
      <p class="card">Couldn't load the audio engine. Try a current version of Chrome, Firefox, or Safari.</p>
    {:else}
      <section class="card gate">
        <p>
          Train your ear with three tests: <strong>Up / Down</strong>, <strong>Pick Two</strong>, and
          <strong>Sequence</strong>. Each one gets harder as you improve. Use <strong>Explore</strong> to learn
          each note's color and shape.
        </p>
        <button class="btn primary" disabled={status === 'loading'} onclick={start}>
          {status === 'loading' ? 'Starting…' : 'Tap to start'}
        </button>
      </section>
    {/if}
  </main>
</div>

<style>
  .app {
    max-width: 920px;
    margin: 0 auto;
    padding: 16px;
    display: grid;
    gap: 1.25rem;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  h1 {
    margin: 0;
    font-size: 1.4rem;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding: 0.25rem;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .tab {
    border: 0;
    background: transparent;
    padding: 0.45rem 0.85rem;
    border-radius: 9px;
  }
  .tab.current {
    background: var(--surface);
    font-weight: 600;
    box-shadow: 0 1px 2px #0002;
  }
  .card {
    margin: 0;
    padding: 1.5rem;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .gate {
    display: grid;
    gap: 1rem;
    justify-items: start;
  }
  .gate p {
    margin: 0;
  }
</style>
```

- [ ] **Step 2: Create `README.md`**

````markdown
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
````

- [ ] **Step 3: Type-check, test and build**

Run: `cd engine && cargo test && cd ../web && npm test && npm run check && npm run build`
Expected: all Rust tests pass, all Vitest tests pass, svelte-check shows `0 errors`, and the Vite build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/App.svelte README.md
git commit -m "feat(web): app shell with start gate, navigation and saved progress"
```

---

### Task 13: End-to-end verification and spec sync

**Files:**
- Modify: `docs/superpowers/specs/2026-09-28-pitch-trainer-design.md`

- [ ] **Step 1: Run the app**

Run: `cd web && npm run dev`, then open `http://localhost:5173`.

- [ ] **Step 2: Manual checklist (every item must pass)**

- [ ] The gate shows **Tap to start**. After clicking, the tabs appear and Explore works as in Task 10, Step 6.
- [ ] **Up / Down:** Start plays two notes with the grey `1st`/`2nd` slots pulsing in time. There are **no glyphs, colors or waveform** until you answer. Buttons are disabled while audio plays. ↑/↓ keys answer. After answering, it shows Correct / Not quite, and the reveal replays with glyphs popping in on each note and a colored waveform. **Next** and Space start a new round.
- [ ] **Pick Two:** Three slots (`1st`, `2nd`, "again", `?`). Keys `1`/`2` answer. The reveal outlines the correct source slot in green.
- [ ] **Sequence:** Level 1 has 3 numbered slots + `?`. Number keys answer.
- [ ] **Replay (2)** counts down to 0 and then disables. `R` also replays.
- [ ] Answering 3 in a row correctly raises Level by 1, and a wrong answer lowers it (never below 1). The streak dots fill and reset.
- [ ] Switching tabs mid-playback stops the audio immediately.
- [ ] Reloading the page and starting again shows the same levels/accuracy per test.
- [ ] In devtools, `localStorage.setItem('pitch-trainer:v1', '{bad')` then reload → the app starts fresh with no error shown.
- [ ] At a phone-width viewport (375 px) the page has no horizontal scroll (the keyboard scrolls inside its own box). Slots wrap and buttons stay usable.
- [ ] Dark mode (OS setting): text and surfaces stay readable.
- [ ] No console errors throughout.

- [ ] **Step 3: Sync spec with intentional deviations**

In `docs/superpowers/specs/2026-09-28-pitch-trainer-design.md`:
- In **Engine API**, change `seed: u64` to `seed: u32` and note: "(u32 so JS passes a number, not a BigInt; widened to u64 internally)". Change method names to their JS camelCase forms (`newRound`, `roundAudio`, `noteAudio`, `scaleNotes`, `scaleAudio`, `stateJson`), and add `progress(kind)` and `scaleNotes(from, to)`.
- In **Sequence › Generation**, replace the rejection-sampling sentence with: "Generation builds sorted gaps directly: one gap is exactly `d`, the others are `d` plus 0–4 random extra semitones within the range budget. The notes are then shuffled. This always succeeds."
- In **Architecture**, replace the `UpDownPanel / PickTwoPanel / SequencePanel` lines with `AnswerButtons.svelte` (labels from `lib/answers.ts`). Add `lib/trainer.ts`, `lib/shapes.ts` and `lib/types.ts`, and move note-name formatting tests from the Rust list to the Web list.
- In **Test types › common flow**, note that JSON field names are camelCase and kinds are `upDown | pickTwo | sequence`.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-09-28-pitch-trainer-design.md
git commit -m "docs: sync spec with implementation decisions"
```
