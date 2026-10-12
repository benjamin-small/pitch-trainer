import { describe, expect, it } from 'vitest';
import { isSharp, noteName } from './noteVisuals';
import { MIDDLE_C, PIANO_BINDINGS, hintFor, noteDisplayName, offsetForCode, physicalKey } from './pianoKeys';

describe('pianoKeys', () => {
  it('plays the white notes on the home row in ascending order', () => {
    const whites = PIANO_BINDINGS.filter((_, offset) => !isSharp(60 + offset)).map((b) => b.hint);
    expect(whites).toEqual(['A', 'S', 'D', 'F', 'J', 'K', 'L', ';']);
  });

  it('leaves G and H unbound', () => {
    expect(offsetForCode('KeyG')).toBeNull();
    expect(offsetForCode('KeyH')).toBeNull();
  });

  it('maps the semicolon key to the top C as one physical key', () => {
    expect(offsetForCode('Semicolon')).toBe(12);
    expect(PIANO_BINDINGS.filter((b) => b.code === 'Semicolon')).toHaveLength(1);
  });

  it('reads the physical key, falling back to the typed character', () => {
    expect(physicalKey({ code: 'Semicolon', key: ':' })).toBe('Semicolon');
    expect(physicalKey({ code: 'KeyA', key: 'A' })).toBe('KeyA');
    expect(physicalKey({ code: '', key: ';' })).toBe('Semicolon');
    expect(physicalKey({ code: '', key: ':' })).toBe('Semicolon');
    expect(physicalKey({ code: '', key: 'j' })).toBe('KeyJ');
    expect(physicalKey({ code: '', key: 'Enter' })).toBe('');
  });

  it('binds every semitone of the octave to a distinct key', () => {
    expect(PIANO_BINDINGS).toHaveLength(13);
    expect(new Set(PIANO_BINDINGS.map((b) => b.code)).size).toBe(13);
    PIANO_BINDINGS.forEach((b, offset) => expect(offsetForCode(b.code)).toBe(offset));
  });

  it('moves hints with the octave so hint, note and pitch agree', () => {
    expect(hintFor(60, 60)).toBe('A');
    expect(hintFor(72, 60)).toBe(';');
    expect(hintFor(73, 60)).toBeNull();
    expect(hintFor(59, 60)).toBeNull();
    expect(hintFor(72, 72)).toBe('A');
    expect(noteName(72 + offsetForCode('KeyJ')!)).toBe('G5');
  });

  it('calls out middle C', () => {
    expect(MIDDLE_C).toBe(60);
    expect(noteDisplayName(60)).toBe('Middle C (C4)');
    expect(noteDisplayName(72)).toBe('C5');
    expect(noteDisplayName(61)).toBe('C#4');
  });
});
