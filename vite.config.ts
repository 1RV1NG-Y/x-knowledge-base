import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// The desktop UI. It shares the phone app's look and some of its components (mobile/src).
const host = process.env.TAURI_DEV_HOST;
const fixtures = fileURLToPath(new URL('./mobile/.dev/fixtures.json', import.meta.url));

// Browser preview only: outside Tauri the UI answers from the phone preview's fixtures
// (scripts/mobile-fixtures.mjs), fetched from this URL.
const devFixtures: Plugin = {
  name: 'kb-dev-fixtures',
  apply: 'serve',
  configureServer(server) {
    server.middlewares.use('/.dev/fixtures.json', (_req, res) => {
      try {
        res.setHeader('content-type', 'application/json');
        res.end(readFileSync(fixtures));
      } catch {
        res.statusCode = 404;
        res.end();
      }
    });
  }
};

export default defineConfig({
  plugins: [svelte(), devFixtures],
  clearScreen: false,
  server: {
    host: host || false,
    port: 1424,
    strictPort: true,
    hmr: host ? { protocol: 'ws', host, port: 1425 } : undefined,
    // Browser preview only: the fixtures point at media in the app's data folder.
    fs: { strict: false },
    watch: { ignored: ['**/src-tauri/**'] }
  }
});
