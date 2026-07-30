import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  // Tauri serves the built files from disk, so absolute asset URLs break.
  base: './',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Matches the WebKit/WebView2 versions Tauri v2 supports.
    target: ['es2022', 'safari16'],
    sourcemap: true,
  },
  server: {
    port: 5174,
    strictPort: true,
  },
  // Tauri surfaces Rust errors better than Vite's overlay does.
  clearScreen: false,
});
