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
