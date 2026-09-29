<script lang="ts">
  import { AudioOut } from './lib/audio';
  import { Trainer } from './lib/trainer';
  import { loadEngineState, saveEngineState } from './lib/storage';
  import { TEST_INFO } from './lib/answers';
  import type { TestKind } from './lib/types';
  import Explore from './components/Explore.svelte';
  import TestRunner from './components/TestRunner.svelte';

  type Mode = 'explore' | TestKind;
  const MODES: { id: Mode; label: string }[] = [
    { id: 'explore', label: 'Explore' },
    { id: 'upDown', label: TEST_INFO.upDown.title },
    { id: 'pickTwo', label: TEST_INFO.pickTwo.title },
    { id: 'sequence', label: TEST_INFO.sequence.title },
  ];

  let status = $state<'gate' | 'loading' | 'ready' | 'error'>('gate');
  let audio = $state.raw<AudioOut | null>(null);
  let trainer = $state.raw<Trainer | null>(null);
  let mode = $state<Mode>('explore');

  async function start() {
    status = 'loading';
    try {
      const out = new AudioOut();
      await out.resume();
      trainer = await Trainer.create(out.sampleRate, loadEngineState());
      audio = out;
      status = 'ready';
    } catch (error) {
      console.error(error);
      status = 'error';
    }
  }

  function persist() {
    if (trainer) saveEngineState(trainer.stateJson());
  }
</script>

<div class="app">
  <header class="top">
    <h1>Pitch Trainer</h1>
    {#if status === 'ready'}
      <nav aria-label="Mode">
        {#each MODES as m (m.id)}
          <button class="tab" class:current={mode === m.id} aria-current={mode === m.id} onclick={() => (mode = m.id)}>
            {m.label}
          </button>
        {/each}
      </nav>
    {/if}
  </header>

  <main>
    {#if status === 'ready' && trainer && audio}
      {#if mode === 'explore'}
        <Explore {trainer} {audio} />
      {:else}
        {#key mode}
          <TestRunner {trainer} {audio} kind={mode} onAnswered={persist} />
        {/key}
      {/if}
    {:else if status === 'error'}
      <p class="card">Couldn't load the audio engine. Try a current version of Chrome, Firefox, or Safari.</p>
    {:else}
      <section class="card gate">
        <p>
          Train your ear with three tests: <strong>Up / Down</strong>, <strong>Pick Two</strong>, and
          <strong>Sequence</strong>. Each one gets harder as you improve. Use <strong>Explore</strong> to learn
          each note's color and shape.
        </p>
        <button class="btn primary" disabled={status === 'loading'} onclick={start}>
          {status === 'loading' ? 'Starting…' : 'Tap to start'}
        </button>
      </section>
    {/if}
  </main>
</div>

<style>
  .app {
    max-width: 920px;
    margin: 0 auto;
    padding: 16px;
    display: grid;
    gap: 1.25rem;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  h1 {
    margin: 0;
    font-size: 1.4rem;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding: 0.25rem;
    border-radius: 12px;
    background: var(--surface-2);
  }
  .tab {
    border: 0;
    background: transparent;
    padding: 0.45rem 0.85rem;
    border-radius: 9px;
  }
  .tab.current {
    background: var(--surface);
    font-weight: 600;
    box-shadow: 0 1px 2px #0002;
  }
  .card {
    margin: 0;
    padding: 1.5rem;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .gate {
    display: grid;
    gap: 1rem;
    justify-items: start;
  }
  .gate p {
    margin: 0;
  }
</style>
