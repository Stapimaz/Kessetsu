# NetLang Geliştirme Yol Haritası (ROADMAP)

> **Bu doküman, NetLang projesinin tek gerçek kaynağıdır (Single Source of Truth).**
> Tüm AI ajanları ve geliştiriciler, herhangi bir geliştirme yapmadan önce bu dokümanı okumalı ve ilgili fazın durumunu kontrol etmelidir.
>
> Son güncelleme: 2026-08-08

---

## Proje Kimliği

**NetLang**, analog ve karma-sinyal devreleri metinle tanımlayan, deterministik SPICE netlist üreten, Ngspice ile simülasyon çalıştıran, yapılandırılmış ERC/test sonuçları veren ve doğrulanmış şema oluşturan bir **devre derleyicisi ve doğrulama altyapısıdır**.

```
Compile, simulate and test circuits like software.
```

### Üç Sütun (Kapsamı daraltmıyoruz, sırayla yapıyoruz)

| Sütun | Hedef Kullanıcı | Çıktı |
|---|---|---|
| **NetLang Core** | Tüm sistem | Rust crate: parser, IR, ERC, SPICE gen, layout |
| **NetLang CLI** | AI ajanları + geliştiriciler | Tek binary: `netlang check/compile/simulate/test/render` |
| **NetLang Web Hub** | İnsanlar | Browser playground: kod → şema → simülasyon → paylaşım |

### Teknik Yığın
- **Çekirdek:** Rust (tek `netlang-core` crate, çoklu modül)
- **Parser:** PEG (`pest` crate, `netlang.pest`)
- **Simülasyon:** Ngspice (native: subprocess, web: WASM Worker — ileride)
- **Web:** React/TypeScript + Vite, WASM üzerinden `netlang-core`
- **Şema:** SVG render (React tarafında, ileride server-side SVG)

---

## Mevcut Durum (Faz 0 — Tamamlandı)

### Çalışan Özellikler
- [x] PEG grammar ile parser (`netlang.pest`, 22 satır)
- [x] AST yapısı (`ast.rs` — ComponentDecl, Connection, UseStmt, SimulateStmt)
- [x] Modül sistemi (`module`, `use`, `flatten()`)
- [x] Graph builder — flood-fill ile net ataması (`graph.rs`)
- [x] Deterministik düğüm isimlendirme (N_{İlkPin} algoritması)
- [x] GND = 0 canonicalization
- [x] SPICE netlist üretimi (`generate_spice()`)
- [x] 6 standart model otomatik injection (2N3904, 2N3906, 2N2222, 1N4148, 1N4007, IRF540)
- [x] DRC kontrolleri (duplicate, undefined, floating pin, short circuit)
- [x] DFS tabanlı layout motoru — chain-based vertical layout
- [x] Wire routing (VCC/GND rail + L-shaped signal wires)
- [x] WASM binding (`compile_netlang()`)
- [x] React webapp — Monaco editör, SVG şema render, SPICE çıktısı, KiCad export
- [x] CLI binary (`bin.rs`, `netlang-cli.rs`)
- [x] 4 örnek devre (common emitter, voltage divider, wheatstone, NC test)
- [x] NC dummy resistor injection (floating pin SPICE crash önleme)

### Desteklenen Bileşenler
| Bileşen | Keyword | Pinler | SPICE Prefix |
|---|---|---|---|
| Resistor | `resistor` | p1, p2 | R_ |
| Capacitor | `capacitor` | p1, p2 | C_ |
| Inductor | `inductor` | p1, p2 | L_ |
| Diode | `diode` | p1, p2 | D_ |
| Transistor (BJT) | `transistor` | c, b, e | Q_ |
| MOSFET | `mosfet` | d, g, s | M_ |
| Op-Amp | `opamp` | in_p, in_n, out, vcc, vee | X_ |
| Voltage Source | `source` | plus, minus | V_ |

