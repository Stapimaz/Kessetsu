# Kessetsu Web App

The browser interface uses React, TypeScript, Vite, and the `kessetsu-core` WASM package. Compilation logic is not duplicated in the Web layer: current source consumes the canonical `kessetsu.compile.v6` report through `compile_kessetsu`; v4/v5 source shares are intentionally recompiled through the current Core. New source features remain unreleased until a tagged deployment.

The site root is a lightweight product landing page. `#editor` opens the full Web Hub, while versioned `#kessetsu=...` share fragments bypass the landing page and open the shared circuit directly. The workspace is loaded as a separate bundle so landing-page design changes do not couple to Monaco, WASM, simulation, or export behavior.

## Local development

Full verification from the repository root:

```powershell
./scripts/verify.ps1
```

Web-only development loop:

```bash
cd webapp
npm ci
npm run build
npm run dev
```

`npm run build` first compiles the Rust Core to WASM under `core/pkg`, then runs the TypeScript and Vite production build. The editor's default circuit comes from the canonical `core/tests/fixtures/benchmarks/rc_filter.kess` fixture; no separate Web-only language example is maintained.
