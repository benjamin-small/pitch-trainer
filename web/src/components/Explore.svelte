<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { AudioOut } from '../lib/audio';
  import type { Trainer } from '../lib/trainer';
  import { HIGHEST, LOWEST, isSharp, noteColor, noteFrequency, noteName } from '../lib/noteVisuals';
  import { MIDDLE_C, hintFor, noteDisplayName, offsetForCode, physicalKey } from '../lib/pianoKeys';
  import NoteGlyph from './NoteGlyph.svelte';
  import Waveform from './Waveform.svelte';

  let { trainer, audio }: { trainer: Trainer; audio: AudioOut } = $props();

  const allKeys = Array.from({ length: HIGHEST - LOWEST + 1 }, (_, i) => LOWEST + i);
  const whiteKeys = allKeys.filter((midi) => !isSharp(midi));
  const blackKeys = allKeys
    .filter(isSharp)
    .map((midi) => ({ midi, whitesBefore: whiteKeys.filter((w) => w < midi).length }));

  let octaveBase = $state(60);
  let current = $state<number | null>(null);

  onDestroy(() => audio.stop());

  function playNote(midi: number) {
    current = midi;
    void audio.play(trainer.noteAudio(midi));
  }

  function playScale() {
    const top = Math.min(octaveBase + 12, HIGHEST);
    const notes = trainer.scaleNotes(octaveBase, top);
    void audio.play(
      trainer.scaleAudio(octaveBase, top),
      notes.map((n) => n.onset),
      (i) => (current = notes[i].midi),
    );
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
    const code = physicalKey(event);
    if (code === 'KeyZ') octaveBase = Math.max(LOWEST, octaveBase - 12);
    else if (code === 'KeyX') octaveBase = Math.min(HIGHEST - 12, octaveBase + 12);
    else {
      const offset = offsetForCode(code);
      if (offset !== null) playNote(octaveBase + offset);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="explore">
  <section class="display">
    <div class="now">
      {#if current !== null}
        <NoteGlyph midi={current} size={132} label={false} />
        <p class="played">{noteDisplayName(current)}</p>
        <p class="freq">{noteFrequency(current).toFixed(1)} Hz</p>
      {:else}
        <p class="hint">Tap a key, or play with A S D F J K L ; on your keyboard.</p>
      {/if}
    </div>
    <Waveform analyser={audio.analyser} color={current !== null ? noteColor(current) : '#888888'} />
  </section>

  <div class="controls">
    <button class="btn" onclick={playScale}>Play scale from {noteName(octaveBase)}</button>
    <span class="octave">
      <kbd>A S D F J K L ;</kbd> play {noteName(octaveBase)}–{noteName(octaveBase + 12)} ·
      <kbd>W E U I O</kbd> sharps · <kbd>Z</kbd> / <kbd>X</kbd> shift octave
    </span>
  </div>

  <div class="keyboard-scroll">
    <div class="keyboard" style:--whites={whiteKeys.length}>
      {#each whiteKeys as midi (midi)}
        <button
          class="key white"
          class:active={current === midi}
          class:middle={midi === MIDDLE_C}
          aria-label={noteDisplayName(midi)}
          onclick={() => playNote(midi)}
        >
          <NoteGlyph {midi} size={18} label={false} />
          <span class="note">{noteName(midi)}</span>
          {#if midi === MIDDLE_C}<span class="middle-tag">Middle</span>{/if}
          <kbd class="hint-key" class:empty={!hintFor(midi, octaveBase)}>{hintFor(midi, octaveBase) ?? ''}</kbd>
        </button>
      {/each}
      {#each blackKeys as { midi, whitesBefore } (midi)}
        <button
          class="key black"
          class:active={current === midi}
          style:--pos={whitesBefore}
          style:--tint={noteColor(midi)}
          aria-label={noteDisplayName(midi)}
          onclick={() => playNote(midi)}
        >
          <span class="note">{noteName(midi)}</span>
          <kbd class="hint-key" class:empty={!hintFor(midi, octaveBase)}>{hintFor(midi, octaveBase) ?? ''}</kbd>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .explore {
    display: grid;
    gap: 1.25rem;
  }
  .display {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .now {
    min-height: 190px;
    display: grid;
    place-items: center;
    align-content: center;
    gap: 0.25rem;
  }
  .freq,
  .hint,
  .octave {
    margin: 0;
    color: var(--muted);
  }
  .freq {
    font-variant-numeric: tabular-nums;
  }
  .played {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem 1rem;
  }
  .keyboard-scroll {
    overflow-x: auto;
    padding-bottom: 0.25rem;
  }
  .keyboard {
    --white-w: calc(100% / var(--whites));
    position: relative;
    display: flex;
    min-width: 600px;
    height: 170px;
  }
  .key {
    border: 1px solid #0003;
    padding: 0;
  }
  .key.white {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    padding-bottom: 8px;
    background: #fbfaf7;
    color: #444;
    border-radius: 0 0 6px 6px;
  }
  .key.black {
    position: absolute;
    top: 0;
    z-index: 1;
    width: calc(var(--white-w) * 0.62);
    left: calc(var(--white-w) * var(--pos) - var(--white-w) * 0.31);
    height: 60%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
    padding-bottom: 6px;
    background: color-mix(in srgb, var(--tint) 55%, #111);
    color: #fff;
    border-radius: 0 0 5px 5px;
  }
  .key.active {
    box-shadow: inset 0 -6px 0 var(--accent);
  }
  .note {
    font-size: 0.75rem;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .key.black .note {
    font-size: 0.6rem;
  }
  .middle-tag {
    font-size: 0.55rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    color: var(--accent);
  }
  .key.middle {
    background: color-mix(in srgb, var(--accent) 10%, #fbfaf7);
  }
  /* Shortcut hints are small outlined keycaps, so they read as typing aids, not note names. */
  .hint-key {
    min-width: 1.1em;
    min-height: 1.1em;
    padding: 0 2px;
    border: 1px solid currentColor;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.55rem;
    font-weight: 400;
    line-height: 1.1em;
    text-align: center;
    opacity: 0.6;
  }
  .hint-key.empty {
    visibility: hidden;
  }
  .octave kbd {
    padding: 0 3px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.85em;
  }
</style>
