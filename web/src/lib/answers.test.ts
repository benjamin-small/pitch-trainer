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
