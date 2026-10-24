import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit({ adapter: adapter(), csp:{mode:'auto',directives:{'default-src':["self"],'script-src':["self"],'style-src':["self","unsafe-inline"],'img-src':["self","data:"],'connect-src':["self"],'frame-ancestors':["none"],'base-uri':["self"],'object-src':["none"]}} })],
  server: { host: '127.0.0.1' }
});
