import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm';
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [
    react(),
    // @ts-ignore
    wasm()
  ],
  resolve: {
    alias: {
      'netlang-core': fileURLToPath(new URL('../core/pkg', import.meta.url))
    }
  },
  server: {
    fs: {
      allow: ['..']
    }
  },
  build: {
    target: 'esnext'
  }
})
