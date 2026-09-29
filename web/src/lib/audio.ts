/** Owns the AudioContext: plays engine-rendered buffers and exposes an analyser for the scope. */
export class AudioOut {
  readonly context: AudioContext;
  readonly analyser: AnalyserNode;
  private current: { source: AudioBufferSourceNode; timers: number[] } | null = null;

  constructor() {
    this.context = new AudioContext();
    this.analyser = this.context.createAnalyser();
    this.analyser.fftSize = 2048;
    this.analyser.connect(this.context.destination);
  }

  get sampleRate(): number {
    return this.context.sampleRate;
  }

  /** Must be called from a user gesture before the first sound. */
  async resume(): Promise<void> {
    if (this.context.state !== 'running') await this.context.resume();
  }

  play(samples: Float32Array, cues: number[] = [], onCue?: (index: number) => void): Promise<void> {
    if (this.context.state !== 'running') void this.context.resume();
    this.stop();
    const buffer = this.context.createBuffer(1, samples.length, this.context.sampleRate);
    buffer.getChannelData(0).set(samples);
    const source = this.context.createBufferSource();
    source.buffer = buffer;
    source.connect(this.analyser);
    const timers = cues.map((seconds, i) => window.setTimeout(() => onCue?.(i), seconds * 1000));
    const playing = { source, timers };
    this.current = playing;
    return new Promise((resolve) => {
      source.onended = () => {
        timers.forEach(clearTimeout);
        if (this.current === playing) this.current = null;
        resolve();
      };
      source.start();
    });
  }

  stop(): void {
    const playing = this.current;
    if (!playing) return;
    this.current = null;
    playing.timers.forEach(clearTimeout);
    playing.source.stop();
  }
}
