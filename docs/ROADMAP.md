# NetLang Geliştirme Yol Haritası

> Bu doküman NetLang'in geliştirme durumu, aktif milestone'u, kabul kriterleri ve görev sırası için **tek gerçek kaynaktır (Single Source of Truth)**.
>
> Mimari kurallar için `docs/architecture.md`, kullanıcıya açık CLI sözleşmesi için `docs/cli_reference.md` kullanılır. Bu belgeler arasında çelişki varsa geliştirme durumu açısından bu roadmap esas alınır ve çelişki aktif milestone içinde düzeltilir.
>
> Son kapsamlı repo denetimi: **2026-08-08**
>
> Aktif milestone: **Faz 3 — Simülasyon ve Assertion Runtime**
>
> Sonraki milestone: **Faz 4 — Layout, Web Hub ve Agent API**

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

NetLang'in AI modelini kendi içinde barındırması zorunlu değildir. Öncelikli hedef, dışarıdaki herhangi bir yetkin AI ajanının CLI veya structured API üzerinden NetLang'i güvenilir bir **tasarım oracle'ı, simülasyon motoru ve doğrulama aracı** olarak kullanabilmesidir.

Kapsam kademeli genişler: ilk güçlü dikey analog ve karma-sinyal/SPICE tabanlı tasarımlardır; uzun vadeli mimari yalnızca eğitim devrelerine, basit örneklere veya tek bir endüstri alanına göre sınırlandırılmaz. Desteklenmeyen fiziksel alanlar ve simulator sınırları açıkça raporlanır; doğrulanmayan bir tasarım doğrulanmış gibi sunulmaz.

### Üç ana ürün yüzeyi

| Yüzey | Hedef kullanıcı | Temel çıktı |
|---|---|---|
| NetLang Core | Tüm sistem | Parser, AST, typed IR, graph, ERC, simulation, ölçüm ve layout |
| NetLang CLI/API | AI ajanları, otomasyon ve geliştiriciler | Tasarla-ölç-doğrula döngüsü için structured araç yüzeyi |
| NetLang Web Hub | Kurulumsuz ürün deneyimi isteyen herkes | Kod/tasarım → doğrulama → şema → simülasyon → export → paylaşım |

### Birinci sınıf ürün çıktıları

- Structured engineering diagnostics ve ölçümler
- Constraint/assertion PASS, FAIL, ERROR sonuçları
- Deterministik SPICE ve simulation dataset'leri
- Okunabilir ve bağlantısal olarak doğrulanmış şema
- PNG, SVG ve ileride PDF gibi görsel export'lar
- LTspice, KiCad schematic ve ileride diğer EDA formatları için düzenlenebilir export'lar
- CLI/API ve Web arasında aynı Core semantiği

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
| `cargo test --all-targets` | Geçiyor | 69 test; parser/IR/compiler/graph/ERC/SPICE/CLI/runtime contract kapsamı mevcut |
| `cargo fmt -- --check` | Geçiyor | Rust kaynakları canonical `rustfmt` biçiminde |
| `cargo clippy --all-targets -- -D warnings` | Geçiyor | Mevcut target'larda warning yok |
| `npm run build` | Geçiyor | WASM paketini sıfırdan üretip web production build'i tamamlıyor |
| `npm run lint` | Geçiyor | React hook dependency uyarısı giderildi; warning yok |
| Örnek CLI matrisi | Geçiyor | Dört geçerli example check+compile oluyor; intentionally-invalid `test_amp.nl`, `NL-E003`/exit 1 veriyor |

### 4.3 Kalan kritik açıklar

