import path from 'node:path';
import { defineConfig } from 'vite';

const ROOT = __dirname;

export default defineConfig(async () => {
  const { svelte } = await import('@sveltejs/vite-plugin-svelte');
  return {
    root: ROOT,
    base: '/svelte/',
    plugins: [svelte()],
    build: {
      outDir: path.resolve(ROOT, 'dist'),
      emptyOutDir: true,
      sourcemap: false,
      target: 'es2020',
    },
  };
});
