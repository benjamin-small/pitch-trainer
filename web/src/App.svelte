<script lang="ts">
  import { Trainer } from './lib/trainer';

  let status = $state('Loading engine…');

  Trainer.create(48000, null)
    .then((trainer) => {
      const view = trainer.newRound('sequence');
      status = `Engine ready: "${view.prompt}" ${view.options} options, ${trainer.roundAudio().length} samples`;
    })
    .catch((error) => (status = `Engine failed: ${error}`));
</script>

<p>{status}</p>
