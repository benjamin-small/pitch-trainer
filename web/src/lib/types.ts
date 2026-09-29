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
