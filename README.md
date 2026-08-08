# NetLang

NetLang, devreleri metinle tanımlayıp yazılım gibi derlemek, simüle etmek ve assertion'larla sınamak için geliştirilen agent-driven bir circuit engineering platformudur. Aynı Rust çekirdeği CLI ve WebAssembly üzerinden çalışır; typed Circuit IR, deterministik graph/ERC, SPICE netlist, deneysel layout ve KiCad çıktıları üretir.

```text
Electrical requirements → NetLang source → compile/ERC → simulate/measure/assert → structured feedback
```

Proje belirli bir eğitim senaryosu veya devre sınıfıyla sınırlı değildir. Hedef; insanların ve AI ajanlarının ölçülebilir gereksinimlerden başlayıp topoloji, değer ve model seçimlerini güvenilir structured feedback ile iteratif geliştirebilmesidir.

## Mevcut kapsam

- Rust parser, module flattening ve typed Circuit IR
- Sürümlü `netlang.compile.v1` compile raporu
- Deterministik net isimlendirme ve `NL-P/C/E/I/S/F` diagnostic alanları
- SPICE üretimi ve Windows ngspice-46 sidecar runtime
- Human/JSON CLI; güvenli output/overwrite ve exit-code sözleşmesi
- WASM tabanlı React playground
- Deneysel automatic layout ve KiCad schematic export

Simülasyon sonuç modeli, assertion runtime ve profesyonel şema kalitesi halen Faz 3 ve sonraki fazların kapsamındadır. Güncel görev ve sınırlar için [roadmap](docs/ROADMAP.md), mimari kurallar için [architecture](docs/architecture.md) esas alınır.

## Hızlı başlangıç

Gereksinimler:

- Rust `1.97.1` ve `wasm32-unknown-unknown` target
- `wasm-pack 0.13.1`
- Node sürümü [`.nvmrc`](.nvmrc) ile eşleşen npm kurulumu
- Windows yerel simülasyonu için repository'deki sidecar; alternatif simulator yolu için `NETLANG_NGSPICE`

CLI'yi derleyip örnek bir devreyi kontrol etmek:

```powershell
cd core
cargo build --release
cargo run --release -- check ../examples/demo_circuit.nl
cargo run --release -- compile ../examples/demo_circuit.nl --output ../examples/demo_circuit.spice
```

Makine-okunabilir çıktı:

```powershell
cargo run --release -- check ../examples/demo_circuit.nl --format json
```

Var olan output dosyaları varsayılan olarak ezilmez; bilinçli overwrite için `--force` gerekir. CLI komutları, JSON alanları ve exit kodları [CLI reference](docs/cli_reference.md) içinde tanımlıdır.

## Build ve doğrulama

Repository'nin canonical kalite kapısı:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1
```

Bu komut dependency kurulumundan sonra Rust fmt, Clippy, test, release build, WASM package, production npm audit, Web lint ve production build adımlarını çalıştırır. Dependency'ler zaten kuruluysa:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1 -SkipNpmInstall
```

Ayrı build yüzeyleri:

```powershell
cd core
cargo test --all-targets
cargo build --release
wasm-pack build . --target web --out-dir pkg --release

cd ../webapp
npm.cmd ci
npm.cmd run build
```

`npm run build`, WASM paketini yeniden üretip Web production build'ini tamamlar.

## Repository yapısı

- `core/`: Rust library, CLI, WASM adaptörü, test corpus'u ve Windows Ngspice runtime
- `examples/`: canonical `.nl` örnekleri
- `webapp/`: React/TypeScript playground
- `docs/`: roadmap, architecture ve CLI sözleşmesi
- `scripts/verify.ps1`: kök kalite kapısı

## Önemli sınırlar

- Backend'ler yalnız typed Circuit IR üzerinden çalışır.
- Generated output fiziksel doğrulama veya mühendis incelemesinin yerine geçmez.
- Embedded runtime'ın provenance/lisans notları [Ngspice runtime README](core/tools/ngspice/README.md) içinde tutulur.
