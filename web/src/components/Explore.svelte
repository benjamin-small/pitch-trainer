<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { AudioOut } from '../lib/audio';
  import type { Trainer } from '../lib/trainer';
  import { HIGHEST, LOWEST, noteColor, noteFrequency, noteName } from '../lib/noteVisuals';
  import { noteDisplayName, offsetForCode, physicalKey } from '../lib/pianoKeys';
  import NoteGlyph from './NoteGlyph.svelte';
  import Piano from './Piano.svelte';
  import Waveform from './Waveform.svelte';

  let { trainer, audio }: { trainer: Trainer; audio: AudioOut } = $props();

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

  <Piano active={current} {octaveBase} onPlay={playNote} />
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
  .octave kbd {
    padding: 0 3px;
    border: 1px solid var(--border);
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.85em;
  }
</style>
