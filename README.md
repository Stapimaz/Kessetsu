# NetLang

NetLang, devreleri metinle tanımlayıp yazılım gibi derlemek, simüle etmek ve assertion'larla sınamak için geliştirilen agent-driven bir circuit engineering platformudur. CLI'ın human modu insanlara, versioned JSON modu AI ajanları ve otomasyona; zero-friction Web Hub ise tarayıcı kullanıcılarına hizmet eder. Bütün yüzeyler aynı Rust çekirdeğini kullanır; typed Circuit IR, deterministik graph/ERC, SPICE netlist, layout ve EDA çıktıları ortak semantikten üretilir.

```text
Electrical requirements → NetLang source → compile/ERC → simulate/measure/assert → structured feedback
```

Proje belirli bir eğitim senaryosu veya devre sınıfıyla sınırlı değildir. Hedef; insanların ve AI ajanlarının ölçülebilir gereksinimlerden başlayıp topoloji, değer ve model seçimlerini güvenilir structured feedback ile iteratif geliştirebilmesidir.

## Mevcut kapsam

- Rust parser, module flattening ve typed Circuit IR
- Sürümlü `netlang.compile.v3` compile raporu ve `netlang.schematic.v1` şema sözleşmesi
- Deterministik net isimlendirme ve `NL-P/C/E/I/S/F` diagnostic alanları
- SPICE üretimi, simulator discovery/provenance ve Windows Ngspice sidecar runtime
- Typed OP/transient/AC/DC simulation sonucu ve PASS/FAIL/ERROR/SKIPPED assertion runtime
- Compact `netlang.cli.v1` JSON, stdin agent döngüsü ve debug `--include` seçimi
- Typed user/package model-subcircuit çözümleme, provenance manifest'i ve `netlang.lock`
- Human/JSON CLI; güvenli output/overwrite ve exit-code sözleşmesi
- WASM tabanlı, gerçek browser simulation çalıştıran React Web Hub
- Canonical, bağlantısı doğrulanmış automatic schematic motoru
- SVG, PNG, PDF, Schematic JSON, SPICE, KiCad ve LTspice export
- Sürümlü, sıkıştırılmış ve package-aware paylaşım URL'leri

Güncel görev ve sınırlar için [roadmap](docs/ROADMAP.md), mimari kurallar için [architecture](docs/architecture.md), fiziksel kapsam için [supported domain](docs/supported_domain.md) ve formül/sign convention'lar için [engineering measurements](docs/engineering_measurements.md) esas alınır.

## Web Hub

NetLang Web Hub, terminal kullanmak istemeyen insanların devreleri CodePen benzeri sade bir çalışma alanında doğrudan tarayıcıdan geliştirebilmesi için tasarlanan ana ürün yüzeyidir. Ayrı bir Web-only motor kullanmaz; CLI ile aynı canonical Rust çekirdeğini WebAssembly üzerinden çalıştırır.

Mevcut repository build'inde Web Hub şunları yapabiliyor:

- Monaco ile NetLang kodunu düzenleme, örnek devre seçme
- WASM üzerinden canlı compile, ERC ve canonical schematic connectivity doğrulama
- Web Worker içinde gerçek OP/transient/AC/DC simulation
- İnteraktif waveform/Bode/DC grafikleri ve assertion sonuçları
- Canonical şemayı zoom/fit ile inceleme
- Yedi görsel, makine ve EDA formatını capability/loss bilgisiyle indirme
- Source ve exact package sürümlerini sıkıştırılmış URL ile paylaşma

İlk sürümde Web Hub içinde AI chat yoktur ve bu gizlenen bir eksik değildir: AI/otomasyon yüzeyi CLI'ın stdin + versioned JSON tool contract'ı, insan yüzeyi Web Hub'dır. İkisi de aynı Core'u kullanır. Provider-independent Web AI tasarım yüzeyi sonraki faz için bilinçli olarak ayrılmıştır. Public Web Hub bağlantısı release deployment tamamlandığında buraya eklenecektir.

## Kısa NetLang örneği

```netlang
net GND
net out

source V1 5V
resistor R1 1k

connect V1.plus, R1.p1 to out
connect V1.minus, R1.p2 to GND

simulate op
```

Aynı kaynak CLI'dan veya Web Hub'dan compile edildiğinde aynı IR, diagnostic ve SPICE sonucu üretilir.

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

Var olan output dosyaları varsayılan olarak ezilmez; bilinçli overwrite için `--force` gerekir. CLI komutları, JSON alanları ve exit kodları [CLI reference](docs/cli_reference.md) içinde tanımlıdır. İlk devre için [tutorial](docs/tutorial.md), kısa çözümler için [cookbook](docs/cookbook.md), sorunlar için [troubleshooting](docs/troubleshooting.md) ve ürün farkları için [Why NetLang?](docs/why_netlang.md) ile devam edin.

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
- `webapp/`: React/TypeScript zero-friction Web Hub
- `docs/`: roadmap, architecture, dil/simulation/export sözleşmeleri ve kullanım rehberleri
- `scripts/verify.ps1`: kök kalite kapısı

## Önemli sınırlar

- Backend'ler yalnız typed Circuit IR üzerinden çalışır.
- Generated output fiziksel doğrulama veya mühendis incelemesinin yerine geçmez.
- Embedded runtime'ın provenance/lisans notları [Ngspice runtime README](core/tools/ngspice/README.md) içinde tutulur.
