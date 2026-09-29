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