- Dağıtılan Ngspice runtime yalnız Windows x86-64 sidecar'dır; Linux/macOS paketleme tamamlanmadı. `NETLANG_NGSPICE` açık executable override'ı mevcuttur.
- Ngspice runner sabit `netlang_temp.spice` dosyasını kullanıyor; paralel çalıştırmaya uygun değil.
- Faz 3 sonuç modeli yalnızca `.meas` map'i ve string error listesi içeriyor; structured OP/transient/AC verisi yok.
- Assertion measurement eksikliği halen `NaN/Not Found` prototip davranışına dayanıyor; Faz 3'te typed ERROR sonucuna dönüşmelidir.
- Layout/KiCad çıktısı deneyseldir; connectivity round-trip, collision ve gerçek KiCad açılabilirlik fixture'ları tamamlanmadı.
- Web runtime için otomatik gerçek-browser smoke testi yok; WASM build + TypeScript/Vite build ve Rust all-output smoke testi mevcut.

### 4.4 Faz yorumu

- Faz 0: Tarihsel MVP tamamlandı.
- Faz 1: Golden/regression ve fail-closed kabul kriterleri Faz 2.5 içinde kapatıldı.
- Faz 2: Typed IR, determinism, semantic validation ve regression borçları Faz 2.5 içinde kapatıldı.
- Faz 3: Temel `.meas` assertion akışı **PROTOTİP**; ayrıntılı Faz 3 kabul kriterleri tamamlanmadı.
- Aktif çalışma: Faz 3 başlangıç audit'inde tanımlanan sırayla simulation domain modeli ve tek runner geliştirilecek.

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

- [ ] `SimulationRequest` ve `SimulationResult` domain tiplerini tanımla.
- [ ] Analysis türlerini typed enum yap: OP, transient, AC, DC sweep.
- [ ] Measurement, warning, error ve raw-log alanlarını ayır.
- [ ] Simulator process status'unu structured biçimde sakla.
- [ ] Her çalıştırma için benzersiz temp directory kullan.
- [ ] Temp cleanup ve failure artifact saklama politikasını tanımla.
- [ ] `simulate` ve `test` için tek runner kullan.
- [ ] Ngspice executable discovery ve version check'i güvenilir yap.
- [ ] Timeout/cancellation desteği ekle.

### 3.2 — Structured Ngspice sonuçları

- [ ] `.meas` parser'ını ayrı ve fixture tabanlı modül yap.
- [ ] Operating point sonuçlarını structured map'e çevir.
- [ ] Transient time-series sonuçlarını structured dataset'e çevir.
- [ ] AC complex/frequency-domain sonuçlarını structured dataset'e çevir.
- [ ] Mümkünse stdout tablo scraping yerine Ngspice raw/wrdata çıktısı kullan.
- [ ] Warning, convergence ve fatal error sınıflandırması yap.
- [ ] Locale, exponent ve line-ending fixture'ları ekle.

### 3.3 — Assertion runtime

- [ ] Her assertion'a deterministik `NL-Txxx` kodu ata.
- [ ] Sonuç durumlarını tanımla: PASS, FAIL, ERROR, SKIPPED.
- [ ] Missing measurement'ı NaN yerine açıklamalı ERROR yap.
- [ ] Absolute ve relative tolerance politikası tanımla.
- [ ] `peak` semantiğini absolute peak olarak kesinleştir.
- [ ] OP analizinde desteklenen assertion metric'lerini tanımla.
- [ ] Current direction/sign convention'ı belgele ve test et.
- [ ] Unit-aware human output üret.
- [ ] JSON sonuçlarına summary ekle.

Örnek hedef JSON:

