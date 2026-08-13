# Kessetsu Web App

Tarayıcı arayüzü React, TypeScript, Vite ve `kessetsu-core` WASM paketiyle çalışır. Derleme mantığı Web içinde tekrar edilmez; `compile_kessetsu` üzerinden canonical `kessetsu.compile.v1` raporu tüketilir.

## Yerel geliştirme

Repository kökünden tam doğrulama:

```powershell
./scripts/verify.ps1
```

Yalnız Web geliştirme akışı:

```bash
cd webapp
npm ci
npm run build
npm run dev
```

`npm run build`, önce Rust çekirdeğini `core/pkg` altına WASM olarak üretir, ardından TypeScript ve Vite production build çalıştırır. Editördeki default devre repository kökündeki `examples/demo_circuit.kess` dosyasıdır; ayrı bir Web-only dil örneği tutulmaz.
