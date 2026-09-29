<script lang="ts">
  let { analyser, color = '#888888' }: { analyser: AnalyserNode; color?: string } = $props();

  let canvas: HTMLCanvasElement;

  $effect(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const data = new Float32Array(analyser.fftSize);
    const stroke = color;
    let frame = 0;

    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const width = Math.round(canvas.clientWidth * dpr);
      const height = Math.round(canvas.clientHeight * dpr);
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      analyser.getFloatTimeDomainData(data);
      ctx.clearRect(0, 0, width, height);
      ctx.lineWidth = 2 * dpr;
      ctx.strokeStyle = stroke;
      ctx.beginPath();
      for (let i = 0; i < data.length; i++) {
        const x = (i / (data.length - 1)) * width;
        const y = (0.5 - data[i] * 0.5) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      frame = requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(frame);
  });
</script>

<canvas bind:this={canvas} class="waveform" aria-hidden="true"></canvas>

<style>
  .waveform {
    display: block;
    width: 100%;
    height: 120px;
    border-radius: 12px;
    background: var(--surface-2);
  }
</style>
