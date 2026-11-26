import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

// The phone app's UI (Android build; see src-tauri/tauri.android.conf.json).
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  root: fileURLToPath(new URL('./mobile', import.meta.url)),
  plugins: [svelte()],
  clearScreen: false,
  build: { outDir: fileURLToPath(new URL('./dist-mobile', import.meta.url)), emptyOutDir: true },
  server: {
    host: host || false,
    port: 1422,
    strictPort: true,
    hmr: host ? { protocol: 'ws', host, port: 1423 } : undefined,
    // Browser preview only: the dev fixtures point at media in the desktop app's data folder.
    fs: { strict: false },
    watch: { ignored: ['**/src-tauri/**'] }
  }
});
