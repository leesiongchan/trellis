import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
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
    })
  ],
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
