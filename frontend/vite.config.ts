import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  server: {
    port: 3000,
    // The browser's Host is passed through (changeOrigin: false): the worker refuses a
    // POST whose Origin host is not the Host it was sent to (issue #133), and the page
    // may be opened as localhost, 127.0.0.1 or a LAN name.
    proxy: {
      '/v1': {
        target: 'http://localhost:8080',
        changeOrigin: false,
      },
      '/health': {
        target: 'http://localhost:8080',
        changeOrigin: false,
      },
    },
  },
  build: {
    target: 'es2022',
    sourcemap: true,
  },
});