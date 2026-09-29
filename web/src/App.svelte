<script lang="ts">
  import { AudioOut } from './lib/audio';
  import { Trainer } from './lib/trainer';
  import Explore from './components/Explore.svelte';

  let audio = $state.raw<AudioOut | null>(null);
  let trainer = $state.raw<Trainer | null>(null);

  async function start() {
    const out = new AudioOut();
    await out.resume();
    trainer = await Trainer.create(out.sampleRate, null);
    audio = out;
  }
</script>

<main style="max-width: 900px; margin: 0 auto; padding: 16px;">
  {#if trainer && audio}
    <Explore {trainer} {audio} />
  {:else}
    <button class="btn primary" onclick={start}>Tap to start</button>
  {/if}
</main>