### Bilinen Sorunlar ve Mimari Borçlar (Faz 0'dan kalan)
1. **🔴 `value: String` — Typed IR yok.** Tüm bileşen değerleri (`10k`, `2N3904`, `SINE(0 1V 1kHz)`) tek bir `String` alanında tutuluyor. Statik doğrulama, typed assertion ve güvenli netlist üretimi imkansız.
2. **🔴 Layout'ta `"Battery"` bug'ı.** `layout.rs`'de `comp_type == "Battery"` kontrolü var ama `ast.rs`'de `ComponentType::Source` → `Debug` format `"Source"` yazdırır. VCC/GND tespiti kırık olabilir.
3. **🟡 DRC terminolojisi yanlış.** Schematic seviyesinde doğru terim ERC (Electrical Rules Check). DRC, PCB physical design için kullanılır.
4. **🟡 `transistor` tipi belirsiz.** NPN/PNP ayrımı AST'de yok, sadece model adından çıkarılıyor.
5. **🟡 `current_source` yok.** Sadece `source` (voltage) destekleniyor.
6. **🟡 User-named net yok.** Netler yalnızca otomatik isimlendiriliyor (N_{İlkPin}).
7. **🟡 Assert/test sistemi yok.** `simulate` komutu analiz başlatıyor ama sonuç doğrulama mekanizması yok.
8. **🟡 CLI çıktısı yapılandırılmış değil.** JSON output yok, error code sistemi yok, source span yok.
9. **🟢 İki ayrı CLI binary var** (`bin.rs` ve `bin/netlang-cli.rs`). Birleştirilmeli.
10. **🟢 Test yetersiz.** Tek bir test (`lib.rs` L20-35). Golden test yok.

---

## Faz 1: Temel Güçlendirme (Mimari Borç Temizliği)

> **Hedef:** Typed IR katmanı ekleyerek projenin tüm ileri özelliklerini güvenli biçimde inşa edebileceği temeli kurmak.
>
> **Başarı kriteri:** `value: String` kalıntısı kalmamış olmalı. Tüm bileşen parametreleri typed. Layout Battery bug'ı düzelmiş. ERC terminolojisi yerleşmiş. En az 10 golden test yazılmış olmalı.

### 1.1 — Circuit IR Modülü (`ir.rs`) — YENİ DOSYA
- [x] `ir.rs` modülü oluştur ve `lib.rs`'e ekle
- [x] `CircuitIR` struct tanımla:
...
- [x] `IRComponent` tanımla — typed parameters:
  ```rust
  pub struct IRComponent {
      pub id: String,
      pub kind: ComponentKind,
      pub parameters: ComponentParams,
      pub model: Option<ModelRef>,
  }
  
  pub enum ComponentKind {
      Resistor, Capacitor, Inductor, Diode,
      BJT(BJTPolarity),     // NPN, PNP
      MOSFET(FETPolarity),  // NMOS, PMOS
      OpAmp,
      VoltageSource,
      CurrentSource,
  }
  
  pub enum BJTPolarity { NPN, PNP }
  pub enum FETPolarity { NMOS, PMOS }
  
  pub enum ComponentParams {
      TwoPinPassive { value: f64, unit: SIUnit },
      BJTParams { polarity: BJTPolarity },
      MOSFETParams { polarity: FETPolarity },
      OpAmpParams,
      DCSource { voltage: f64 },
      ACSource { waveform: Waveform },
  }
  
  pub enum Waveform {
      Sine { offset: f64, amplitude: f64, frequency: f64 },
      Pulse { v1: f64, v2: f64, delay: f64, rise: f64, fall: f64, width: f64, period: f64 },
      PWL { points: Vec<(f64, f64)> },
  }
  
  pub enum SIUnit { Ohm, Farad, Henry, Volt, Ampere, Hertz }
  ```
- [x] `ModelRef` struct tanımla:
  ```rust
  pub struct ModelRef {
      pub name: String,           // "2N3904"
      pub kind: ComponentKind,    // BJT(NPN)
      pub source: ModelSource,    // Builtin, UserDefined
  }
  ```
- [x] `ast_to_ir()` dönüşüm fonksiyonu yaz — AST'den IR'ye:
  - [x] SI prefix parser (`10k` → 10000.0, `100uF` → 0.0001, `1MHz` → 1000000.0)
  - [x] String value → typed params dönüşümü
  - [x] Model name → ModelRef çözümleme
  - [x] Source string parsing (`"SINE(0 1V 1kHz)"` → `Waveform::Sine`)
