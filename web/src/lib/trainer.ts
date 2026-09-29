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
