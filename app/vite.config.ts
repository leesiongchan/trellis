import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, lazyPlugins } from 'vite-plus';
import pandacss from '@pandacss/vite';

export default defineConfig({
  plugins: lazyPlugins(() =>
    process.env.VITEST
      ? [svelte(), pandacss({ transform: true })]
      : [
          sveltekit({
            adapter: adapter(),
            csp: {
              mode: 'auto',
              directives: {
                'default-src': ['self'],
                'script-src': ['self'],
                'style-src': ['self', 'unsafe-inline'],
                'img-src': ['self', 'data:'],
                'connect-src': ['self'],
                'frame-ancestors': ['none'],
                'base-uri': ['self'],
                'object-src': ['none']
              }
            }
          }),
          pandacss({ transform: true })
        ]
  ),
  resolve: {
    alias: {
      '#lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
      '#styled': fileURLToPath(new URL('./styled-system', import.meta.url))
    }
  },
  test: { include: ['tests/unit/**/*.test.ts', 'tests/integration/**/*.test.ts'] },
  server: {
    host: '127.0.0.1',
    proxy: Object.fromEntries(
      ['/access', '/api', '/health', '/metrics'].map((path) => [
        path,
        { target: 'http://127.0.0.1:3001', changeOrigin: false }
      ])
    )
  }
});
