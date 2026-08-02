import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import * as wasm from 'vite-plugin-wasm';
import * as topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [
    react(),
    wasm.default(),
    topLevelAwait.default()
  ],
  server: {
    fs: {
      allow: ['..']
    }
  }
})
