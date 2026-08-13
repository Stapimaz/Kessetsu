import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm';
import { fileURLToPath, URL } from 'node:url'
import { copyFileSync, mkdirSync } from 'node:fs'
import { resolve } from 'node:path'

export default defineConfig({
  base: process.env.NETLANG_BASE_PATH ?? '/',
  plugins: [
    react(),
    // @ts-ignore
    wasm(),
    {
      name: 'netlang-runtime-license-bundle',
      closeBundle() {
        const licenseDir = resolve(import.meta.dirname, 'dist/licenses')
        mkdirSync(licenseDir, { recursive: true })
        copyFileSync(
          resolve(import.meta.dirname, 'node_modules/eecircuit-engine/LICENSE'),
          resolve(licenseDir, 'eecircuit-engine-MIT.txt'),
        )
        copyFileSync(
          resolve(import.meta.dirname, '../core/tools/ngspice/docs/COPYING'),
          resolve(licenseDir, 'ngspice-COPYING.txt'),
        )
        copyFileSync(
          resolve(import.meta.dirname, '../core/assets/fonts/RobotoMono-OFL.txt'),
          resolve(licenseDir, 'RobotoMono-OFL.txt'),
        )
      },
    }
  ],
  resolve: {
    alias: {
      'netlang-core': fileURLToPath(new URL('../core/pkg', import.meta.url))
    }
  },
  optimizeDeps: {
    // The simulator is loaded from a module Worker. Pre-bundling it prevents
    // Vite's dev server from discovering the 20 MB engine after the Worker
    // has already started and forcing a silent dependency reload.
    include: ['eecircuit-engine']
  },
  worker: {
    // eecircuit-engine contains ESM/import.meta based Emscripten bootstrap
    // code. A classic Worker bundle stalls before our message handler runs.
    format: 'es'
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