- [x] Tüm downstream kodları IR kullanacak şekilde güncelle:
  - [x] `graph.rs`: `generate_spice()` → IR'den SPICE üret
  - [x] `erc.rs` (eski drc.rs): IR üzerinde kontrol yap
  - [x] `layout.rs`: IR'den layout üret
  - [x] `wasm.rs`: IR'yi JSON olarak da döndür

**Kabul Kriterleri:**
- `cargo test` geçiyor
- `value: String` artık SPICE üretiminde doğrudan kullanılmıyor
- Tüm örnek devreler (`examples/*.nl`) doğru IR üretiyor
- `10k`, `2.2k`, `100uF`, `1MHz` gibi SI prefixler doğru parse ediliyor

### 1.2 — Battery Bug Fix (`layout.rs`)
- [x] `layout.rs` içindeki tüm `"Battery"` string karşılaştırmalarını `"Source"` ile değiştir
- [x] `get_through_pin()` fonksiyonundaki `"Battery"` → `"Source"` düzelt
- [x] VCC/GND tespitinin doğru çalıştığını test et

**Kabul Kriterleri:**
- `demo_circuit.nl` doğru layout üretiyor (VCC üstte, GND altta)
- `wheatstone.nl` çökmüyor

### 1.3 — DRC → ERC Yeniden Adlandırma
- [x] `drc.rs` → `erc.rs` olarak yeniden adlandır
- [x] `DrcError` → `ErcDiagnostic` olarak yeniden adlandır
- [x] Error code sistemi ekle:
  ```rust
  pub struct ErcDiagnostic {
      pub code: String,        // "NL-E001"
      pub severity: Severity,  // Error, Warning, Info
      pub message: String,
      pub component: Option<String>,
      pub pin: Option<String>,
  }
  pub enum Severity { Error, Warning, Info }
  ```
- [x] Tüm referansları güncelle (`lib.rs`, `wasm.rs`, `bin.rs`, `netlang-cli.rs`)
- [x] Mevcut 4 kontrol için error code ata:
  - `NL-E001`: Duplicate component declaration
  - `NL-E002`: Undefined component reference
  - `NL-E003`: Floating pin
  - `NL-E004`: Direct short circuit

**Kabul Kriterleri:**
- `cargo test` geçiyor
- Projede `drc` kelimesi kalmamış (WASM API hariç — geriye uyumluluk)
- Her hata mesajı `NL-EXXX` kodu içeriyor

### 1.4 — CLI Birleştirme ve JSON Output
- [x] `bin.rs` ve `bin/netlang-cli.rs`'yi tek CLI olarak birleştir → `bin/netlang.rs`
- [x] Alt komut yapısı ekle:
  ```
  netlang check <file.nl>       # Parse + ERC
  netlang compile <file.nl>     # Parse + ERC + SPICE netlist üret
  netlang simulate <file.nl>    # compile + Ngspice çalıştır
  netlang render <file.nl>      # compile + SVG şema üret (Faz 3)
  ```
- [x] `--format json` flag'i ekle — JSON diagnostic output
- [x] `--format human` (varsayılan) — insan-okunur renkli output
- [x] Exit code standardı: 0 = başarılı, 1 = ERC hatası, 2 = parse hatası, 3 = simülasyon hatası

**Kabul Kriterleri:**
- `netlang check examples/demo_circuit.nl` çalışıyor
- `netlang check examples/test_amp.nl --format json` yapılandırılmış JSON döndürüyor (floating pin hatası)
- Eski `bin.rs` ve `netlang-cli.rs` silinmiş

### 1.5 — Temel Test Altyapısı
- [ ] Parser golden testler:
  - Tüm `examples/*.nl` dosyaları için AST snapshot testleri
  - Geçersiz syntax corpus'u (en az 5 hatalı input)
  - Edge case'ler: Unicode karakter, boş dosya, sadece yorum
- [ ] Graph golden testler:
  - Pin sırasından bağımsız aynı net ID (determinism)
  - GND canonicalization
  - NC pin dummy resistor injection
- [ ] SPICE snapshot testler:
  - `demo_circuit.nl` → beklenen SPICE netlist karşılaştırma
  - `wheatstone.nl` → beklenen SPICE netlist karşılaştırma
