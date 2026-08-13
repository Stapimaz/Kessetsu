# NetLang Geliştirme Yol Haritası

> Bu doküman NetLang'in geliştirme durumu, aktif milestone'u, kabul kriterleri ve görev sırası için **tek gerçek kaynaktır (Single Source of Truth)**.
>
> Mimari kurallar için `docs/architecture.md`, kullanıcıya açık CLI sözleşmesi için `docs/cli_reference.md` kullanılır. Bu belgeler arasında çelişki varsa geliştirme durumu açısından bu roadmap esas alınır ve çelişki aktif milestone içinde düzeltilir.
>
> Son kapsamlı repo denetimi: **2026-08-13**
>
> Aktif milestone: **Faz 4 — Profesyonel Şema, Web Hub ve Yayın**
>
> Önceki milestone: **Faz 3 — Simülasyon ve Assertion Runtime (tamamlandı)**

---

## 1. Proje Kimliği

**NetLang**, elektriksel gereksinimlerden başlayarak devrelerin insanlar veya AI ajanları tarafından iteratif biçimde geliştirilmesini mümkün kılan; devreleri metinle tanımlayan, typed bir Circuit IR üzerinden deterministik SPICE netlist üreten, simülasyon ve ölçüm çalıştıran, yapılandırılmış doğrulama sonuçları veren ve profesyonel şema/EDA çıktıları oluşturmayı hedefleyen bir **agent-driven circuit engineering platformudur**.

```text
Compile, simulate and test circuits like software.
```

### Ürün kuzey yıldızı

NetLang'in hedefi belirli bir eğitim senaryosu, kullanıcı seviyesi veya tek bir devre sınıfıyla sınırlı değildir. Bir AI ajanı ya da insan, tasarım gereksinimlerini NetLang'in ölçülebilir constraint/assertion modeline dönüştürebilmeli; devre topolojisini ve component değerlerini iteratif olarak geliştirip her adımda güvenilir structured feedback alabilmelidir.

Hedef döngü:

```text
Elektriksel gereksinimler
    → devre/topoloji adayı
    → compile + semantic validation + ERC
    → simulate + measure + assert
    → structured engineering feedback
    → topoloji/değer/model revizyonu
    → gereksinimleri karşılayan doğrulanmış tasarım
    → yüksek kaliteli şema ve EDA export'ları
```

NetLang'in AI modelini kendi içinde barındırması zorunlu değildir. Öncelikli hedef, dışarıdaki herhangi bir yetkin AI ajanının CLI'ın versioned JSON sözleşmesi üzerinden NetLang'i güvenilir bir **tasarım oracle'ı, simülasyon motoru ve doğrulama aracı** olarak kullanabilmesidir. Ayrı bir Agent API servisi zorunlu değildir; ilerideki SDK/MCP adaptörleri aynı Core ve CLI sözleşmesinin ince yüzeyleri olabilir.

Kapsam kademeli genişler: ilk güçlü dikey analog ve karma-sinyal/SPICE tabanlı tasarımlardır; uzun vadeli mimari yalnızca eğitim devrelerine, basit örneklere veya tek bir endüstri alanına göre sınırlandırılmaz. Desteklenmeyen fiziksel alanlar ve simulator sınırları açıkça raporlanır; doğrulanmayan bir tasarım doğrulanmış gibi sunulmaz.

### Geliştirme ve yayın modeli

NetLang, LLM ajanlarıyla yürütülen agentic-first bir geliştirme projesidir. Fazlar klasik ekip takvimi, süre tahmini veya erken MVP yayın dilimleri değildir; teknik bağımlılıkları, doğrulama kanıtlarını ve tamamlanma sırasını takip eden kalite kapılarıdır. Geleneksel geliştirme süresi varsayımları gerekçe gösterilerek ürün vizyonu daraltılmaz.

Repository bütünleşik ürün geliştirmesi sırasında private kalır. CLI/Agent yüzeyi, güvenilir simulation/assertion runtime, Web Hub, profesyonel şema ve hedef EDA export'ları ilgili kabul kriterlerini birlikte karşıladıktan sonra public yayın yapılır. Prototip davranışlar roadmap içinde dürüstçe işaretlenir; fakat sırada bekleyen dış kullanıcı veya erken yayın baskısı ürün sırasını belirlemez.

**Ürün sanity-check kararı (2026-08-09):**

- [x] Agentic-first geliştirme hızını, fazların takvim değil kanıt kapısı olduğunu ve bütünleşik yayın stratejisini roadmap'e kaydet.
- [x] Web Hub'ı root README'de CLI ile eşit önemde ana ürün yüzeyi olarak görünür kıl; mevcut prototip ile yayın hedefini açıkça ayır.

**Yayın öncesi reality-check kararı (2026-08-12):**

- [x] Agent API'yi ayrı ürün gibi konumlandırma; CLI human/JSON modlarını aynı domain sonucunun iki renderer'ı olarak tanımla.
- [x] Varsayılan JSON'un agent döngüsü için kompakt, derin AST/IR/graph/SPICE/raw-log alanlarının opt-in olması gerektiğini Faz 3 sözleşmesine ekle.
- [x] Gerçek devre tasarımı için zorunlu model/subcircuit taşınabilirliğini ve temel component kapsamını ilk public yayın öncesine al.
- [x] Gain, bandwidth, output power, efficiency, distortion ve component dissipation gibi ürünün değerini oluşturan ölçümleri Faz 3'e al.
- [x] İlk public yayın kapısına canonical ürün benchmark'ları, cross-platform CLI paketleri, language reference ve destek matrisi ekle.

**Faz 4 ürün sırası kararı (2026-08-13):**

- [x] Faz 4'ü birbirinden kopuk subsystem teslimleri yerine RC ile başlayan ve power-amplifier'a genişleyen dikey ürün dilimleri olarak planla.
- [x] Mevcut Web/layout kodunu kanıtsız silme veya koruma; browser ve schematic characterization sonrasında katman bazında reuse/refactor/rewrite kararı ver.
- [x] Web Hub içine gömülü doğal-dil AI tasarım/chat arayüzünü ilk public yayın için kritik yol dışında tutup provider-independent Faz 5+ fikri olarak kaydet.
- [x] Şema ve EDA export'larını Web'e özel özellik yapma; bütün desteklenen formatları canonical Schematic IR kullanan Core exporter adaptörleri ve CLI artifact sözleşmesi üzerinden sun.

### Üç ana ürün yüzeyi

| Yüzey | Hedef kullanıcı | Temel çıktı |
|---|---|---|
| NetLang Core | Tüm sistem | Parser, AST, typed IR, graph, ERC, simulation, ölçüm ve layout |
| NetLang CLI (human + JSON) | AI ajanları, otomasyon ve geliştiriciler | Tasarla-ölç-doğrula döngüsü için insan-okunur ve structured araç yüzeyi |
| NetLang Web Hub | Kurulumsuz ürün deneyimi isteyen herkes | Kod/tasarım → doğrulama → şema → simülasyon → export → paylaşım |

### Birinci sınıf ürün çıktıları

- Structured engineering diagnostics ve ölçümler
- Constraint/assertion PASS, FAIL, ERROR sonuçları
- Deterministik SPICE ve simulation dataset'leri
- Okunabilir ve bağlantısal olarak doğrulanmış şema
- PNG, SVG ve ileride PDF gibi görsel export'lar
- LTspice, KiCad schematic ve ileride diğer EDA formatları için düzenlenebilir export'lar
- CLI ve Web arasında aynı Core semantiği

### Mevcut teknik yığın

- Core: Rust, tek `netlang-core` crate
- Parser: `pest` PEG grammar
- Native simulator: Windows sidecar Ngspice executable
- Web: React, TypeScript, Vite ve WASM
- Şema: Rust layout verisi, React SVG renderer ve deneysel KiCad export

---

## 2. Durum Modeli

Roadmap görevleri aşağıdaki anlamlarla takip edilir:

- `[x]`: Kod yazılmış, kabul kriteri doğrulanmış ve ilgili kalite kapıları geçmiştir.
- `[ ]`: Yapılmamış veya kabul kriteri henüz kanıtlanmamıştır.
- `PROTOTİP`: Çalışan kod vardır fakat API, test veya hata yönetimi tamamlanmamıştır.
- `ERTELENDİ`: Bilinçli biçimde ileriki milestone'a taşınmıştır; mevcut fazın blocker'ı değildir.

Bir başlık yalnızca alt görevlerin tamamı ve kabul kriterleri doğrulandıktan sonra tamamlanmış sayılır. Üst başlıkta bağımsız `[x]` işareti kullanılmaz.

### Tamamlanma kanıtı

Bir görev tamamlanırken mümkün olan yerlerde aşağıdakilerden biri eklenmelidir:

- Otomatik test adı veya fixture
- Doğrulama komutu
- İlgili diagnostic/JSON sözleşmesi
- Gerekliyse kısa karar notu

Sadece kodun bulunması tamamlanma kanıtı değildir.

---

## 3. Değiştirilemez Mimari Kurallar

1. **IR tek backend kaynağıdır.** ERC, SPICE, layout ve structured output AST'yi atlayarak doğrudan çıktı üretmez.
2. **Component sözlüğü katmanlar arasında tutarlıdır.** Parser, IR, backend ve frontend aynı canonical component türlerini kullanır; compatibility davranışı yalnız parser sınırında kalır.
3. **ERC, DRC değildir.** Schematic seviyesindeki kontroller ERC olarak adlandırılır.
4. **Net üretimi deterministiktir.** Aynı canonical circuit aynı node adlarını ve byte-for-byte aynı SPICE çıktısını üretmelidir.
5. **User-named net otomatik adı ezer; belirsizlik sessiz çözülmez.** Aynı fiziksel nete birden fazla kullanıcı adı verilirse diagnostic üretilir.
6. **Geçersiz değer fail-open olamaz.** Hatalı veya eksik değer sessizce `0`, boş model adı ya da geçersiz SPICE satırına dönüşmez.
7. **Yeni syntax mevcut geçerli örnekleri sebepsiz yere kırmaz.** Bilinçli breaking change ayrı migration notu gerektirir.
8. **Warning ve error farklıdır.** Warning tek başına derlemeyi durdurmaz; error downstream çıktı üretimini engeller.
9. **Test ve build artifact'leri kaynak kontrolüne girmez.** Doğrulama komutlarından sonra çalışma ağacı temiz kalır.
10. **Faz kapısı kanıtla kapanır.** `fmt`, `clippy`, test ve ilgili build adımları geçmeden milestone tamamlanmaz.

---

## 4. Başlangıç Denetimi ve Güncel Durum

> 2026-08-08 başlangıç denetiminde bulunan borçlar Faz 2.5 görevlerinin kaynağıdır. Aşağıdaki durum 2026-08-09 itibarıyla günceldir; tarihsel test sayıları ve kapanmış riskler ilgili görevlerin kanıt notlarında korunur.

### 4.1 Doğrulanmış çalışan parçalar

- [x] PEG parser ve AST üretimi mevcut.
- [x] Module/use flattening temel örnekte çalışıyor.
- [x] Versioned `compile_source -> CompileReport` hattı mevcut; CLI ve WASM aynı hattı kullanıyor.
- [x] Resistor, capacitor, inductor, diode, BJT, MOSFET, op-amp, voltage source ve current source türleri tanımlı.
- [x] User-named net syntax'ı parse ediliyor ve temel örnekte SPICE node adı olarak kullanılıyor.
- [x] Typed DC/sine source syntax'ı temel örnekte parse ediliyor.
- [x] Assertion syntax'ı IR'ye ve `.meas` komutlarına taşınıyor.
- [x] ERC için `NL-E001`–`NL-E009`, parser için `NL-P001`, semantic conversion için `NL-Cxxx` diagnostic alanları mevcut.
- [x] `check`, `compile`, `simulate`, `test` ve stub `render` subcommand'leri mevcut.
- [x] Ngspice sidecar executable repo ortamında çalışıyor.
- [x] CLI success/simulation failure/assertion failure yolları process-boundary integration testleriyle exit 0/3/4 üretiyor.
- [x] Web default source ortak example'dan geliyor; SPICE, layout ve KiCad backend smoke testi var.

