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

  /** Bumped on every playback so a superseded playback's cleanup can't touch the newer one. */
  let playback = 0;

  async function playChallenge() {
    if (!view) return;
    const id = ++playback;
    phase = 'playing';
    try {
      await audio.play(trainer.roundAudio(), view.onsets, (i) => (cue = i));
    } finally {
      if (id === playback) {
        cue = null;
        phase = 'answering';
      }
    }
  }

  function nextRound() {
    (document.activeElement as HTMLElement | null)?.blur();
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
    const id = ++playback;
    phase = 'revealing';
    revealed = 0;
    try {
      await audio.play(
        trainer.roundAudio(),
        notes.map((n) => n.onset),
        (i) => {
          cue = i;
          revealed = i + 1;
        },
      );
    } finally {
      if (id === playback) {
        cue = null;
        revealed = notes.length;
        phase = 'revealed';
      }
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
    const onButton = (event.target as HTMLElement | null)?.closest('button');
    if (event.key === ' ') {
      // Space advances from the start screen and from the reveal, even mid-replay.
      if (phase === 'ready' || phase === 'revealing' || phase === 'revealed') {
        event.preventDefault();
        nextRound();
      } else if (!onButton) {
        event.preventDefault(); // avoid page scroll
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
      <p class="prompt" aria-live="polite">
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
        <button class="btn primary" onclick={nextRound}>Next</button>
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