- [ ] IR dönüşüm testleri:
  - `resistor R1 10k` → `ComponentParams::TwoPinPassive { value: 10000.0, unit: Ohm }`
  - `source Vin "SINE(0 1V 1kHz)"` → `Waveform::Sine { ... }`
  - Bilinmeyen model warning testi

**Kabul Kriterleri:**
- En az 15 test, hepsi geçiyor
- `cargo test` CI-ready

### 1.6 — Doküman Güncellemeleri
- [ ] `architecture.md` güncelle (aşağıdaki "Architecture Güncellemeleri" bölümüne bak)
- [ ] `AGENTS.md` güncelle — yeni modül isimleri ve kurallar
- [ ] Bu ROADMAP.md'deki Faz 1 checkbox'larını güncelle

**Kabul Kriterleri:**
- architecture.md, gerçek kod yapısını doğru yansıtıyor
- Terminoloji tutarlı (ERC, Source, IR)

---

## Faz 2: Dil Genişletme

> **Hedef:** NetLang'ı "başlangıç sözlüğü"nden "gerçek bir DSL"ye dönüştürmek. User-named nets, assertions, typed source expressions ve current source desteği.
>
> **Başarı kriteri:** `assert` keyword'ü çalışıyor. User-named netler SPICE çıktısında görünüyor. `current_source` destekleniyor. Grammar 40+ satır.
>
> **Ön koşul:** Faz 1 tamamen tamamlanmış olmalı.

### 2.1 — Grammar Genişletme (`netlang.pest`)
- [x] `net` keyword'ü ekle:
  ```
  net output
  connect R1.p2 output
  connect C1.p1 output
  ```
- [x] Çoklu connect syntax'ı ekle:
  ```
  connect R1.p2, C1.p1, Q1.b to output
  ```
- [x] `assert` keyword'ü ekle:
  ```
  assert max(V(out)) < 3.3V
  assert min(V(out)) > 0V
  assert peak(I(D1)) < 100mA
  ```
- [x] Typed source expression ekle:
  ```
  source Vin sine(0V, 1V, 1kHz)
  source Vdc 5V
  source Ipulse pulse(0mA, 100mA, 1ms, 10us, 10us, 500us, 1ms)
  ```
  Eski string syntax da geriye uyumlu kalsın.
- [x] `current_source` keyword'ü ekle:
  ```
  current_source I1 10mA
  current_source Iac sine(0mA, 5mA, 10kHz)
  ```
- [x] BJT alt-tip ipucu ekle (opsiyonel):
  ```
  transistor Q1 2N3904          // Model'den NPN çıkarılır
  transistor Q2 npn             // Explicit polarity, ideal model
  transistor Q3 pnp 2N3906     // Explicit polarity + model
  ```

**Kabul Kriterleri:**
- Tüm yeni syntax'lar parse ediliyor
- Eski syntax geriye uyumlu çalışıyor
- Grammar 40+ satır

### 2.2 — User-Named Nets (Graph + SPICE)
- [x] `graph.rs`'de user-named net desteği:
  - `net output` ifadesi bir net oluşturur
  - `connect X.pin output` o net'e bağlar
  - User-named net isimleri otomatik N_{...} isimlerinden önceliklidir
- [x] SPICE çıktısında user net isimleri kullanılsın:
  - `v(output)` — user-named
  - `v(N_C1_p1)` — otomatik (fallback)
- [x] Otomatik isimlendirme algoritması KORUNUR — sadece user ismi yoksa devreye girer

**Kabul Kriterleri:**
- User-named net SPICE'ta doğru isimle görünüyor
- Aynı nete hem `net output` hem N_{...} atanamıyor (user ismi kazanır)
- Mevcut örnekler kırılmadan çalışıyor

### 2.3 — Current Source Desteği
- [x] `ComponentType::CurrentSource` → `ast.rs`
- [x] `ComponentKind::CurrentSource` → `ir.rs`
- [x] Parser'da `current_source` keyword'ü
- [x] SPICE prefix: `I_`
- [x] Pinler: `plus`, `minus` (voltage source ile aynı)
- [x] ERC: source ile aynı kontroller
- [x] Layout: source ile aynı sembol (farklı render sonra)

**Kabul Kriterleri:**
- `current_source I1 10mA` çalışıyor
- SPICE çıktısında `I_I1 N_... 0 10mA` görünüyor

