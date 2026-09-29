import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  // Relative asset URLs so the build works from any subpath (e.g. GitHub Pages /pitch-trainer/).
  base: './',
  plugins: [svelte()],
  test: { include: ['src/**/*.test.ts'] },
});
