import { noteName } from './noteVisuals';

/** MIDI note 60, written C4 (see engine/src/notes.rs). */
export const MIDDLE_C = 60;

export interface PianoBinding {
  /** Physical key, as `KeyboardEvent.code`, so Shift and layout casing don't matter. */
  code: string;
  /** Hint shown on the piano key. */
  hint: string;
}

/**
 * Computer keys for one octave, C to C, indexed by semitone offset.
 * White notes rest on the home row (A S D F / J K L ;), leaving G and H
 * free; sharps sit on the row above, between the white notes they split.
 */
export const PIANO_BINDINGS: readonly PianoBinding[] = [
  { code: 'KeyA', hint: 'A' }, // C
  { code: 'KeyW', hint: 'W' }, // C#
  { code: 'KeyS', hint: 'S' }, // D
  { code: 'KeyE', hint: 'E' }, // D#
  { code: 'KeyD', hint: 'D' }, // E
  { code: 'KeyF', hint: 'F' }, // F
  { code: 'KeyU', hint: 'U' }, // F#
  { code: 'KeyJ', hint: 'J' }, // G
  { code: 'KeyI', hint: 'I' }, // G#
  { code: 'KeyK', hint: 'K' }, // A
  { code: 'KeyO', hint: 'O' }, // A#
  { code: 'KeyL', hint: 'L' }, // B
  { code: 'Semicolon', hint: ';' }, // C
];

/**
 * The physical key for a key event, as a `KeyboardEvent.code`. Falls back to
 * the typed character when `code` is empty (some virtual keyboards), treating
 * `:` as the same key as `;`.
 */
export function physicalKey(event: { code: string; key: string }): string {
  if (event.code) return event.code;
  if (event.key === ';' || event.key === ':') return 'Semicolon';
  return /^[a-z]$/i.test(event.key) ? `Key${event.key.toUpperCase()}` : '';
}

/** Semitone offset above the octave base for a physical key, or null if it plays nothing. */
export function offsetForCode(code: string): number | null {
  const offset = PIANO_BINDINGS.findIndex((b) => b.code === code);
  return offset >= 0 ? offset : null;
}

/** Shortcut hint for a note given the current octave base, or null if it has none. */
export function hintFor(midi: number, octaveBase: number): string | null {
  return PIANO_BINDINGS[midi - octaveBase]?.hint ?? null;
}

/** Note name for played-note feedback, calling out middle C. */
export function noteDisplayName(midi: number): string {
  return midi === MIDDLE_C ? `Middle C (${noteName(midi)})` : noteName(midi);
}
