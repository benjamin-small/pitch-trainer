<script lang="ts">
  import type { AnswerOption } from '../lib/answers';

  let {
    options,
    disabled,
    chosen,
    correct,
    onChoose,
  }: {
    options: AnswerOption[];
    disabled: boolean;
    chosen: number | null;
    correct: number | null;
    onChoose: (value: number) => void;
  } = $props();
</script>

<div class="answers">
  {#each options as option (option.value)}
    <button
      class="btn answer"
      class:right={correct === option.value}
      class:wrong={chosen === option.value && correct !== null && correct !== option.value}
      {disabled}
      onclick={() => onChoose(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .answers {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.6rem;
  }
  .answer {
    min-width: 4.5rem;
    font-size: 1.1rem;
    font-weight: 600;
  }
  .answer.right {
    border-color: var(--good);
    box-shadow: inset 0 0 0 2px var(--good);
    opacity: 1;
  }
  .answer.wrong {
    border-color: var(--bad);
    box-shadow: inset 0 0 0 2px var(--bad);
    opacity: 1;
  }
</style>