```json
{
  "schema_version": "1",
  "tests": [
    {
      "code": "NL-T001",
      "status": "PASS",
      "metric": "max(V(out))",
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

### 3.4 — Simulation CLI/API sözleşmesi

- [ ] `netlang simulate` structured analysis sonucu döndürür.
- [ ] `netlang test` structured assertion sonucu döndürür.
- [ ] Human ve JSON modları aynı domain sonucunu render eder.
- [x] Simulation failure exit 3, assertion failure exit 4 verir. _(Faz 2.5 cross-platform fake-process contract testleri.)_
- [x] JSON stdout parse edilebilir ve logsuzdur. _(Success/failure/assertion CLI integration testleri.)_
- [ ] Simulation fixture'ları CI'da güvenilir çalışır.

### Faz 3 kabul kriterleri

- [ ] OP, transient ve AC için en az birer gerçek Ngspice integration fixture'ı geçer.
- [ ] Simulation result'ları typed ve serialize edilebilirdir.
- [ ] Assertion sonuçları PASS/FAIL/ERROR ayrımını doğru yapar.
- [ ] Paralel iki simulation dosya çakışması yaşamaz.
- [ ] CLI human/JSON ve exit-code contract testleri geçer.
- [ ] Faz 2.5 kalite kapıları geçmeye devam eder.

### Faz 3'ten ertelenen işler

- `ERTELENDİ` Datasheet voltage/current limit ERC: Component Knowledge Base gerektirir.
- `ERTELENDİ` HIGH/MEDIUM/LOW confidence skoru: önce ölçülebilir verification report tanımlanmalıdır.
- `ERTELENDİ` Layout round-trip connectivity: layout/export milestone'una taşınmıştır.

---

## 7. Faz 4 — Layout, Web Hub ve Agent API

Faz 4 ayrıntıları Faz 3 kapanışında yeniden denetlenecektir. Mevcut yön:

### 4.1 — Layout doğrulanabilirliği ve export

- [ ] Layout wire/pin veri modelini açık bağlantı semantiğiyle güçlendir.
- [ ] Geometry crossing ile electrical junction ayrımını temsil et.
- [ ] Layout round-trip connectivity kontrolü.
- [ ] Native/server-side SVG export.
- [ ] `netlang render circuit.nl -o circuit.svg`.
- [ ] KiCad export doğrulama fixture'ları.

### 4.2 — Web simulation runtime

- [ ] Ngspice WASM feasibility ve lisans/performans kararı.
- [ ] Web Worker içinde simulation.
- [ ] Cancellation ve progress callback.
- [ ] Ana thread'i bloklamayan runtime.

### 4.3 — Monaco NetLang desteği

- [ ] Syntax highlighting.
- [ ] Autocomplete.
- [ ] Inline diagnostic ve source span.
- [ ] Hover component bilgisi.

### 4.4 — Simulation grafik paneli

- [ ] Transient plot.
- [ ] AC/Bode plot.
- [ ] DC sweep plot.
- [ ] Assertion threshold overlay.

### 4.5 — Paylaşım

- [ ] Client-side compressed circuit URL.
- [ ] URL'den güvenli yükleme ve compile.
- [ ] Format/schema version migration.

### 4.6 — Agent API

- [ ] stdin/stdout JSON agent modu.
- [ ] `check`, `compile`, `simulate`, `test` komutları.
- [ ] Versioned schema.
- [ ] Idempotent ve deterministik sonuç.
- [ ] Structured suggested actions.

---

## 8. Faz 5+ — Uzun Vadeli Vizyon

- Component Knowledge Base ve datasheet kuralları
- Üretici SPICE model registry'si ve provenance
- `netlang.lock` ile reproducible model builds
- Requirements/constraint schema ve agent-driven design loop araçları
- Parametric sweep, optimization ve design-space exploration
- Output power, gain, bandwidth, efficiency, dissipation, distortion ve stability ölçümleri
- Simulation-based voltage/current/thermal kontroller
- Ölçülebilir Verification Report; gerekirse sonrasında calibrated confidence
- Subcircuit/component library
- VS Code extension
- PCB export, footprint mapping ve BOM
- LTspice ve diğer EDA schematic/netlist export adapter'ları
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

**3.1 — Simulation domain modeli ve runner.**

Faz 2.5 yerel ve remote kalite kanıtlarıyla tamamen kapandı. Faz 3 başlangıç audit'i günceldir; ilk uygulama paketi typed `SimulationRequest`/`SimulationResult`, analysis enum'u ve `simulate`/`test` tarafından paylaşılacak tek runner sınırıdır.
