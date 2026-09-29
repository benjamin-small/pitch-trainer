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