### 4.2 Denetimde doğrulanan kalite durumu

| Kontrol | Sonuç | Açıklama |
|---|---|---|
| `cargo test --all-targets` | Geçiyor | 111 test; parser/IR/compiler/graph/ERC/SPICE/CLI/runtime/measurement/benchmark kapsamı mevcut |
| `cargo fmt -- --check` | Geçiyor | Rust kaynakları canonical `rustfmt` biçiminde |
| `cargo clippy --all-targets -- -D warnings` | Geçiyor | Mevcut target'larda warning yok |
| `npm run build` | Geçiyor | WASM paketini sıfırdan üretip web production build'i tamamlıyor |
| `npm run lint` | Geçiyor | React hook dependency uyarısı giderildi; warning yok |
| Örnek CLI matrisi | Geçiyor | Dört geçerli example check+compile oluyor; intentionally-invalid `test_amp.nl`, `NL-E003`/exit 1 veriyor |

### 4.3 Kalan kritik açıklar

- Dağıtılan Ngspice runtime yalnız Windows x86-64 sidecar'dır; Linux/macOS paketleme tamamlanmadı. `NETLANG_NGSPICE` açık executable override'ı mevcuttur.
- Layout/KiCad çıktısı deneyseldir; connectivity round-trip, collision ve gerçek KiCad açılabilirlik fixture'ları tamamlanmadı.
- Web runtime için otomatik gerçek-browser smoke testi yok; WASM build + TypeScript/Vite build ve Rust all-output smoke testi mevcut.
- Browser içinde simulation/measurement adaptörü ve grafik yüzeyi henüz native runner ile aynı ürün kabul seviyesinde değildir.

### 4.4 Faz yorumu

- Faz 0: Tarihsel MVP tamamlandı.
- Faz 1: Golden/regression ve fail-closed kabul kriterleri Faz 2.5 içinde kapatıldı.
- Faz 2: Typed IR, determinism, semantic validation ve regression borçları Faz 2.5 içinde kapatıldı.
- Faz 3: Typed simulation, measurement, assertion, agent CLI, model registry ve gerçek ürün benchmark'larıyla tamamlandı.
- Aktif çalışma: Faz 4 ayrıntılarını Faz 3'te doğrulanan Core sözleşmeleri üzerine yeniden denetlemek; ardından profesyonel şema, Web Hub ve yayın kapısını sırayla uygulamak.

---

## 5. Faz 2.5 — Stabilizasyon ve Sağlamlaştırma

### Hedef

Faz 0–2 özelliklerinin temiz clone'da reproducible, deterministik, fail-closed, testlerle korunan ve CLI/WASM/Web yüzeylerinde tutarlı çalıştığını kanıtlamak.

### Başarı kriteri

- Generated artifact'ler source control'u kirletmiyor.
- Rust format, Clippy, test, WASM ve web build kapıları geçiyor.
- Geçersiz input sessizce elektriksel olarak farklı bir circuit'e dönüşmüyor.
- Aynı canonical circuit byte-for-byte aynı SPICE'ı üretiyor.
- CLI ve WASM aynı compile pipeline'ından aynı diagnostic/IR/SPICE sonucunu alıyor.
- Web default örneği güncel core ile çalışıyor.
- Dokümanlar gerçek davranışı anlatıyor.

### Faz 2.5 dışında kalanlar

Bu milestone sırasında aşağıdaki özellikler uygulanmayacaktır:

- Tam OP/transient/AC sonuç parser'ı
- Datasheet tabanlı voltage/current limit kontrolleri
- Confidence HIGH/MEDIUM/LOW skoru
- Layout round-trip connectivity doğrulaması
- Web simulation graph'ları
- Yeni component aileleri

---

### 2.5.0 — Roadmap ve durum sözleşmesi