### 2.4 — Assertion Altyapısı (Temel)
- [x] AST'de `AssertStmt` ekle:
  ```rust
  pub struct AssertStmt {
      pub metric: String,    // "max", "min", "peak", "rms", "settle"
      pub signal: String,    // "V(out)", "I(D1)"
      pub comparator: Cmp,   // Lt, Gt, Eq, Le, Ge
      pub threshold: f64,
      pub unit: SIUnit,
  }
  ```
- [x] IR'de `Assertion` olarak temsil et
- [x] SPICE `.control` bloğuna `.meas` komutları olarak dönüştür
- [x] Simülasyon sonuç parsing'i (Faz 3'te tam çalışacak, burada altyapı)

**Kabul Kriterleri:**
- `assert max(V(out)) < 3.3V` parse ediliyor
- IR'de assertion olarak temsil ediliyor
- SPICE'ta `.meas` komutu olarak görünüyor (sonuç evalution Faz 3)

---

### Faz 3: Simülasyon & Doğrulama Altyapısı [x]
- [x] **Ngspice CLI Entegrasyonu** (`sim_result.rs` içinde subprocess olarak ngspice'ı çağır)
- [x] **Assertion Değerlendirme Motoru**
  - Ngspice stdout'tan `.meas` sonuçlarını parse et
  - Epsilon töleransıyla beklenen değer (threshold) ile ölçülen değeri karşılaştır
- [x] **Simülasyon Hata Yönetimi** (DC OP Failure, convergence hataları)
- [x] **`netlang test` CLI Komutu** eklendi (JSON format desteği ile)

**Kabul Kriterleri:**
- `netlang test circuit.nl` komutu assertion sonuçlarını PASS/FAIL olarak raporluyor. JSON formatında structured sonuç döndürüyor.

> **Ön koşul:** Faz 2 tamamen tamamlanmış olmalı.

### 3.1 — Ngspice Sonuç Parser'ı (`sim_result.rs`) — YENİ DOSYA
- [ ] Ngspice stdout'unu parse eden modül
- [ ] Operating Point sonuçları → `HashMap<String, f64>`
- [ ] Transient sonuçları → `Vec<(f64, HashMap<String, f64>)>` (zaman serileri)
- [ ] AC sonuçları → frekans domain verileri
- [ ] Convergence hatası tespiti
- [ ] Ngspice error/warning mesajları ayrıştırma

### 3.2 — Assertion Runtime
- [ ] `.meas` sonuçlarını Ngspice çıktısından çıkar
- [ ] Assertion değerlendirme motoru:
  ```
  PASS NL-T001: max(V(out)) = 3.21V < 3.3V
  FAIL NL-T002: peak(I(D1)) = 184mA, expected < 100mA
  ```
- [ ] `netlang test <file.nl>` komutu — compile + simulate + assert
- [ ] JSON test sonuç formatı:
  ```json
  {
    "tests": [
      { "code": "NL-T001", "status": "PASS", "metric": "max(V(out))", "actual": 3.21, "threshold": 3.3 },
      { "code": "NL-T002", "status": "FAIL", "metric": "peak(I(D1))", "actual": 0.184, "threshold": 0.1 }
    ],
    "summary": { "total": 2, "passed": 1, "failed": 1 }
  }
  ```

### 3.3 — Simülasyon Tabanlı ERC Kontrolleri
- [ ] Voltaj limit aşımı tespiti (model/datasheet'ten)
- [ ] Akım limit aşımı tespiti
- [ ] DC operating point başarısızlığı
- [ ] Convergence problemi raporlama
- [ ] Yeni ERC kodları: NL-S001 ~ NL-S010 (simulation-based)

### 3.4 — Confidence Report
- [ ] Her derleme sonucunda coverage raporu:
  ```
  ERC: 4/4 structural checks passed
  Simulation: completed (tran 10ms)
  Model coverage: 5/6 components have physical models
  Assertions: 3/4 passed
  Confidence: HIGH
  ```
- [ ] Confidence seviyeleri: `HIGH`, `MEDIUM`, `LOW`, `UNKNOWN`
- [ ] JSON formatında da döndür

### 3.5 — Layout Round-Trip Connectivity Check
- [ ] Layout çıktısındaki wire'lardan connectivity graph oluştur
- [ ] Orijinal netlist graph ile karşılaştır
- [ ] `✓ Schematic connectivity verified` veya hata raporu

---

## Faz 4: Web Hub & Agent API

> **Hedef:** Web arayüzünü tam işlevsel playground'a dönüştürmek. AI ajanları için structured API. URL paylaşım.
>
> **Başarı kriteri:** Web'de simülasyon çalışıyor. URL ile devre paylaşılabiliyor. JSON-RPC ile agent erişimi mümkün.
>
> **Ön koşul:** Faz 3'ün en az 3.1 ve 3.2 maddesi tamamlanmış olmalı.

### 4.1 — Web Worker WASM Simülasyonu
- [ ] Ngspice WASM derlenmesi araştır (Emscripten)
- [ ] Web Worker içinde `netlang-core.wasm` + `ngspice.wasm` çalıştır
- [ ] Ana thread donmadan simülasyon
- [ ] Simülasyon progress callback'i

### 4.2 — Monaco NetLang Syntax Desteği
- [ ] Özel language definition: keyword highlighting, autocomplete
- [ ] Inline ERC hata gösterimi (squiggly lines)
- [ ] Hover ile bileşen bilgisi

### 4.3 — Simülasyon Grafik Paneli
- [ ] Transient analiz sonuçlarını grafik olarak çiz (Chart.js veya Plotly)
- [ ] AC analiz Bode plot
- [ ] DC sweep grafik
- [ ] Assert threshold'larını grafik üzerinde çizgi olarak göster

### 4.4 — URL Paylaşım Sistemi
- [ ] Client-side compressed URL: `/#/c/<base64-compressed-code>`
  - Küçük devreler için, sunucuya veri göndermez
  - LZ-String veya benzer sıkıştırma
- [ ] URL'den kod yükleme ve otomatik derleme

### 4.5 — SVG Export
- [ ] Server-side (veya WASM-side) SVG render — React bağımlılığı olmadan
- [ ] `netlang render circuit.nl -o circuit.svg` CLI komutu
- [ ] SVG içinde net label'ları, bileşen değerleri

### 4.6 — Agent API (JSON-RPC / Structured Output)
- [ ] `netlang agent` modu — stdin'den JSON komut al, stdout'a JSON sonuç yaz
- [ ] Komutlar:
  ```json
  {"cmd": "check", "code": "resistor R1 10k\n..."}
  {"cmd": "compile", "code": "..."}
  {"cmd": "simulate", "code": "..."}
  {"cmd": "test", "code": "..."}
  ```
- [ ] Structured `suggested_actions`:
  ```json
  {
    "kind": "insert_component",
    "component": "resistor",
    "reason": "Current limiting for LED"
  }
  ```
- [ ] Idempotent komutlar — aynı input, aynı output garantisi

---

## Uzun Vadeli Vizyon (Faz 5+)

Bu maddeler aktif geliştirme planında değil, ama vizyondan silinmemiştir:

- **Component Knowledge Base** — Datasheet rules, manufacturer models, model registry
- **Lockfile sistemi** — `netlang.lock` ile reproducible builds
- **VS Code Extension** — Syntax highlighting, inline diagnostics, preview
- **PCB Export** — KiCad footprint mapping, BOM üretimi
- **Cloud simulation** — Büyük devreler için sunucu taraflı Ngspice
- **Subcircuit library** — Hazır devre blokları (voltage regulator, H-bridge, filter)
- **Multi-ground** — AGND, DGND, chassis ground desteği
- **Advanced layout** — Sugiyama/layered algorithm, hypergraph support
- **Trait SimulationBackend** — Farklı simulator backend'leri (LTSpice, Xyce)
- **Ticari model** — Private registry, team projects, CI dashboards

---

## Pipeline Akışı (Hedef Mimari — Faz 2 Sonrası)

```
NetLang Source (.nl)
      │
      ▼
┌─────────────┐
│   Parser    │  netlang.pest → pest → AST
│  (parser.rs) │
└──────┬──────┘
       │
       ▼
┌──────────────┐
│  AST → IR    │  ast_to_ir() — typed dönüşüm
│  (ir.rs)     │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────────────────┐
│                  Circuit IR (Typed)                    │
│  components: Vec<IRComponent>                         │
│  nets: Vec<NetDef>                                    │
│  analyses: Vec<Analysis>                              │
│  assertions: Vec<Assertion>                           │
└───┬──────────┬──────────────┬────────────┬───────────┘
    │          │              │            │
    ▼          ▼              ▼            ▼
┌───────┐ ┌────────┐   ┌──────────┐  ┌──────────┐
│  ERC  │ │ SPICE  │   │ Layout   │  │  JSON    │
│       │ │ Backend│   │ Engine   │  │  Output  │
└───────┘ └────────┘   └──────────┘  └──────────┘
    │          │              │            │
    ▼          ▼              ▼            ▼
  Diagnostics  .spice       SVG/KiCad    API Response
  (NL-EXXX)    netlist      schematic    (structured)
```

---

## Dosya Yapısı (Hedef — Faz 2 Sonrası)

```
NetLang/
├── .agents/
│   └── AGENTS.md               # AI ajan kuralları
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Modül export'ları
│       ├── netlang.pest        # PEG grammar
│       ├── parser.rs           # Source → AST
│       ├── ast.rs              # AST tipleri
│       ├── ir.rs               # ★ Circuit IR (typed) — YENİ
│       ├── graph.rs            # Net assignment + SPICE generation
│       ├── erc.rs              # ★ Electrical Rules Check (eski drc.rs)
│       ├── layout.rs           # Schematic layout engine
│       ├── kicad.rs            # KiCad export
│       ├── sim_result.rs       # ★ Ngspice result parser — YENİ (Faz 3)
│       ├── wasm.rs             # WASM bindings
│       └── bin/
│           └── netlang.rs      # ★ Birleşik CLI — YENİ
├── docs/
│   ├── architecture.md         # Mimari anayasa (güncel tutulacak)
│   ├── cli_reference.md        # ★ CLI Komutları ve JSON yapısı
│   └── ROADMAP.md              # ★ Bu dosya
├── examples/
│   ├── demo_circuit.nl
│   ├── test_amp.nl
│   ├── wheatstone.nl
│   └── test_nc.nl
└── webapp/
    ├── src/
    │   ├── App.tsx
    │   ├── App.css
    │   ├── index.css
    │   └── main.tsx
    └── ...
```

---

## Kurallar ve Kırmızı Çizgiler

Bu kurallar tüm fazlarda geçerlidir ve ihlal edilemez:

1. **Source, Battery Değil.** Voltaj kaynakları için `battery` kelimesi ASLA kullanılmaz, `source` kullanılır. (Parser'da backward-compat olarak kabul edilir ama AST'de `Source`'a dönüşür.)
2. **Düğüm Algoritması Deterministik.** `graph.rs`'deki N_{İlkPin} algoritması her zaman aynı graph için aynı sonucu üretir. User-named netler bu algoritmayı değiştirmez, üstüne eklenir.
3. **Orientation Kuralları.** `layout.rs`'de GND aşağı, VCC yukarı, input sola, output sağa yönelir. `is_signal_pin` ve `get_comp_def` pin koordinatları bu kurallara uyar.
4. **IR Tek Gerçek Kaynak.** Faz 1 sonrasında tüm backend'ler (SPICE, Layout, ERC, JSON) yalnızca Circuit IR üzerinden çalışır. AST'den doğrudan SPICE üretimi yapılmaz.
5. **Geriye Uyumluluk.** Yeni syntax eklenirken mevcut `.nl` dosyaları kırılmamalıdır. Mevcut örnekler her zaman çalışmalıdır.
6. **Test Kapısı.** Her faz tamamlandığında `cargo test` %100 geçmelidir. Başarısız test ile commit yapılmaz.

---

## Nasıl Kullanılır (AI Ajanları İçin)

1. **İlk adım:** Bu dosyayı oku.
2. **Mevcut fazı bul:** Checkbox'lara bak, `[ ]` olan ilk görev nerede?
3. **O fazın ön koşullarını kontrol et:** Önceki faz tamamen `[x]` mi?
4. **Görevi yap, testi yaz, checkbox'ı `[x]` yap.**
5. **Bu dosyayı güncelle** — tarih ve durum notu ekle.

> **⚠️ Faz atlama YASAKTIR.** Faz 2'ye geçmek için Faz 1'in tüm checkbox'ları `[x]` olmalıdır.
