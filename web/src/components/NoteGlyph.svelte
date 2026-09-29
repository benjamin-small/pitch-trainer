<script lang="ts">
  import { noteVisual } from '../lib/noteVisuals';
  import { shapeGeometry } from '../lib/shapes';

  let { midi, size = 96, label = true }: { midi: number; size?: number; label?: boolean } = $props();

  const visual = $derived(noteVisual(midi));
  const geo = $derived(shapeGeometry(visual.shape));
</script>

<figure class="glyph" style:width="{size}px">
  <svg viewBox="-50 -50 100 100" width={size} height={size} role="img" aria-label={visual.label}>
    {#if geo.type === 'circle'}
      <circle r={geo.r} fill={visual.color} />
    {:else if geo.type === 'ring'}
      <circle r={geo.r} fill="none" stroke={visual.color} stroke-width={geo.width} />
    {:else if geo.type === 'polygon'}
      <polygon points={geo.points} fill={visual.color} />
    {:else}
      <path d={geo.d} fill={visual.color} />
    {/if}
  </svg>
  {#if label}
    <figcaption>{visual.label}</figcaption>
  {/if}
</figure>

<style>
  .glyph {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }
  svg {
    display: block;
  }
  figcaption {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