- [x] Faz 3'ün yanlış tamamlandı işaretini kaldır.
- [x] Aktif milestone'u Faz 2.5 olarak tanımla.
- [x] Faz 0–3 mevcut durumunu repo denetimi kanıtlarıyla kaydet.
- [x] Görev durumlarının anlamını ve milestone kapısını tanımla.
- [x] Faz 3 kapsamından datasheet-limit ve confidence zorunluluklarını ayır.
- [x] `docs/architecture.md` içindeki mevcut-durum iddialarını bu roadmap ile eşitle. _(2.5.7'de tamamlandı.)_
- [x] `docs/cli_reference.md` sözleşmesini gerçek CLI davranışıyla eşitle. _(2.5.7'de tamamlandı.)_

**Kabul kriteri:** Roadmap içinde tamamlandı görünen fakat alt görevleri boş olan faz bulunmuyor.

---

### 2.5.1 — Repo ve build hijyeni

- [x] Kök `.gitignore` oluştur.
- [x] `core/target` dizinini Git tracking'den çıkar; yerel build cache'i silme.
- [x] `core/out.txt` debug çıktısını tracking'den çıkar.
- [x] Generated `examples/*.spice` dosyaları için açık politika belirle:
  - `examples/` yalnızca kullanıcıya yönelik `.nl` kaynaklarını tutar.
  - Derlenen `examples/*.spice` dosyaları generated output olarak ignore edilir.
  - Regression oracle'ları `core/tests/fixtures/golden/` altında açıkça takip edilir.
- [x] `core/pkg` için tek ve belgelenmiş WASM build komutu oluştur: `cd webapp; npm run build:wasm`.
- [x] Web build'in önceden oluşturulmuş yerel artifact'e gizlice bağımlı olmamasını sağla:
  - `npm ci`, `core/pkg` bulunmayan çalışma ağacında doğrulandı.
  - `npm run build`, önce WASM paketini üretir ve sonra web production build'i çalıştırır.
- [x] `wasm-pack-init.exe` ve `wasm-pack-init.stamp` yerel bootstrap artifact'lerini tracking'den çıkar ve ignore et.
- [x] Ngspice runtime için gerekli minimum dosya setini, lisansı ve sürümü belgeleyip doğrula:
  - Windows x86-64 `ngspice-46` konsol runtime'ı temiz geçici dizinde doğrulandı.
  - Tutulan dosyalar ve lisans/release kapısı `core/tools/ngspice/README.md` içinde kayıtlı.
- [x] Ngspice kaynak/test ağacının tamamının repoda tutulup tutulmayacağına karar ver:
  - GUI, 707 upstream örneği, opsiyonel XSPICE/OSDI kütüphaneleri ve fazla vendor dokümanları kaldırıldı.
  - Minimal analog runtime 7 dosya ve yaklaşık 8.36 MB olarak tutuluyor.
- [x] Kök doğrulama script'i ekle:
  - Windows PowerShell: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1`
  - PowerShell 7: `pwsh -NoProfile -File scripts/verify.ps1`
- [x] Build toolchain sürümlerini sabitle:
  - Minimum Rust `1.97.1`: `core/Cargo.toml`; CI aynı exact sürümü, rustfmt, Clippy ve `wasm32-unknown-unknown` target'ını kurar.
  - Node.js `24.15.0`: `.nvmrc`
  - CI wasm-pack `0.13.1`: `.github/workflows/ci.yml`
- [x] CI'yı `origin/main` üzerinde yeşil doğrula:
  - [x] `.github/workflows/ci.yml` içine Rust fmt, Clippy, test ve release build kapılarını ekle.
  - [x] Aynı job'a WASM build, web lint ve web production build kapılarını ekle.
  - [x] Yerel/CI drift'ini önlemek için `scripts/verify.ps1` entrypoint'ini kullan.
  - [x] Remote GitHub Actions run'ının başarıyla tamamlandığını doğrula. _[`db24ee3` için CI run #31282475618](https://github.com/Stapimaz/NetLang/actions/runs/31282475618), 2026-08-09 tarihinde 4m56s içinde başarıyla tamamlandı._
- [x] Build/test'in çalışma ağacında yeni non-ignored değişiklik üretmediğini önce/sonra Git snapshot'ıyla doğrula.
- [x] Production dependency audit bulgularını gider ve `npm audit --omit=dev` kapısını sıfır bilinen vulnerability ile doğrula:
  - 2026-08-09 baseline: 1 high, 3 moderate, 1 low.
  - Etkilenen zincirler: `vite -> postcss -> nanoid`, `vite-plugin-top-level-await -> uuid`, `@monaco-editor/react -> monaco-editor -> dompurify`.
  - `audit fix --force` kullanılmadan; doğrudan/transitive upgrade veya gereksiz plugin kaldırma kararı build smoke testiyle kanıtlanacak.

**Dependency audit kanıtı (2026-08-09):** Kullanılmayan `vite-plugin-top-level-await` ve `uuid` zinciri kaldırıldı; `nanoid` 3.3.18'e güncellendi. Monaco 0.56'nın exact `dompurify` 3.4.8 bağımlılığı, upstream yeni Monaco release'i bulunmadığı için aynı API hattındaki güvenlik yaması 3.4.13'e npm `overrides` ile sabitlendi. `npm audit --omit=dev` sıfır bulgu verdi; audit adımı `scripts/verify.ps1` içine zorunlu kapı olarak eklendi ve pluginsiz WASM/Web production build ile birlikte geçti.

**Kabul kriterleri:**

- `cargo test` çalışma ağacında tracked artifact değiştirmiyor.
- Temiz clone belgelenmiş komutla Core, WASM ve Web build edebiliyor.
- `core/target` Git tarafından takip edilmiyor.
- Generated output politikası dokümante ve testlerde uygulanıyor.

**Not:** Geçmiş Git objelerini küçültmek için history rewrite bu milestone'un zorunlu parçası değildir; gerekirse ayrı, açık onaylı bakım operasyonu olarak yapılır.

---

### 2.5.2 — Characterization ve regression test altyapısı

#### Test organizasyonu

- [x] `core/tests/` integration test yapısını oluştur.
- [x] `core/tests/fixtures/valid` corpus'unu oluştur.
- [x] `core/tests/fixtures/invalid` parser/semantic corpus'unu oluştur.
- [x] `core/tests/fixtures/golden` snapshot altyapısını ve ilk canonical fixture'ı oluştur.
- [x] CLI testleri için process/sequence bazlı isolated temp directory kullan.
- [x] Snapshot/CLI helper'ında platform path'i, JSON-escaped path ve CRLF'i normalize et.

#### Parser testleri

- [x] Tüm `examples/*.nl` dosyalarını parser test matrisine al.
- [x] Boş dosya testi.
- [x] Yalnızca yorum testi.
- [x] UTF-8 BOM testi; frontend normalizasyonuna kadar açıkça reject edilir.
- [x] Unicode davranışı testi ve açık politika: yorumlarda destekli, identifier'lar ASCII.
- [x] Eksik `to`, eksik değer ve eksik parantez testleri.
- [x] Bilinmeyen component keyword testi.
- [x] Module/use/port flattening testleri.
- [x] Legacy syntax kararını testle kilitle: desteklenmeyen eski component keyword'leri ve `to` içermeyen `connect` formu açıkça reject edilir.

#### IR testleri

- [x] `10k`, `2.2k`, `100uF`, `1MHz`, negatif değer ve scientific notation testleri; scientific notation mevcut durumda açıkça reject edilir.
- [x] Mevcut sine parametrelerinin SI scaling testi.
- [x] Mevcut builtin NPN/PNP/NMOS model çözümleme testi; PMOS builtin henüz yok.
- [x] Bilinmeyen model davranışı testi.
- [x] Assertion threshold/unit testleri.

**2.5.3'e taşınan desired-behavior testleri:** voltage/current fiziksel unit ayrımı, pulse parametreleri, PMOS politikası ve hatalı değerin `0.0` olmaması. Bu beklentiler mevcut fail-open davranışı golden'lamadan, ilgili semantic değişiklikten hemen önce yazılacaktır.

#### Graph/ERC testleri

- [x] Connection sırası değişse de aynı canonical SPICE çıktısı testi.
- [x] Component declaration sırası değişse de aynı canonical SPICE sonucu testi.
- [x] Tek voltage source için GND canonicalization testi.
- [x] Birden fazla bağımsız source için lexicographic legacy ground fallback characterization testi.
- [x] User-named net önceliği testi.
- [x] Invalid/undefined component testi (`NL-E002`).
- [x] NL-E001–NL-E004 için ayrı regression fixture'ları ve testleri.
- [x] Diagnostic sıralamasının aynı circuit için deterministik olduğu stress testi.

**2.5.4'e taşınan desired-behavior testleri:** multiple-ground ambiguity, aynı nete iki kullanıcı adı conflict'i, invalid pin ve warning/non-blocking davranışı. İlgili diagnostic'ler henüz bulunmadığı için mevcut sessiz kabul davranışı sözleşmeye dönüştürülmeyecektir.

#### SPICE golden testleri

- [x] Minimal source/resistor circuit canonical golden netlist'i.
- [x] `demo_circuit.nl` golden netlist.
- [x] `wheatstone.nl` golden netlist.
- [x] `test_features.nl` golden netlist.
- [x] Current source ve sine source golden netlist.
- [x] Named net golden netlist.
- [x] Model injection sırası golden testi.
- [x] Assertion `.meas` golden testi.
- [x] NC davranışını açıkça tanımlayan golden test.

#### CLI contract testleri

- [x] Human ve JSON success testleri.
- [x] Parse, I/O ve ERC hata testleri.
- [x] Exit code 0/1/2 testleri.
- [x] Global `--format` yerleşim testi; mevcut sözleşmede option subcommand'den önce gelir.
- [x] JSON stdout'un parse/I-O loglarıyla kirlenmediği test.

**2.5.5/Faz 3'e taşınan desired-behavior testleri:** simulation error/exit 3, assertion failure/exit 4, `render` stub nonzero exit ve simulator JSON stdout saflığı. Platform/runtime bağımlı bu sözleşmeler ilgili fail-closed uygulamayla birlikte eklenecektir.

**Kabul kriterleri:**

- Kritik parser/IR/graph/ERC/SPICE/CLI dalları otomatik testlerle korunuyor.
- En az 35 anlamlı test var; sayı tek başına yeterli kabul edilmiyor.
- Tüm geçerli örnekler ve intentionally-invalid fixture'lar beklenen sonucu veriyor.
- `cargo test` CI ortamında deterministik geçiyor.

**Kanıt:** 35 test yerelde `cargo test --all-targets` ile, aynı suite kök doğrulama akışında Rust release + WASM + Web kapılarıyla birlikte geçmiştir. Remote GitHub Actions sonucu private-repo erişimi nedeniyle 2.5.1 altında ayrıca açık tutulur.

---

### 2.5.3 — Typed IR ve semantic validation

- [x] `parse_si_value` yerine sayı, SI prefix ve fiziksel birimi ayrı doğrulayan parser oluştur. _(Eski public helper strict parser'a compatibility wrapper olarak bağlıdır.)_
- [x] Decimal, negatif ve scientific notation desteğini testlerle tanımla.
- [x] Desteklenmeyen trailing text'i semantic error yap.
- [x] Bütün `unwrap_or(0.0)` fail-open dönüşümlerini kaldır.
- [x] Eksik passive value'yu semantic error yap.
- [x] Voltage source ve current source parametrelerini fiziksel olarak doğru ayrı tiplerle temsil et.
- [x] DC ve waveform source değerlerini typed enum ile temsil et.
- [x] Waveform parametre sayısını ve her parametrenin unit boyutunu doğrula.
- [x] `Unknown { original_value }` fallback'ini yalnızca açıkça güvenli kullanım varsa koru; aksi halde kaldır. _(Fallback kaldırıldı; module port ve diode açık parametre variant'ları kullanıyor.)_
- [x] Bilinmeyen model politikasını tanımla:
  - [x] Builtin model: `resolve_model` ile typed kind + provenance.
  - [x] User-defined model: declaration/include syntax tanımlanana kadar desteklenmiyor ve sessiz kabul edilmiyor.
  - [x] Unsupported/unknown model: `NL-C003`; model kind/polarity mismatch: `NL-C004`.
- [x] Modeli olmayan BJT/MOSFET/diode/op-amp'ın geçersiz SPICE üretmesini engelle.
- [x] Assertion signal, comparator, threshold ve unit validation ekle.
- [x] IR conversion diagnostic'leri için `NL-Cxxx` kod alanı oluştur.
- [x] IR tiplerine gerekli Serde desteğini ekle.
- [x] WASM structured output'a typed IR ekle.

**Typed quantity paketi kanıtı (2026-08-09):** `Quantity + SIUnit`, ayrı `VoltageSource`/`CurrentSource` parametreleri ve `SourceValue::{Dc, Waveform}` IR sözleşmesine eklendi. SINE/PULSE değer, frekans ve zaman boyutları strict doğrulanıyor; invalid/missing değerler IR'ye ulaşmıyor. `ir_characterization` içindeki 12 test dahil toplam 40 Rust testi, mevcut SPICE golden corpus'u ve kök `scripts/verify.ps1` Rust release + WASM + Web kapılarıyla birlikte geçti. Büyük/küçük harf BJT polarity parser sınırında normalize edildi; `examples/test_amp.nl` artık boş model token'ı yerine default 2N3904 modeline çözülür.

**Model/diagnostic paketi kanıtı (2026-08-09):** `SemanticDiagnostic` için `NL-C001..NL-C007` alanı oluşturuldu. Builtin default model, unsupported model, model-kind/polarity mismatch, modelsiz op-amp, explicit module-port parametresi ve serializable diagnostic davranışları `ir_characterization` ile; CLI semantic exit 1 ve saf JSON diagnostic sözleşmesi `cli_contract` ile korunuyor. Bu dilimdeki legacy WASM parse/semantic alanları daha sonra 2.5.5 ortak `diagnostics` sözleşmesiyle değiştirildi. Toplam 45 Rust testi ile Rust fmt/Clippy/release, SPICE golden, WASM ve Web lint/build kapılarının tamamı `scripts/verify.ps1` üzerinden geçti.

**Kabul kriterleri:**

- Geçersiz veya eksik değer sessizce `0` olmaz.
- Geçersiz model boş SPICE token'ı üretmez.
- Current source, voltage isimli alanla temsil edilmez.
- CLI ve WASM semantic hataları structured diagnostic olarak verir.
- Tüm eski geçerli örnekler bilinçli migration kararı olmadan kırılmaz.

---

### 2.5.4 — Deterministik graph ve ERC sağlamlaştırması

- [x] `9999` sentinel yerine `Option<NetId>` veya typed lookup sonucu kullan.
- [x] Component pin tanımlarını graph/ERC/layout/SPICE için tek merkezde topla.
- [x] Invalid pin reference diagnostic'i ekle.
- [x] Component/net namespace collision politikasını tanımla ve test et.
- [x] Graph traversal başlangıçlarını canonical pin sırasıyla üret.
- [x] Net ID üretimini collection iteration sırasından bağımsız yap.
- [x] Ground çözümleme politikasını tamamla:
  - [x] Explicit GND/reference net önceliği.
  - [x] Legacy devreler için lexicographic primary source-minus fallback.
  - [x] Birden fazla bağımsız olasılıkta ambiguity diagnostic.
- [x] User-named net conflict diagnostic'i ekle.
- [x] Diagnostic'leri code/component/pin temelinde canonical sırala.
- [x] Component emission ve standard model injection için sıralı collection kullan.
- [x] SPICE sayı formatını canonical ve platform bağımsız yap.
- [x] Aynı circuit'i 100 kez tekrarlı derleyen byte-for-byte determinism stress testi ekle.

**Typed net/pin kataloğu kanıtı (2026-08-09):** `NetId` serde-transparent newtype olarak tanımlandı; `get_net` bağlantısız pin için `None` döndürüyor ve `9999` sentinel üretim kodundan kaldırıldı. `component.rs` pin adları, canonical SPICE sırası, layout koordinatları, source prefix'i ve signal/through metadata'sı için graph/ERC/SPICE/layout'un ortak kaynağıdır. `invalid_pin.nl` artık `NL-E005` üretir. Katalog invariant ve sentinel-yokluğu regression testleriyle toplam 47 Rust testi, golden SPICE ve tam Rust/WASM/Web kapısı geçti.

**Namespace/ground paketi kanıtı (2026-08-09):** Component/net collision `NL-E006`, aynı fiziksel net üzerindeki birden fazla user name `NL-E007`, bağımsız ground adayları `NL-E008`, duplicate net declaration `NL-E009` üretir. Explicit `net GND` legacy source-minus fallback'ten önce gelir; explicit referans yoksa fallback lexicographic ve deterministiktir fakat ambiguity artık sessiz değildir. ERC diagnostic'leri `(code, component, pin, message)` ile canonical sıralanır. Dört invalid fixture ve explicit/legacy ground regression testleri dahil toplam 50 Rust testi; golden SPICE, audit=0 ve tam Rust/WASM/Web kapısı geçti.

**Canonical SPICE number kanıtı (2026-08-09):** Passive, DC source ve SINE/PULSE/PWL waveform değerleri tek `format_spice_number` yolundan geçer. Negative zero `0`, orta aralık trimlenmiş decimal, küçük/büyük değer normalize edilmiş lowercase exponent olarak yazılır; binary float artıkları golden netlist'e sızmaz (`100uF -> 1e-4`). Formatter regression testiyle toplam 51 Rust testi ve güncellenmiş golden corpus geçti. Üretilen `demo_circuit.spice` ayrıca gömülü Ngspice 46 batch OP analizini exit 0 tamamladı; audit=0 dahil tam Rust/WASM/Web kapısı geçti.

**Kabul kriterleri:**

- Aynı canonical circuit 100 tekrarda byte-for-byte aynı SPICE üretir.
- Bağlantı ekleme sırası node adını değiştirmez.
- Birden fazla user name veya ground ambiguity sessizce çözülmez.
- Backend'ler ortak component/pin tanımlarını kullanır.

---

### 2.5.5 — Tek compile pipeline ve CLI sözleşmesi

- [x] Library seviyesinde tek compile entrypoint tasarla:

  ```rust
  compile_source(source, options) -> CompileReport
  ```

- [x] `CompileReport` içinde aşağıdaki alanları tanımla:
  - Schema version
  - AST (opsiyonel/debug)
  - Typed IR
  - Diagnostics
  - Net/graph özeti
  - SPICE netlist
  - Layout
  - KiCad schematic _(opsiyonel backend çıktısı)_
- [x] Parser → flatten → IR → graph → ERC → backend sırasını tek yerde uygula.
- [x] CLI'nin bu entrypoint'i kullanmasını sağla.
- [x] WASM'in aynı entrypoint'i kullanmasını sağla.
- [x] Error severity varsa downstream output üretimini tek merkezden engelle.
- [x] Warning varsa başarılı output ile birlikte döndür.
- [x] JSON çıktıya `schema_version` ekle.
- [x] `--format` seçeneğini gerçek global CLI option yap.
- [x] Exit code sözleşmesini kesinleştir:
  - 0: success
  - 1: semantic/ERC failure
  - 2: parse/I-O failure
  - 3: simulation failure
  - 4: assertion failure
- [x] JSON stdout'a log veya progress yazma; logları stderr'e taşı.
- [x] `render` uygulanmadıysa nonzero error ver veya uygulanana kadar CLI yüzeyinden kaldır.
- [x] `simulate` JSON success sonucunun process status ve simulator errors ile uyumlu olmasını sağla.
- [x] Output file ve overwrite politikasını tanımla.

**Compile API dilimi kanıtı (2026-08-09):** `netlang.compile.v1` şema sürümüne sahip, filesystem/process I/O yapmayan `compile_source(source, options) -> CompileReport` çekirdek entrypoint'i eklendi. Parse (`NL-P001` + satır/sütun), flatten (`NL-C008`), semantic (`NL-Cxxx`) ve ERC (`NL-Exxx`) sonuçları stage/severity bilgili ortak diagnostic tipine normalize ediliyor. IR ve deterministik sıralı graph özeti raporda korunurken error-severity diagnostic backend üretimini merkezi olarak kesiyor; AST, SPICE, layout ve KiCad çıktıları typed options ile seçiliyor. Bu kütüphane sözleşmesi 9 yeni integration testiyle korunuyor. CLI ve WASM migrasyonu ayrı, sıradaki paketlerdir; bu aşamada mevcut dış sözleşmeleri değiştirilmedi.

**WASM adaptör dilimi kanıtı (2026-08-09):** `compile_netlang`, kendi parse/flatten/IR/graph/ERC/backend zincirini kurmak yerine yalnızca `compile_source(..., CompileOptions::all_outputs())` raporunu JS'e serialize eden fail-closed bir adaptöre indirildi. Web tüketicisi legacy `parse_error`/`erc_errors` alanlarından sürümlü ortak `diagnostics` sözleşmesine geçirildi; schema uyuşmazlığı ve error severity eski çıktıları ekranda bırakmadan hata veriyor. Kök kalite kapısındaki WASM package ve Web TypeScript/lint/build adımları bu entegrasyonu doğrular.

**CLI sözleşme dilimi kanıtı (2026-08-09):** `check/compile/simulate/test` artık parse/IR/graph/ERC zincirini tekrarlamıyor ve canonical `CompileReport` kullanıyor. CLI JSON'u `netlang.compile.v1` raporunu `status`, `spice_file` ve `tests` alanlarıyla genişletiyor; integration testi AST/IR/diagnostics/graph/SPICE alanlarının library raporuyla birebir aynı olduğunu doğruluyor. `--format` iki konumda da çalışıyor; parse/I-O=2, semantic/ERC=1, simulation=3 ve assertion=4 yolları tek format-bağımsız akışta tanımlı. Output varsayılanı `<source>.spice`; mevcut hedef yalnız `--force` ile eziliyor, kaynak dosya hedef olamıyor. `render` `NL-F001`/exit 2 ile fail-closed. Simulator process status ve error/fatal/aborted çıktısı başarı JSON'una dönüşmüyor; gömülü Ngspice ile JSON success smoke testi de geçti. `NETLANG_NGSPICE` override'ı üzerinden çalışan platform-bağımsız fake-process integration testleri success/simulation-error/assertion-failure JSON ve exit 0/3/4 yollarını koruyor. CLI reference'taki beş komut ailesinin davranış matrisi integration seviyesinde kapsandı; ilgili Rust suite toplam 66 teste çıktı.

**Kabul kriterleri:**

- Aynı source CLI ve WASM'de aynı IR, diagnostics ve SPICE sonucunu üretir.
- Human ve JSON modları aynı exit code semantiğini kullanır.
- Error JSON ile exit code 0 birlikte görülmez.
- CLI reference'taki bütün örnekler integration test olarak çalışır.

---

### 2.5.6 — Web/WASM senkronizasyonu

- [x] Web default kodunu güncel grammar'a geçir.
- [x] Default kodda `source` terminolojisi kullan.
- [x] Eski `connect A B` syntax'ını güncel `connect A to B` ile değiştir.
- [x] Default circuit'i mümkünse ortak fixture/example üzerinden yükle.
- [x] SVG renderer'a canonical voltage-source sembolünü ekle; eski fallback'i kaldır.
- [x] `CurrentSource` sembolünü veya açık geçici fallback'i tanımla.
- [x] KiCad export'ta `Source`/`CurrentSource` mapping'ini düzelt.
- [x] Layout içindeki eski voltage-source fallback'lerini kaldır.
- [x] WASM package build'ini npm/kök build akışına bağla.
- [x] WASM result için `any` yerine TypeScript interface/generated type kullan.
- [x] React hook lint uyarısını düzelt.
- [x] Kullanılmayan Vite template CSS ve asset'lerini temizle. _(`App.css`, template hero/React/Vite görselleri ve kullanılmayan public icon seti kaldırıldı.)_
- [x] Web default circuit compile smoke testi ekle.

**Web/WASM senkronizasyon dilimi kanıtı (2026-08-09):** Web editörünün hard-coded ve legacy syntax kullanan kaynağı kaldırıldı; default içerik doğrudan repository'deki golden-korumalı `examples/demo_circuit.nl` dosyasından raw import ediliyor. Aynı dosya Rust integration testinde `CompileOptions::all_outputs()` ile diagnostics olmadan SPICE + layout + KiCad üretmek zorunda. Web compile/layout/diagnostic sınırındaki `any` tipleri explicit TypeScript interface'lere çevrildi. SVG renderer ayrı `Source` ve `CurrentSource` sembolleri kullanıyor; layout ve üretim Web kodu canonical component türleriyle eşitlendi. KiCad mapping'leri `Simulation_SPICE:VDC/IDC` olarak testle sabitlendi. Toplam 68 Rust testi, WASM release package, Web lint ve production build birlikte geçti.

**Kabul kriterleri:**

- Temiz clone'da WASM üretildikten sonra web build geçer.
- Default circuit parse, semantic validation, ERC, SPICE ve layout üretimini tamamlar.
- Canonical component türleri source kodu ve renderer mapping'lerinde tutarlıdır.
- `npm run lint` warning vermeden geçer.

---

### 2.5.7 — Doküman ve final kalite kapısı

- [x] Kök `README.md` oluştur:
  - Proje amacı
  - Hızlı başlangıç
  - Core/WASM/Web build
  - Örnek komutlar
- [x] `docs/architecture.md` dosyasını gerçek code path ve sınırlarla eşitle.
- [x] `docs/cli_reference.md` içine `test`, exit code 4, JSON schema ve option yerleşimini ekle.
- [x] Repo talimatlarını root `AGENTS.md` konumuna taşı; component kataloğu ve canonical `scripts/verify.ps1` kalite kapısıyla eşitle.
- [x] `webapp/README.md` Vite template metni yerine gerçek Web Hub dokümanı yap.
- [x] Ngspice runtime sürüm/lisans/dağıtım belgesini ekle: `core/tools/ngspice/README.md`.
- [x] Roadmap Faz 2.5 checkbox'larını yalnızca kanıtlanan sonuçlara göre kapat.
- [x] Faz 3 başlangıç denetimi yap ve audit notu ekle.

#### Zorunlu final komutları

```powershell
cd core
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release

cd ../webapp
npm.cmd ci
npm.cmd audit --omit=dev
npm.cmd run lint
npm.cmd run build:wasm
npm.cmd run build:web

# Aynı kapıları kökten sırayla çalıştıran Windows komutu
cd ..
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1

# PowerShell 7 alternatifi
pwsh -NoProfile -File scripts/verify.ps1
```

#### Final kabul matrisi

- [x] Rust fmt geçiyor.
- [x] Rust Clippy `-D warnings` ile geçiyor.
- [x] Tüm Rust ve CLI testleri geçiyor.
- [x] Release build geçiyor.
- [x] WASM build geçiyor.
- [x] Web lint warning vermeden geçiyor.
- [x] Web production build geçiyor.
- [x] Production npm audit sıfır bilinen vulnerability ile geçiyor.
- [x] Tüm geçerli örnekler check/compile matrisinden geçiyor.
- [x] Invalid corpus beklenen diagnostic kodlarını veriyor.
- [x] Determinism stress testi geçiyor.
- [x] Doğrulama sonrası Git çalışma ağacı temiz.

**Final yerel kapı kanıtı (2026-08-09):** Dependency kurulumu atlanmadan `scripts/verify.ps1` çalıştı: `npm ci` 50 paketi temiz kurdu ve audit etti; 69 Rust/CLI testi, fmt, Clippy `-D warnings`, release build, WASM release package, ikinci production audit (`0 vulnerabilities`), Web lint ve production build geçti. CLI example integration matrisi dört geçerli repository example'ını check+compile eder; intentionally-invalid `test_amp.nl` için `NL-E003`/exit 1 bekler. ERC `NL-E001..009`, parser/semantic invalid corpus ve 100x SPICE + 50x diagnostic determinism tekrarları testlerle korunur. Doğrulama scripti öncesi/sonrası worktree snapshot'ını karşılaştırarak yeni artifact oluşmadığını da denetledi; bu final doküman commit'i sonrası status ayrıca temiz doğrulanacaktır.

**Faz 2.5 kapanış kanıtı (2026-08-09):** Yerel final matrisinin tamamı ve aynı canonical `scripts/verify.ps1` akışını kullanan remote GitHub Actions run'ı geçti. Milestone kapandı; Faz 3 ön koşulu sağlandı.

---

## 6. Faz 3 — Simülasyon ve Assertion Runtime

### Ön koşul

Faz 2.5 final kabul matrisinin tamamı geçmelidir.

### Mevcut prototip

- [x] Ngspice sidecar subprocess ile çağrılabiliyor. _(PROTOTİP)_
- [x] Basit `.meas` satırları stdout'tan çıkarılabiliyor. _(PROTOTİP)_
- [x] Basit comparator evaluation çalışıyor. _(PROTOTİP)_
- [x] `netlang test` human/JSON temel sonuç üretiyor. _(PROTOTİP)_

Bu maddeler Faz 3'ün tamamlandığı anlamına gelmez; aşağıdaki structured runtime bunların yerini alacaktır.

### Faz 3 başlangıç audit'i — 2026-08-09

Faz 2.5 sırasında CLI'nin process/exit kabuğu sağlamlaştırıldı ancak simulation domain modeli halen prototiptir. Uygulamaya başlamadan önce geçerli code path gerçekleri:

1. `simulate`, SPICE dosyasını doğrudan `Command::output()` ile çalıştırırken `test`, `sim_result::run_simulation` yolunu kullanıyor; tek runner yok.
2. `run_simulation`, processler arasında paylaşılan sabit `netlang_temp.spice` yolunu kullanıyor; önce benzersiz run directory ve cleanup/artifact politikası gerekir.
3. `SimResult`, `HashMap<String, f64> + Vec<String>` taşır; process status, raw log, warning, analysis türü ve typed dataset domain alanları yoktur.
4. `.meas` parsing stdout satır bölme heuristic'idir; locale, exponent, duplicate measurement ve malformed output fixture'ları yoktur.
5. Missing measurement internal olarak `NaN`, CLI JSON'da `actual: null` olur; PASS/FAIL/ERROR/SKIPPED ayrımı yoktur.
6. Analysis komutları IR'de raw `cmd/args` string'idir; unsupported simulator komutu compile aşamasında doğrulanmaz.
7. `NETLANG_NGSPICE` override'ı ve exit 0/3/4 contract testleri vardır; executable version/capability check ve timeout/cancellation yoktur.

**Uygulama sırası kararı:** 3.1'de önce typed `SimulationRequest/Result`, analysis enum'u ve tek runner; ardından unique temp lifecycle + timeout. 3.2 parser/fixture katmanı bu domain üzerine kurulacak. 3.3 assertion result modeli, raw `f64/NaN` davranışını kullanan son consumer olarak daha sonra taşınacak. Mevcut stdout parser genişletilerek kalıcı API yapılmamalıdır.

### 3.1 — Simulation domain modeli ve runner

- [x] `SimulationRequest` ve `SimulationResult` domain tiplerini tanımla.
- [x] Domain sözleşmesini native process ve gelecekteki browser simulator adaptörlerinden bağımsız tut.
- [x] Analysis türlerini typed enum yap: OP, transient, AC, DC sweep.
- [x] Measurement, warning, error ve raw-log alanlarını ayır.
- [x] Simulator process status'unu structured biçimde sakla.
- [x] Her çalıştırma için benzersiz temp directory kullan.
- [x] Temp cleanup ve failure artifact saklama politikasını tanımla.
- [x] `simulate` ve `test` için tek runner kullan.
- [x] Ngspice executable discovery ve version check'i güvenilir yap.
- [x] Timeout/cancellation desteği ekle.

**3.1 kapanış kanıtı (2026-08-12):** `netlang.simulation.v1` request/result sözleşmesi ve backend-neutral `SimulationRunner` sınırı eklendi. OP/transient/AC/DC sweep artık typed ve unit-aware Circuit IR analysis varyantlarıdır; malformed veya unsupported analysis, module-flatten için ayrılmış `NL-C008` ile çakışmadan `NL-C009` diagnostic'iyle simulator öncesinde fail-closed olur. Native Ngspice runner executable discovery + version probe, structured process/log/warning/error/measurement alanları, benzersiz run directory, cleanup/retain-on-failure politikası, 30 saniyelik default timeout ve cancellation token uygular. `simulate` ile `test` aynı runner'ı kullanır. Fake-process integration testleri success, launch failure, simulator failure, timeout, cancellation, artifact retention ve paralel iki run izolasyonunu doğrular; gerçek bundled Ngspice 46 version probe'u ayrıca geçti. Canonical `scripts/verify.ps1` kapısı 81 Rust/CLI testi, fmt, Clippy `-D warnings`, release/WASM build, production npm audit (`0 vulnerabilities`), Web lint ve production build ile tamamen geçti.

### 3.2 — Structured Ngspice sonuçları

- [x] `.meas` parser'ını ayrı ve fixture tabanlı modül yap.
- [x] Operating point sonuçlarını structured map'e çevir.
- [x] Transient time-series sonuçlarını structured dataset'e çevir.
- [x] AC complex/frequency-domain sonuçlarını structured dataset'e çevir.
- [x] Mümkünse stdout tablo scraping yerine Ngspice raw/wrdata çıktısı kullan.
- [x] Warning, convergence ve fatal error sınıflandırması yap.
- [x] Locale, exponent ve line-ending fixture'ları ekle.

**3.2 kapanış kanıtı (2026-08-12):** Prototip stdout splitter kaldırıldı; `.meas` ve `wrdata` için ayrı `simulation_parser` modülü eklendi. Measurement parser duplicate/malformed/non-finite değerleri fail-closed reddeder; exponent, decimal-comma ve LF/CRLF fixture'larıyla korunur. Generated SPICE deterministic analysis artifact isimleri ve `wrdata all` kullanır. OP sorted scalar map'e, transient/DC real-series dataset'e, AC frequency + complex-series dataset'e dönüşür. `NL-S003..006` warning/convergence/fatal/result-parse sınıfları structured runtime diagnostics olarak taşınır. Bundled Ngspice ile gerçek OP, transient ve AC integration fixture'ları geçer; Linux CI aynı testler için Ngspice kurar. Canonical `scripts/verify.ps1` kapısı 88 Rust/CLI testi, gerçek Ngspice fixtures, fmt, Clippy `-D warnings`, release/WASM build, production audit (`0 vulnerabilities`), Web lint ve production build ile geçti.

### 3.3 — Assertion runtime

- [x] Her assertion'a deterministik `NL-Txxx` kodu ata.
- [x] Sonuç durumlarını tanımla: PASS, FAIL, ERROR, SKIPPED.
- [x] Missing measurement'ı NaN yerine açıklamalı ERROR yap.
- [x] Absolute ve relative tolerance politikası tanımla.
- [x] `peak` semantiğini absolute peak olarak kesinleştir.
- [x] OP analizinde desteklenen assertion metric'lerini tanımla.
- [x] Current direction/sign convention'ı belgele ve test et.
- [x] Unit-aware human output üret.
- [x] JSON sonuçlarına summary ekle.

Örnek hedef JSON:

```json
{
  "schema_version": "netlang.assertion.v1",
  "assertions": [
    {
      "code": "NL-T001",
      "status": "PASS",
      "metric": "max",
      "signal": "V(out)",
      "actual": 3.21,
      "threshold": 3.3,
      "unit": "V"
    }
  ],
  "summary": {
    "total": 1,
    "passed": 1,
    "failed": 0,
    "errors": 0
  }
}
```

**3.3 kanıtı:** Assertion motoru `netlang.assertion.v1` raporu, deterministik `NL-Txxx` kimlikleri ve dört ayrı durum üretir. Dataset-first metric çözümleme OP/transient/DC verisini typed olarak işler; eksik/uygunsuz veri `ERROR`, başarısız simulation `SKIPPED` olur. Absolute peak, OP ve current-sign semantiği ile absolute/relative tolerance regression testleriyle korunur. CLI human renderer engineering prefix + birim kullanır; JSON aynı domain raporunu summary ile serialize eder. Bundled Ngspice üzerinde gerçek `test_features.nl` smoke testi iki assertion'ı typed `PASS` sonucu ile tamamladı.

### 3.4 — Simulation CLI sözleşmesi

- [x] `netlang simulate` structured analysis sonucu döndürür.
- [x] `netlang test` structured assertion sonucu döndürür.
- [x] Human ve JSON modları aynı domain sonucunu render eder.
- [x] Varsayılan JSON envelope'u yalnız status, diagnostics, summary, measurements, assertions ve artifact referanslarını taşıyan kompakt agent çıktısı olsun.
- [x] AST, IR, graph, SPICE netlist, dataset ve raw simulator log gibi hacimli debug alanlarını açık `--include` seçimiyle opt-in yap.
- [x] Compile, simulation ve assertion şemalarını açıkça sürümle; bilinmeyen schema sürümü fail-closed olsun.
- [x] Simulation failure exit 3, assertion failure exit 4 verir. _(Faz 2.5 cross-platform fake-process contract testleri.)_
- [x] JSON stdout parse edilebilir ve logsuzdur. _(Success/failure/assertion CLI integration testleri.)_
- [x] Simulation fixture'ları CI'da güvenilir çalışır. _(3.2 real-Ngspice fixture'ları canonical suite ve Linux CI içinde; 3.4 CLI contract fake runner ile platformlar arası korunuyor.)_

**3.4 kanıtı:** `netlang.cli.v1` compact envelope `check`, `compile`, `simulate` ve `test` için ortak status/diagnostic/summary/measurement/assertion/artifact modelini kullanır. `netlang.compile.v1`, `netlang.simulation.v1` ve `netlang.assertion.v1` sürümleri `domain_versions` ile ilan edilir. AST/IR/graph/SPICE/dataset/raw-log yalnız `--include` ile `debug` altına girer. `--schema-version` bilinmeyen sürümü output oluşturmadan `NL-F002` ile fail-closed reddeder. Human ve JSON simulation render'ları aynı dataset/measurement sayısını contract testinde doğrular; JSON stdout raw logla kirlenmez.

### 3.5 — Agent-ready CLI contract

- [x] Ayrı daemon veya Agent API servisi kurmadan `check`, `compile`, `simulate` ve `test` komutlarını versioned JSON sözleşmesiyle sun.
- [x] Dosyasız tool kullanımı için NetLang source'u stdin'den alma seçeneği ekle.
- [x] Agent sonucunu human terminal metni parse etmeye gerek bırakmayacak şekilde typed, kompakt ve structured tut.
- [x] Aynı request için idempotent ve deterministik sonuç üret.
- [x] Dış bir AI ajanının compile → simulate → measure → revise döngüsünü fixture tabanlı uçtan uca testle doğrula.

**3.5 kanıtı:** `FILE` konumundaki `-`, source'u stdin'den alır; açık `--output` yoksa compile/simulate/test netlist'i yalnız bellekte kullanır. Dört komut da `netlang.cli.v1` envelope'u döndürür. Contract testi side-effect-free check/compile/simulate/test isteklerini ikişer kez çalıştırıp stdout'un byte-for-byte aynı olduğunu doğrular. E2E agent fixture'ı 10 Ω adayını compile eder, structured simulation measurement'ında 200 mA okur, `NL-T001=FAIL` sonucunu işler, source'u 100 Ω'a revize eder ve 20 mA ile `PASS` sonucuna ulaşır; hiçbir adım human terminal metni parse etmez.

### 3.6 — Simulation model ve subcircuit temeli

- [x] Güvenli, typed user-defined SPICE `.model` ve `.subckt` declaration/include sözleşmesi tasarla.
- [x] Subcircuit pin sırası ile NetLang component pinlerini ortak katalog üzerinden doğrula.
- [x] En az bir doğrulanmış op-amp, PMOS ve power-transistor yolu sun; dilde tanımlı hiçbir temel component türü bütünüyle kullanılamaz kalmasın.
- [x] Model kind, polarity, required pin, simulator capability ve isim çakışmalarını compile aşamasında fail-closed doğrula.
- [x] Model kaynağı, lisansı, sürümü ve content hash'ini taşıyan provenance manifest'i tanımla.
- [x] `netlang.lock` ile model/subcircuit çözümlemesini reproducible yap.
- [x] User model/subcircuit içeriğinin typed IR sınırını atlayarak kontrolsüz SPICE directive enjekte edememesini test et.

**3.6 kanıtı:** Typed `model diode|bjt|mosfet` ve fixed-template `subcircuit opamp` declaration'ları yalnız whitelist edilmiş numeric parametrelerden canonical directive üretir; raw SPICE source'a alınmaz. Ortak component kataloğu op-amp pin sırasını doğrular. `NL-C010..013` parameter/metadata, package, pin/capability ve case-insensitive name conflict alanlarını fail-closed ayırır. `NLANG_OPAMP_V1`, `NLANG_PMOS_V1` ve `NLANG_POWER_NPN_V1` aynı gerçek Ngspice OP fixture'ında başarıyla çalışır. `netlang.models.v1` source/license/version/simulator/SHA-256 provenance taşır; exact `netlang_analog@1.0.0` çözümlemesi deterministic `netlang.lock.v1` üretir ve CLI bunu artifact olarak yazar. Çok satırlı `.control`/`.include` payload'ları dahil injection corpus'u error sonrası backend üretmediğini doğrular.

### 3.7 — Engineering measurements ve ürün benchmark'ları

- [x] Node voltage, branch/device current ve instantaneous/average/RMS power için typed measurement primitive'leri tanımla.
- [x] `min`, `max`, absolute `peak`, `average`, `rms`, frequency/phase ve zaman penceresi semantiklerini birimlerle birlikte kesinleştir.
- [x] Voltage/current gain, bandwidth/cutoff, output RMS power, efficiency, THD ve component dissipation için derived measurement modeli oluştur.
- [x] Derived measurement'ların assertion içinde güvenle kullanılmasını sağla; eksik veya desteklenmeyen veri typed ERROR üretsin.
- [x] Device voltage/current/power limitlerini simulation sonucu üzerinden sınayacak assertion'ları ekle.
- [x] Her mühendislik metriğini formül, sign convention, analysis gereksinimi ve birimiyle dokümante et.
- [x] Canonical RC filtre benchmark'ı: cutoff ve AC response.
- [x] Canonical gain-stage benchmark'ı: bias, gain, bandwidth ve clipping davranışı.
- [x] Canonical çok katlı power-amplifier benchmark'ı: 8 Ω yükte hedef output power, gain, distortion ve component dissipation koşulları.
- [x] En az bir benchmark'ta dış AI ajanının başarısız tasarım adayını structured sonuçla revize edip hedefleri karşılayan adaya ulaşmasını E2E doğrula.

**3.7 kapanış kanıtı (2026-08-12):** `netlang.measurement.v1`, `V(net|device)`, `I(device)` ve `P(device)` primitive'lerini; typed reduction/time-window semantiğini ve gain, bandwidth/cutoff, frequency, phase, output RMS power, efficiency, THD, clipping ve dissipation metric'lerini dataset-first değerlendirir. Eksik/uygunsuz veri message-bearing `ERROR` olur. `docs/engineering_measurements.md` formül, birim, sign convention ve analysis gereksinimlerini kaydeder. Gerçek Ngspice benchmark suite'i RC filtresinde yaklaşık 1 kHz cutoff; gain stage'de yaklaşık 10 gain ve 100 kHz bandwidth; dört katlı amplifikatörde 8 Ω üzerinde yaklaşık 1.95 W, 56.5 gain, %2.23 THD ile clipping/dissipation/device-stress koşullarını doğrular. Real-agent E2E testi 10 kHz'e kayan hatalı RC adayının versioned JSON `FAIL` sonucunu okuyup kondansatörü revize ederek beş assertion'ın tamamını `PASS` yapar. Compile IR genişlediği için rapor `netlang.compile.v2`'ye yükseltildi; CLI `domain_versions` içinde measurement sürümünü de ilan eder.

### Faz 3 kabul kriterleri

- [x] OP, transient ve AC için en az birer gerçek Ngspice integration fixture'ı geçer. _(3.2 real Ngspice fixture matrisi.)_
- [x] Simulation result'ları typed ve serialize edilebilirdir. _(`netlang.simulation.v1` + serde contract testleri.)_
- [x] Assertion sonuçları PASS/FAIL/ERROR ayrımını doğru yapar. _(3.3 typed status ve regression suite.)_
- [x] Paralel iki simulation dosya çakışması yaşamaz. _(3.1 unique run-directory integration testi.)_
- [x] CLI human/JSON ve exit-code contract testleri geçer. _(3.4–3.5 cross-platform contract suite.)_
- [x] Dış AI ajanı human terminal metni parse etmeden versioned sözleşmeyle tasarım döngüsü kurabilir. _(3.5 stdin agent-revision E2E.)_
- [x] User-defined ve packaged model/subcircuit çözümlemesi reproducible, provenance bilgili ve fail-closed çalışır. _(3.6 typed registry, SHA-256 manifest ve lock contract.)_
- [x] RC filtre, gain stage ve power-amplifier benchmark'ları tanımlı mühendislik hedeflerini gerçek Ngspice sonuçlarıyla doğrular. _(`engineering_benchmarks` gerçek Ngspice suite'i.)_
- [x] Faz 2.5 kalite kapıları geçmeye devam eder. _(111 Rust testi; fmt, Clippy `-D warnings`, release, WASM, audit=0, Web lint ve production build canonical `scripts/verify.ps1` ile geçti.)_

**Faz 3 kapanışı (2026-08-12):** 3.1–3.7 alt görevleri ve bütün kabul kriterleri doğrulandı. Faz 3, prototype `.meas` hattı yerine versioned simulation/measurement/assertion domain'leri, side-effect-free agent CLI, reproducible model registry ve gerçek devre benchmark'larıyla kapanmıştır. Sonraki çalışma Faz 4 audit'i ve sıralamasıdır; Faz 3'e ait açık checkbox yoktur.

### Faz 3'ten ertelenen işler

- `ERTELENDİ` Datasheet voltage/current limit ERC: Component Knowledge Base gerektirir.
- `ERTELENDİ` HIGH/MEDIUM/LOW confidence skoru: önce ölçülebilir verification report tanımlanmalıdır.
- `ERTELENDİ` Layout round-trip connectivity: layout/export milestone'una taşınmıştır.

---

## 7. Faz 4 — Profesyonel Şema, Web Hub ve Yayın

### Hedef

Faz 3'te doğrulanan Core/CLI motorunu, hesap veya yerel kurulum gerektirmeyen zero-friction Web Hub ve güvenilir EDA/görsel çıktılarla bütünleşik ilk public ürüne dönüştürmek.

Faz 4 subsystem'leri birbirinden kopuk biçimde sonuna kadar geliştirmez. Önce küçük fakat gerçek bir devreyi editor → compile/ERC → şema → simulation → grafik/assertion → export zincirinden geçiren dikey dilim kurulur; aynı ürün yolu daha sonra gain-stage ve power-amplifier benchmark'larına genişletilir.

### Faz 4 kapsam sınırı

**Bu fazda zorunlu:** doğrulanabilir profesyonel şema, browser simulation, interaktif sonuçlar, canonical benchmark parity'si, görsel/makine-okunabilir/düzenlenebilir export ailesi, paylaşım, cross-platform CLI paketleri, public dokümantasyon ve production yayın.

**Bu fazda ertelendi:** Web Hub içine gömülü doğal-dil AI tasarım/chat arayüzü, yönetilen AI sağlayıcısı veya kullanıcı API-key akışı, hesaplar/cloud project storage, ekip özellikleri, otomatik optimization/design-space exploration ve geniş üretici model registry'si. Web AI daha sonra aynı provider-independent Core/CLI tool sözleşmelerinin ince consumer'ı olarak eklenebilir; Faz 4 UI'sı devre semantiğini kendi içinde yeniden uygulayarak bu yolu kapatmamalıdır.

### Faz 4 başlangıç audit'i — 2026-08-13

Mevcut Web/layout kodu tamamen boş değildir ve kanıt görmeden silinmeyecektir:

1. `webapp/src/App.tsx`, Monaco editor, WASM compile/ERC, deneysel SVG şema, SPICE görünümü ve KiCad/SPICE indirme akışı sunar.
2. Web tek büyük React component'i içinde UI state, symbol çizimleri ve download davranışını birlikte taşır; sürdürülebilir ürün mimarisi değildir fakat çalışan davranışlar characterization kaynağıdır.
3. Web `netlang.compile.v1` beklerken Core `netlang.compile.v2` üretir. Production TypeScript/Vite build'i gerçek browser/WASM çağrısını çalıştırmadığı için bu schema drift'ini yakalamamıştır.
4. `LayoutResult`, component koordinatları ve `net_id + polyline points` taşır. Açık pin endpoint'i, junction, geometry crossing, net label ve connectivity proof yoktur.
5. Chain-based layout basit rail/chain devrelerinde başlangıç değeri sağlar; feedback loop, bridge, yüksek fan-out, çoklu source ve karmaşık analog topolojilerde kalite garantisi yoktur.
6. React renderer'daki symbol geometrisi Core component kataloğuyla typed/sürümlü bir symbol sözleşmesi paylaşmaz.
7. Browser simulation ve grafik sonucu yoktur; Web şu anda yalnız compile-time WASM playground'udur.

**Yeniden yazım kararı:** Web shell, renderer, Schematic IR ve layout algoritması tek paket olarak “koru” veya “sil” kararı almaz. 4.0 characterization sonrasında her katman için ayrı `koru`, `refactor et` veya `değiştir` kararı ve gerekçesi kaydedilir. Canonical Core sözleşmeleri, fixture'lar ve doğrulanmış kullanıcı davranışları her durumda korunur.

---

### 4.0 — Ürün sözleşmesi, characterization ve teknik kararlar

- [x] Mevcut Web Hub için gerçek browser smoke testi kur; WASM init + canonical source compile akışının schema drift'ini yakaladığını göster. _Kanıt: `webapp/tests/e2e/browser-smoke.spec.ts`; gerçek Chromium'da WASM init, canonical compile, Monaco, SPICE ve şema kontrolü._
- [x] Web'in hardcoded compile schema'sını Core tarafından ilan edilen güncel sözleşmeyle eşitle; bilinmeyen sürümü fail-closed tut. _Kanıt: WASM `compile_schema_version()` export'u; Web bilinmeyen raporu reddetmeye devam eder._
- [x] Mevcut şema çıktısını şu corpus üzerinde fixture/screenshot ve connectivity baseline ile karakterize et. _Kanıt: `layout_characterization` altı fixture'da component ve bağlı-net coverage'ını denetler; browser smoke gerçek renderer görünürlüğünü kontrol eder:_
  - minimal source/resistor,
  - RC filtre,
  - Wheatstone bridge,
  - op-amp feedback gain stage,
  - yüksek fan-out,
  - çok katlı power amplifier.
- [x] Web shell, SVG renderer, Schematic IR ve layout algoritması için ayrı reuse/refactor/rewrite karar kaydı oluştur. _Kanıt: `docs/decisions/0001-phase-4-schematic-and-web-architecture.md`._
- [x] İlk public sürümün supported-domain matrix'ini dondur: desteklenen component, model, analysis, measurement ve bilinçli fiziksel sınırlar. _Kanıt: `docs/supported_domain.md`._
- [x] İlk dikey ürün kabul senaryosunu RC filtre; ana vizyon/eval senaryosunu 8 Ω power amplifier olarak kesinleştir. _ADR 0001 ve supported-domain matrix içinde kaydedildi._
- [x] Browser simulation için Ngspice WASM/Web Worker ile service-backed adaptörü lisans, artifact boyutu, startup, runtime, cancellation, model desteği, offline/privacy ve deployment açısından ölç; kararı ADR olarak kaydet. _Kanıt: ADR 0002; exact `eecircuit-engine` 1.7.0 paketi 40,693,211 byte unpacked / 20.4 MB ESM olarak ölçüldü ve Worker yolu seçildi._
- [x] Schematic IR ile render/export katmanlarının sahiplik ve sürüm sınırını architecture dokümanına işle. _Circuit IR → Schematic IR → exporter sahipliği ve Web sınırı `docs/architecture.md` bölüm 7'de._
- [x] Faz 4 sırasının mevcut audit bulgularına göre uygulanabilirliğini tekrar kontrol et; kanıtsız büyük yeniden yazıma başlama. _Karar: compile/browser entegrasyonunu önce characterization ile düzelt; ardından canonical Schematic IR, worker runtime ve dikey Web sırasını koru._

**4.0 kabul kriteri:** Mevcut davranış testle görünürdür; browser simulation ve schematic architecture kararları yazılıdır; hangi katmanın neden korunacağı veya değiştirileceği belirsiz değildir.

---

### 4.1 — Canonical ve doğrulanabilir Schematic IR

- [x] Versioned, serialize edilebilir ve collection-order'dan bağımsız `netlang.schematic.v1` sözleşmesini tanımla. _Kanıt: `core/src/schematic.rs`; compile contract `netlang.compile.v3`._
- [x] Component instance, symbol kind/variant, value/model label, orientation ve canonical pin anchor'larını açık tiplerle taşı. _`SchematicComponent` ve `PinAnchor`._
- [x] Wire segment, pin endpoint, named net, junction ve bağlanmayan geometry crossing kavramlarını ayrı temsil et. _`SchematicWire`, `WireEndpoint`, `SchematicNet`, `Junction`, `Crossing`._
- [x] GND, supply ve net-label kullanımını uzun rail/karmaşık kablo kalabalığını azaltan semantik öğeler yap. _Ground/supply ve explicit yüksek-fan-out netler typed label olur._
- [x] Layout sonucundan canonical `NetlistGraph` connectivity'sini yeniden kurup net/pin eşdeğerliğini fail-closed doğrula. _`ConnectivityReport`; mismatch `NL-L001` ile backend'i durdurur._
- [x] Aynı Circuit IR'nin tekrarlar ve declaration/connection sırası değişimlerinde byte-stable Schematic IR ürettiğini test et. _`schematic_json_and_svg_are_byte_stable_across_repeated_compiles`, `declaration_and_connection_order_do_not_change_schematic`._
- [x] Symbol tanımlarını Core component kataloğuyla tek kaynaktan eşle; Web'in ayrı ve drift edebilen pin geometrisi tanımlamasını engelle. _`CatalogSymbol`/`PinSide`; Web yalnız Core SVG'yi gösterir._
- [x] Mevcut chain heuristic'ini baseline olarak tutup feedback/bridge/fan-out/power-amplifier corpus'unda iyileştir veya daha uygun algoritmayla değiştir. _Legacy korunup canonical yol deterministic layered placement + orthogonal cost router ile değiştirildi._
- [x] Wire–symbol, label–symbol, symbol–symbol collision ve gereksiz crossing/bend ölçütleri için otomatik kalite raporu oluştur. _`QualityReport`; altı-devre corpus kapısı._
- [x] Connectivity golden ve visual golden corpus'unu Windows/Linux'ta deterministik hale getir. _SVG SHA-256 golden testi platform-bağımsızdır; Playwright altı devreyi gerçek Chromium'da render eder ve isteğe bağlı screenshot üretir._

**4.1 kabul kriteri:** Şema yalnız güzel görünen bir polyline koleksiyonu değildir; canonical circuit ile bağlantısal eşdeğerliği otomatik kanıtlanır ve karmaşık benchmark corpus'unda tanımlı okunabilirlik kapılarını geçer.

---

### 4.2 — Browser simulation runtime ve ortak adaptör

- [x] 4.0 ADR'ında seçilen browser runtime yolunu `SimulationRunner` domain sınırını koruyarak uygula. _Kanıt: WASM `prepare_browser_simulation`/`evaluate_browser_simulation` köprüleri ve UI'dan bağımsız `BrowserSimulationRunner`._
- [x] Simulator işini Web Worker veya eşdeğer izole runtime'da çalıştır; ana thread üzerinde blocking process/parsing yapma. _Kanıt: dynamic-import kullanan dedicated module Worker `simulation.worker.ts`; gerçek Chromium testi._
- [x] OP, transient, AC ve DC sweep dataset'lerini native `netlang.simulation.v1` semantiğine dönüştür. _Kanıt: dört analizi birlikte çalıştıran `browser_analysis_matrix.nl` ve E2E dataset-kind matrisi._
- [x] `netlang.measurement.v1` ve `netlang.assertion.v1` sonuçlarını browser'da Core ile aynı şekilde üret. _Kanıt: simulator adapter yalnız typed dataset üretir; WASM Core aynı `evaluate_assertions` fonksiyonunu çağırır._
- [x] Timeout, cancellation, progress, worker crash/restart ve stale-result suppression davranışlarını tanımla. _Kanıt: `browserRunner.test.ts` dört lifecycle testi; cancellation sonrası gerçek Worker restart E2E testi._
- [x] Raw log ve büyük dataset'leri varsayılan UI state/JSON akışına gereksiz kopyalamadan opt-in debug olarak tut. _Worker request `includeRawLog=false` varsayılanıdır; Core değerlendirmesine tek typed sonuç aktarılır._
- [x] Native/browser simulator sürümü ve sayısal tolerans farklarını structured provenance içinde görünür yap. _`simulator.executable/version`, runtime manifest ve parity toleransı._
- [x] RC fixture'ında native ve browser compile/SPICE/dataset/measurement/assertion parity testini geçir. _Kanıt: `simulation-parity.spec.ts`, gerçek native CLI ve Chromium sonuçlarını assertion bazında karşılaştırır; beş karar PASS._
- [x] Runtime ve model artifact'leri için lisans notice, integrity/hash ve cache/update politikasını uygula. _Kanıt: exact lock, `runtime-manifest.json`, full license bundle, `verify-runtime.mjs` ve ADR 0002 cache/update politikası._

**4.2 kabul kriteri:** RC filtre hesap veya kurulum olmadan browser'da simüle edilir; UI responsive kalır; native/browser farkı gizlenmez ve tanımlı tolerans içinde aynı mühendislik kararını verir.

---

### 4.3 — İlk zero-friction dikey Web Hub dilimi

- [x] Mevcut Web shell'i 4.0 kararına göre modülerleştir veya değiştir; editor, diagnostics, schematic, results ve export ayrı sorumluluklar taşısın. _Kanıt: workspace hook'u ve `EditorPanel`/`SchematicPanel`/`ResultsPanel`/`ArtifactBar` ayrımı._
- [x] İlk açılışta çalışan RC filtre örneği ve tek belirgin `Run` akışı sun. _RC fixture varsayılandır; simulation yalnız Results header'daki Run ile başlar._
- [x] CodePen sadeliğinde editor + diagnostics + şema + sonuç panelini responsive tek çalışma alanında birleştir. _Desktop iki sütunlu çalışma alanı, 900 px altında tek sütun._
- [x] Monaco NetLang syntax highlighting, autocomplete ve hover component/model bilgisini ekle. _`monaco.ts` Monarch grammar, completion ve hover provider'ları._
- [x] Parser/semantic/ERC diagnostic source span'lerini editörde inline göster; panel ile source konumu arasında navigasyon sağla. _Parser exact, Core semantic/ERC source annotation; Monaco markers ve tıklanabilir diagnostic satırları._
- [x] Debounced live compile ile bilinçli simulation çalıştırmasını ayır; pahalı simulation her keystroke'ta tetiklenmesin. _250 ms compile debounce; source değişimi eski simulation sonucunu temizler._
- [x] OP değer görünümü, transient plot, AC magnitude/phase (Bode) ve DC sweep plot bileşenlerini ortak typed dataset modelinden üret. _Dört dataset sekmesi `simulation-parity.spec.ts` ile gerçek browser'da doğrulanır._
- [x] Signal seçimi, engineering unit/prefix, cursor değeri, zoom/pan ve okunabilir empty/error/loading/cancelled state'lerini ekle. _Plot signal seçici/cursor/wheel zoom; schematic pointer+keyboard pan/zoom; lifecycle state'leri._
- [x] Assertion PASS/FAIL/ERROR/SKIPPED sonuçlarını ve threshold overlay'lerini ilgili grafik/sinyalle ilişkilendir. _Assertion chip'leri ve primitive metric threshold çizgisi; transient matrix E2E kapısı._
- [x] Klavye erişilebilirliği, temel mobil/tablet davranışı ve koyu/açık tema kontrastını doğrula. _Semantic region/label/focus kontrolleri, 390×844 E2E ve theme toggle._
- [x] Browser E2E testinde kaynak düzenleme → diagnostic → düzeltme → simulation → assertion → şema güncelleme döngüsünü geçir. _Kanıt: `web-workspace.spec.ts`._

**4.3 kabul kriteri:** Yeni kullanıcı RC filtrenin kodunu değiştirip cutoff sonucunu ve şemasını aynı çalışma alanında görebilir; hiçbir adım terminal, hesap veya yerel kurulum istemez.

---

### 4.4 — Gerçek devre parity'si, model ürünü ve ölçüm sertleştirmesi

- [x] Gain-stage ve dört katlı power-amplifier fixture'larını Web örnek seçicisine ekle. _RC, Gain Stage ve Power Amplifier aynı source-of-truth fixture'lardan yüklenir._
- [x] RC, gain-stage ve power-amplifier için CLI/Web compile, canonical SPICE, model manifest/lock, measurement ve assertion parity matrisi oluştur. _Kanıt: `simulation-parity.spec.ts` ve `benchmark-parity.spec.ts`; üç benchmark'ın native/browser kararları PASS._
- [x] Native ve browser build'lerinde aynı builtin/package modelin byte-for-byte aynı canonical directive/netlist'e ulaştığını test et. _Benchmark builtins ve `web-model-security.spec.ts` exact `netlang_analog@1.0.0` package'ı native artifact ile byte karşılaştırır._
- [x] İlk yayın benchmark'larının ihtiyaç duyduğu lisansı doğrulanmış başlangıç model paketini tamamla; geniş üretici registry'sini Faz 5+'ta tut. _Apache-2.0 `netlang_analog@1.0.0`, NLANG op-amp ve simetrik `NLANG_POWER_NPN/PNP_V1`; power benchmark legacy-provenance transistor modellerinden çıkarıldı._
- [x] Web'de model seçimi, provenance/lisans/sürüm/simulator bilgisini görüntüleme ve yalnız typed declaration/package import akışı sun. _Source editor typed seçim yüzeyidir; Models popover resolved manifest'i ve provenance alanlarını gösterir._
- [x] User model içeriğinin Web'de de raw SPICE/control sınırını atlayamadığını security corpus'uyla test et. _`web-model-security.spec.ts` injection'ı `NL-C010` ile durdurur, SPICE/Run üretmez._
- [x] Derived output power, efficiency ve dissipation metric'lerine açık steady-state zaman penceresi desteği ekle. _Ortak `start,stop` sözleşmesi, adaptive timestep için time-weighted RMS/average ve power benchmark 2–10 ms penceresi._
- [x] THD'yi explicit fundamental, ölçüm penceresi ve spectral leakage/window politikasıyla ürün seviyesinde kesinleştir. _`thd(signal,fundamental,start,stop,hann)`; deterministic resampling, Hann projection, H2–H5, sample/period/Nyquist fail-closed kapıları._
- [x] Low-pass dışı cevaplar için cutoff/bandwidth semantiğini açık metric veya fail-closed sınırla tanımla. _İlk AC noktası maksimumun %1'i içinde değilse ERROR; `bandwidth_fails_closed_for_non_low_pass_response`._
- [x] `Vce/Vbe/Vds/Vgs` gibi açık terminal-pair stress ölçümünü typed primitive olarak tasarla ve benchmark'la doğrula. _`V(component.pin,component.pin)` shared catalog pin doğrulaması; power benchmark `V(QN.c,QN.e)`._
- [x] Power-amplifier Web sonucunda 8 Ω output power, gain, THD, clipping, device stress ve dissipation koşullarını göster. _Ayrıca dual-supply steady-state efficiency; 12 assertion gerçek Chromium'da PASS._

**4.4 kabul kriteri:** Üç canonical benchmark native ve browser'da aynı PASS/FAIL mühendislik kararını verir; model ve ölçüm varsayımları kullanıcıdan gizlenmez.

---

### 4.5 — Profesyonel render ve EDA export

- [x] Core içinde versioned exporter adapter/capability sözleşmesi tanımla; Web, CLI veya her exporter kendi devre/layout semantiğini yeniden kurmasın. _`netlang.export.v1`; `exporter.rs` yalnız verified Schematic IR/typed IR artefaktlarını dağıtır._
- [x] İlk yayın format matrisini kullanım amacı, format açıklığı, lisans, bağlantı fidelity'si, hedef uygulama doğrulanabilirliği ve bakım maliyetiyle kaydet. _`docs/export_formats.md` kaynaklı karar matrisi._
- [x] İlk sınıf görsel çıktılar olarak SVG, PNG ve PDF'i yalnız canonical Schematic IR üzerinden üret. _PNG resvg, PDF svg2pdf; ikisi canonical SVG projection'ını kullanır._
- [x] Makine-okunabilir/değişimsiz çıktılar olarak canonical SPICE ve versioned Schematic IR JSON'u CLI/Web artifact'i yap. _Web/native exact-byte E2E parity._
- [x] İlk sınıf düzenlenebilir EDA çıktıları olarak KiCad schematic ve LTspice schematic üret. _Portable embedded KiCad symbols ve gerçek bundled-symbol LTspice ASC._
- [x] Qucs-S, CircuitJS, EasyEDA/EDIF ve güncel kullanım araştırmasında değerli bulunan diğer açık/erişilebilir hedefleri capability matrisiyle değerlendir; doğrulanamayan formatı yalnız dosya üretiyor diye desteklenmiş sayma. _İlk yayın dışında bırakma gerekçeleri ve resmi format kaynakları kaydedildi._
- [x] Aynı sembol/geometri/font ölçüm sözleşmesini browser ve native render'da paylaş. _Tek Rust/WASM exporter ve repo-içi OFL Roboto Mono._
- [x] SVG export'ta semantic text, deterministic viewBox ve theme-independent okunabilir stil üret. _Boyut/viewBox açık; white/transparent projection; golden hash corpus._
- [x] PNG export için açık çözünürlük/scale/background seçenekleri ve görsel parity testi ekle. _0.25–8 scale, white/transparent; browser/native byte equality._
- [x] PDF export'ta sayfa boyutu, orientation, margin, vector text ve multi-page politikasını tanımla. _Tek sayfa content media-box, doğal orientation, Schematic IR margin, deterministic vector glyph; parçalama yok._
- [x] `netlang render circuit.nl -o circuit.svg|png|pdf` komutunu güvenli overwrite ve structured diagnostic sözleşmesiyle uygula. _Extension allowlist, source overwrite koruması, `--force`, `NL-X/NL-I`._
- [x] `netlang export circuit.nl --target <target> --output <path>` komutuyla bütün düzenlenebilir/makine-okunabilir hedefleri aynı structured artifact sözleşmesinden sun. _Global JSON `--format` ile çakışmaması için hedef bayrağı bilinçli olarak `--target`._
- [x] CLI JSON'da exporter schema/version, artifact path/hash, connectivity verification ve warning/loss report alanlarını taşı. _`JsonArtifact` additive contract._
- [x] KiCad schematic export'ta symbol, pin, wire, junction, label ve model/value alanlarını connectivity fixture'larıyla doğrula. _RC/gain/power Core corpus ve KiCad XML netlist references._
- [x] LTspice schematic export adaptörü ve aynı bağlantı fixture'larını oluştur. _Gerçek `.asy` pin koordinatlarına remap, model/analysis directive ve üç benchmark corpus._
- [x] KiCad/LTspice dosyalarının desteklenen hedef uygulama sürümlerinde gerçekten açıldığını release smoke testine ekle. _`verify-eda-exports.ps1`: KiCad 10.0.0 ve LTspice 24.1.9 üç benchmark PASS._
- [x] Desteklenen her Core/CLI export'unu Web Hub'da aynı capability/loss bilgisiyle tek ve anlaşılır export alanında sun. _Yedi Core-advertised format, capability details ve per-artifact status._
- [x] Unsupported exporter özelliğini sessiz veri kaybı yerine structured fail-closed diagnostic yap. _`NL-X003/NL-X013` unverified connectivity ve unsupported symbol tests._

**4.5 kabul kriteri:** Her ilan edilen format Core ve CLI'dan üretilebilir; Web aynı motorun ince arayüzüdür. Görsel çıktılar okunabilir, makine çıktıları sürümlü/deterministik, düzenlenebilir EDA çıktıları canonical graph ile bağlantısal olarak eşdeğer ve hedef uygulamada doğrulanmış biçimde açılır. Lossy dönüşüm sessiz yapılmaz.

---

### 4.6 — Paylaşım, ürün anlatısı ve bağımsız agent kanıtı

- [ ] Versioned ve boyut-limitli client-side compressed circuit URL formatı tanımla.
- [ ] URL'den güvenli decode → migrate → compile akışını malformed/decompression-bomb fixture'larıyla fail-closed doğrula.
- [ ] Paylaşılan URL'nin source ve gerekli package/version bilgisini schema kaybı olmadan round-trip ettiğini test et.
- [ ] İlk açılış örnek seçicisini RC, gain-stage ve power-amplifier açıklamalarıyla tamamla.
- [ ] NetLang language reference, simulation/assertion/measurement reference, supported-domain matrix, tutorial, cookbook ve troubleshooting dokümanlarını tamamla.
- [ ] “Neden NetLang?” anlatısını ham SPICE, geleneksel simulator ve code-based circuit araçlarından ölçülebilir farklarla açıkla; rakip iddialarını yayın öncesinde güncel kaynaklarla doğrula.
- [ ] Dış bir LLM'nin doğal dilde 8 Ω power-amplifier gereksiniminden başlayıp CLI structured feedback ile devreyi revize etmesini versioned, tekrar çalıştırılabilir agent eval olarak kaydet.
- [ ] Agent eval'de model/sürüm, prompt, tool çağrıları, iterasyonlar, son source ve assertion raporunu provenance ile sakla; nondeterministik canlı LLM çağrısını canonical CI kapısı yapma.
- [ ] Aynı son source'u Web Hub paylaşım URL'sinde açıp şema, grafik, assertion ve export akışını ürün demosu olarak doğrula.
- [ ] Web içinde AI chat/tasarım arayüzü olmamasını eksiklik gibi gizleme; ilk yayında AI yüzeyinin CLI/tool contract, insan yüzeyinin Web Hub olduğunu açık anlat.

**4.6 kabul kriteri:** İnsan ürünü ve agent ürünü aynı Core üzerinde birleşir; doğal-dil agent demosu gerçek simülasyonla kanıtlıdır fakat Web Hub herhangi bir AI sağlayıcısına bağımlı değildir.

---

### 4.7 — Cross-platform paketleme ve public yayın kapısı

- [ ] Windows x86-64, Linux x86-64 ve seçilen macOS mimarileri için CLI release artifact matrisi tanımla.
- [ ] Her artifact için simulator paketleme/keşif, executable provenance, lisans notice, checksum ve version probe yolunu tamamla.
- [ ] Temiz Windows/Linux/macOS ortamında kurulum → ilk compile → ilk gerçek simulation → assertion smoke testini otomatikleştir.
- [ ] Web Hub production deployment, cache headers, WASM/worker MIME, CSP, error telemetry sınırı ve rollback prosedürünü doğrula.
- [ ] Canonical Rust/WASM/Web kapısına gerçek-browser E2E, schematic visual/connectivity, benchmark parity ve release artifact testlerini ekle.
- [ ] Güvenlik, dependency/license ve generated-artifact audit'lerini release kapısı yap.
- [ ] README'ye canlı Web Hub bağlantısı, doğrulanmış kurulum yolları, ürün ekran görüntüleri ve destek sınırlarını ekle.
- [ ] Public release tag/changelog/migration notu üret ve repository/Web Hub public görünürlüğünü yalnız tüm kabul kriterlerinden sonra aç.

### Faz 4 ve ilk public yayın kabul kriterleri

- [ ] Kullanıcı hesap veya yerel kurulum olmadan Web Hub'ı açıp canonical bir devreyi compile, simulate, measure, inspect ve export edebilir.
- [ ] Web ve CLI aynı Core semantiğini ve versioned compile/schematic/simulation/measurement/assertion sözleşmelerini kullanır.
- [ ] Şema canonical graph connectivity kontrolünden geçer; karmaşık power-amplifier topolojisinde tanımlı readability/collision kapılarını sağlar.
- [ ] SVG/PNG/PDF ve Schematic IR JSON indirilebilir; KiCad/LTspice ile ilan edilen diğer EDA fixture'ları hedef uygulamalarda bağlantı kaybı olmadan açılır.
- [ ] Web'de sunulan her export aynı source için CLI'dan da alınabilir ve aynı exporter schema/capability sözleşmesine bağlıdır.
- [ ] OP, transient, AC ve DC sweep sonuçları Web'de interaktif olarak görüntülenir; assertions ilgili signal/threshold ile ilişkilidir.
- [ ] User-defined ve packaged modeller native/Web yüzeylerinde reproducible, provenance bilgili ve injection-safe çalışır.
- [ ] RC filtre, gain-stage ve power-amplifier benchmark'ları CLI ve Web Hub'da aynı mühendislik kararlarını verir.
- [ ] Dış AI agent doğal dil gereksiniminden başlayarak human terminal metni parse etmeden gerçek power-amplifier hedeflerine ulaşabildiğini versioned eval ile gösterir.
- [ ] Paylaşılabilir URL devreyi ve gerekli version/package bilgisini schema kaybı olmadan round-trip eder.
- [ ] Desteklenen platformlarda CLI kurulumu ve ilk simülasyon temiz makine release smoke testinden geçer.
- [ ] Yeni kullanıcı yalnız public dokümanlarla bir devreyi tanımlayıp ölçebilir, assertion ekleyebilir ve hedef formatlarda export edebilir.
- [ ] Canonical kalite kapısı, browser E2E, schematic connectivity/visual corpus, benchmark parity ve release artifact doğrulamaları geçer.
- [ ] Bütün kabul kriterleri tamamlandıktan sonra repository ve Web Hub public yayınlanır.

---

## 8. Faz 5+ — Uzun Vadeli Vizyon

- Web Hub içinde provider-independent doğal-dil AI tasarım/chat arayüzü
- Bring-your-own-provider/API-key ve gerekirse yönetilen AI servis seçeneği
- Hesaplar, cloud project storage, ekip ve paylaşım yetkileri
- Component Knowledge Base ve datasheet kuralları
- Geniş üretici SPICE model registry'si ve otomatik güncelleme/provenance akışı
- Requirements/constraint schema ve agent-driven design loop araçları
- Parametric sweep, optimization ve design-space exploration
- Stability analizi ve ileri RF/noise/Monte Carlo ölçümleri
- Datasheet tabanlı voltage/current/thermal kontroller
- Ölçülebilir Verification Report; gerekirse sonrasında calibrated confidence
- Geniş reusable subcircuit/component library
- VS Code extension
- PCB export, footprint mapping ve BOM
- Diğer EDA schematic/netlist export adapter'ları
- Advanced layout: layered/Sugiyama ve hypergraph
- Multi-ground: AGND, DGND, chassis
- Alternative simulator backend'leri: Xyce/LTspice adapter'ları
- Cloud simulation ve ekip/CI dashboard'ları

---

## 9. Hedef Pipeline

```text
NetLang source
    │
    ▼
Parser ─────────────► NL-Pxxx diagnostics
    │
    ▼
AST + source spans
    │
    ▼
Semantic analysis / typed Circuit IR ─► NL-Cxxx diagnostics
    │
    ├────────► Graph + structural ERC ─► NL-Exxx diagnostics
    ├────────► Deterministic SPICE
    ├────────► Layout IR
    └────────► Structured JSON/WASM
                    │
                    ▼
              Simulation backend
                    │
                    ├────► NL-Sxxx diagnostics
                    └────► NL-Txxx assertion results
```

---

## 10. Çalışma ve Güncelleme Protokolü

Her geliştirme oturumunda:

1. `docs/ROADMAP.md` içinden aktif milestone ve ilk açık görevi kontrol et.
2. İlgili mimari kuralı `docs/architecture.md` içinde doğrula.
3. Davranış değişecekse önce characterization/regression testi ekle.
4. Kodu uygula.
5. İlgili kalite kapılarını çalıştır.
6. Yalnızca kanıtlanan checkbox'ları `[x]` yap.
7. Gerekirse karar veya kapsam değişikliğini roadmap'e yaz.
8. Çalışma ağacının beklenmeyen artifact ile kirlenmediğini kontrol et.

### Aktif sıradaki ilk iş

**4.0 — Ürün sözleşmesi, characterization ve teknik kararlar.**

İlk uygulama dilimi mevcut Web/WASM runtime'ını gerçek browser smoke testiyle görünür kılacak, `netlang.compile.v1`/`v2` drift'ini fail-closed düzeltecek ve minimal/RC/bridge/gain-stage/fan-out/power-amplifier corpus'unda mevcut schematic davranışını karakterize edecektir. Bu kanıttan sonra Web shell, renderer, Schematic IR ve layout algoritması için ayrı reuse/refactor/rewrite kararı verilecek; ardından browser simulation ADR'ı kapatılacaktır.
