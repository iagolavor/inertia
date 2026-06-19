import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const apiProxy = {
  target: 'http://127.0.0.1:4783',
  changeOrigin: true,
  rewrite: (path: string) => path.replace(/^\/api/, ''),
  timeout: 120_000,
  proxyTimeout: 120_000
};

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': apiProxy
    }
  },
  preview: {
    proxy: {
      '/api': apiProxy
    }
  }
});
