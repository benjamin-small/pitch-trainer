<script lang="ts">
  import { HIGHEST, LOWEST, isSharp, noteColor, noteName } from '../lib/noteVisuals';
  import { MIDDLE_C, hintFor, noteDisplayName } from '../lib/pianoKeys';
  import NoteGlyph from './NoteGlyph.svelte';

  let {
    active = null,
    marked = [],
    octaveBase = null,
    compact = false,
    onPlay,
  }: {
    /** Note to highlight as sounding now. */
    active?: number | null;
    /** Notes to mark softly, such as every note in a round. */
    marked?: number[];
    /** Octave the typing shortcuts start from; null hides the shortcut hints. */
    octaveBase?: number | null;
    /** Shorter keys without glyphs, for tight layouts. */
    compact?: boolean;
    /** Called when a key is clicked; without it the keyboard is display-only. */
    onPlay?: (midi: number) => void;
  } = $props();

  const allKeys = Array.from({ length: HIGHEST - LOWEST + 1 }, (_, i) => LOWEST + i);
  const whiteKeys = allKeys.filter((midi) => !isSharp(midi));
  const blackKeys = allKeys
    .filter(isSharp)
    .map((midi) => ({ midi, whitesBefore: whiteKeys.filter((w) => w < midi).length }));
</script>

<div class="keyboard-scroll">
  <div class="keyboard" class:compact style:--whites={whiteKeys.length}>
    {#each whiteKeys as midi (midi)}
      <button
        class="key white"
        class:active={active === midi}
        class:marked={marked.includes(midi)}
        class:middle={midi === MIDDLE_C}
        aria-label={noteDisplayName(midi)}
        disabled={!onPlay}
        onclick={() => onPlay?.(midi)}
      >
        {#if !compact}<NoteGlyph {midi} size={18} label={false} />{/if}
        <span class="note">{noteName(midi)}</span>
        {#if midi === MIDDLE_C}<span class="middle-tag">Middle</span>{/if}
        {#if octaveBase !== null}
          <kbd class="hint-key" class:empty={!hintFor(midi, octaveBase)}>{hintFor(midi, octaveBase) ?? ''}</kbd>
        {/if}
      </button>
    {/each}
    {#each blackKeys as { midi, whitesBefore } (midi)}
      <button
        class="key black"
        class:active={active === midi}
        class:marked={marked.includes(midi)}
        style:--pos={whitesBefore}
        style:--tint={noteColor(midi)}
        aria-label={noteDisplayName(midi)}
        disabled={!onPlay}
        onclick={() => onPlay?.(midi)}
      >
        <span class="note">{noteName(midi)}</span>
        {#if octaveBase !== null}
          <kbd class="hint-key" class:empty={!hintFor(midi, octaveBase)}>{hintFor(midi, octaveBase) ?? ''}</kbd>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
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
  .keyboard.compact {
    height: 100px;
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
  .key:disabled {
    cursor: default;
    opacity: 1;
  }
  .key.marked {
    box-shadow: inset 0 -3px 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .key.active {
    box-shadow: inset 0 -6px 0 var(--accent);
  }
  .key.white.active {
    background: color-mix(in srgb, var(--accent) 30%, #fbfaf7);
  }
  .key.black.active {
    background: color-mix(in srgb, var(--tint) 85%, #fff);
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
</style>
