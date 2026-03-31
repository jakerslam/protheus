// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative); this file is build wrapper config only.

import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
  plugins: [sveltekit()],
});
