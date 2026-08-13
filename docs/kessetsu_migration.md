# Kessetsu Kimlik Migrasyonu

Bu belge, yayınlanmamış projenin geçici kimliğinden Kessetsu'ya tek seferlik ve temiz biçimde taşınması için uygulanacak sözleşmedir. Migrasyon tamamlanana kadar Faz 4'ün diğer işleri bekler; yarım bir marka geçişi release edilemez.

## Kesinleşen kimlik

| Yüzey | Yeni kimlik |
|---|---|
| Ürün ve repository adı | `Kessetsu` |
| CLI executable ve komut | `kess` |
| Devre kaynak uzantısı | `.kess` |
| Rust package | `kessetsu-core` |
| Rust crate/import | `kessetsu_core` |
| Rust CLI source | `core/src/bin/kess.rs` |
| Parser grammar | `core/src/kessetsu.pest` |
| Versioned schema namespace | `kessetsu.*` |
| Diagnostic prefix | `KES-*` |
| Builtin model prefix | `KESSETSU_*` |
| Environment prefix | `KESSETSU_*` |
| Model lockfile | `kessetsu.lock` |
| Website/domain | `kessetsu.com` |

## Migrasyon politikası

- Proje henüz public olmadığı için eski ürün adı, kaynak uzantısı, schema kimlikleri, diagnostic kodları veya environment değişkenleri için compatibility alias bırakılmaz.
- Değişiklik basit bir görünen-metin değişimi değildir. Dosya yolları, paket/binary adları, serialization sözleşmeleri, fixtures, goldens, workflow'lar, release artifact'leri ve dokümantasyon birlikte taşınır.
- Circuit IR, deterministic node naming, ERC ve exporter mimarisi değişmez; yalnız proje kimliği ve ona bağlı public contract adları taşınır.
- Git geçmişi yeniden yazılmaz. Eski kimliğin geçmiş commitlerde bulunması aktif ürün yüzeyinde kalıntı sayılmaz.
- Repository adı ve yerel kök klasör adı, içerik migrasyonu doğrulandıktan sonra ayrı dış adım olarak değiştirilir.

## Uygulama ve kabul listesi

- [x] Tracked kaynak/fixture dosyalarını `.kess` olarak taşı ve bütün referansları güncelle.
- [x] CLI binary, Rust package/crate ve parser grammar kimliklerini taşı.
- [x] Schema, diagnostic, builtin model, lockfile ve environment kimliklerini taşı.
- [x] Web Hub/WASM isimleri, kullanıcı metinleri, paylaşım ve export sözleşmelerini taşı.
- [x] README, mimari, referanslar, lisans/notice metinleri ve diğer belgeleri taşı.
- [x] GitHub Actions, Pages, release artifact'leri, scriptler ve repository URL'lerini taşı.
- [x] Generated/golden dosyaları canonical üreticilerle yeniden oluştur.
- [x] Case-insensitive audit'te aktif tracked tree içinde eski ürün adı, eski crate adı, eski environment/model prefix'i veya eski kaynak uzantısı kalmadığını doğrula.
- [x] `kess --help`, representative `.kess` compile/simulate/render/export ve JSON schema/diagnostic yollarını doğrula.
- [x] Canonical `scripts/verify.ps1` kalite kapısını geçir.
- [x] Roadmap ve bu belgeyi doğrulama kanıtlarıyla kapat.
- [ ] GitHub repository adını `Kessetsu`, remote URL'yi ve yerel klasör adını güncelle.

## Bilinçli olarak bu migrasyonun dışında kalanlar

- Domain DNS, production Web Hub deployment ve public yayın açılışı.
- Marka tescili veya hukuki uygunluk görüşü.
- Git geçmişindeki eski commit içeriklerinin silinmesi.
- Devre dili semantiğinde, simulation davranışında veya schematic layout algoritmasında özellik değişikliği.

## Doğrulama kanıtı

2026-08-14 doğrulaması:

- Case-insensitive full-tree audit, Git geçmişi ve üçüncü taraf dependency klasörü dışında eski ürün adı, eski crate/model/environment/diagnostic önekleri, eski executable kimliği veya eski kaynak uzantısı bulmadı.
- `kess --help` doğru binary/ürün kimliğini; RC hedefli smoke `kessetsu.cli.v1`, `kessetsu.compile.v3`, `kessetsu.simulation.v1` sözleşmelerini ve 5/5 assertion sonucunu doğruladı. SVG ve KiCad artifact'leri `.kess` kaynağından üretildi.
- Şematik corpus canonical Kessetsu üreticisiyle yeniden oluşturuldu; schema değişiminin etkilediği altı deterministic SVG golden hash'i yeni byte çıktılarıyla güncellendi ve connectivity/quality kapıları geçti.
- Temiz cache sonrası canonical `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1` tamamen PASS: Rust fmt/Clippy/test/release, WASM, Web lint + 10 unit + production build, runtime/deployment audit, 11 Chromium E2E, dependency/license/security audit, replayable agent eval, paketlenmiş Windows `kess.exe` üzerinde gerçek Ngspice 12/12 smoke ve RC/gain/power KiCad+LTspice smoke.
- Eski isimli ignored release/WASM/Web-test artifact'leri silindi; Rust build cache'i temizlenip yalnız yeni kimlikle baştan üretildi.
- Kalan dış adım: doğrulanmış commit pushlandıktan sonra GitHub repository ve yerel kök klasör adını değiştirmek.
