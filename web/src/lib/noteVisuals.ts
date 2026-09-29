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
